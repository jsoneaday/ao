use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};
use async_trait::async_trait;
use moka::{future::Cache, Expiry};
use sqlx::{prelude::{FromRow, Row}, sqlite::SqliteRow, Sqlite};
use tokio::sync::RwLock;
use validator::Validate;
use crate::domain::{
    dal::{FindProcessSchema, SaveProcessSchema}, 
    model::model::{EvaluationSchema, ProcessSchema, RawTagSchema}, 
    schema_utils::{is_earlier_than, is_equal_to, is_later_then, PartialEvaluationSchema}, 
    utils::error::{CuErrors, HttpError, SchemaValidationError}
};
use futures::{future::{AbortHandle, Abortable}, Future};
use super::sqlite::{SqliteClient, PROCESSES_TABLE, ConnGetter};

pub struct SelectQuery {
    pub sql: String,
    pub parameters: Vec<String>
}

pub struct InsertQuery {
    pub sql: String,
    pub parameters: (
        String,
        String,
        String, // Option<Vec<u8>>,
        String,
        String,
        String,
        String
    )
}

/**
 * Used to indicate we are interested in the latest cached
 * memory for the given process
 */
pub const LATEST: &str = "LATEST";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Expiration {
    /// The value never expires.
    Never,
    OneSecond,
    FiveSeconds,
    TenSeconds,
}

impl Expiration {
    pub fn as_duration(&self) -> Option<Duration> {
        match self {
            Expiration::Never => None,
            Expiration::OneSecond => Some(Duration::from_secs(1)),
            Expiration::FiveSeconds => Some(Duration::from_secs(5)),
            Expiration::TenSeconds => Some(Duration::from_secs(10))
        }
    }

    pub fn get_expiration_from_ms(ttl: u64) -> Self {
        let _ttl = ttl / 1000;
        match _ttl {
            0 => Expiration::Never,
            1 => Expiration::OneSecond,
            2..=5 => Expiration::FiveSeconds,
            6..=10 => Expiration::TenSeconds,
            _ => panic!("Unexpected ttl value for cache expiration") // todo: need to fix this to return error
        }
    }
}

pub struct ProcessesToLoad {
    /// number of items in cache
    size: u64,
    calculated_size: u64,
    processes: Vec<Process>
}

pub struct Process {
    process: String,
    size: u64
}

#[derive(Clone)]
pub struct CacheEntry {
    evaluation: EvaluationSchema,
    memory: Option<Vec<u8>>,
    /// As name says represents a file, using this as File is not cloneable
    file: Option<Vec<u8>>,
    expiration: Expiration
}

enum Destination {
    IsString(String),
    IsEvaluation(EvaluationSchema)
}

#[derive(Debug)]
#[allow(unused)]
pub struct ProcessQuerySchema {
    pub id: String,
    pub signature: Option<String>,
    pub data: String,
    pub anchor: Option<String>,
    pub owner: String,
    /// json string
    pub tags: String,
    /// json string
    pub block: String
}

impl<'r> FromRow<'r, SqliteRow> for ProcessQuerySchema {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(
            ProcessQuerySchema { 
                id: row.try_get("id")?, 
                signature: row.try_get("signature")?,
                data: row.try_get("data")?,
                anchor: row.try_get("anchor")?,
                owner: row.try_get("owner")?,
                tags: row.try_get("tags")?,
                block: row.try_get("block")?
            }
        )
    }
}

pub struct AoProcessCacheExpiry;
impl Expiry<String, CacheEntry> for AoProcessCacheExpiry {
    fn expire_after_create(&self, _key: &String, value: &CacheEntry, _created_at: std::time::Instant) -> Option<Duration> {
        let duration = value.expiration.as_duration();
        duration
    }
}

/**
 * @type {{
 *  get: LRUCache<string, { evaluation: Evaluation, File?: string, Memory?: ArrayBuffer }>['get']
 *  set: LRUCache<string, { evaluation: Evaluation, File?: string, Memory?: ArrayBuffer }>['set']
 *  lru: LRUCache<string, { evaluation: Evaluation, File?: string, Memory?: ArrayBuffer }>
 * }}
 *
 * @typedef Evaluation
 * @prop {string} processId
 * @prop {string} moduleId
 * @prop {string} epoch
 * @prop {string} nonce
 * @prop {string} timestamp
 * @prop {number} blockHeight
 * @prop {string} ordinate
 * @prop {string} encoding
 * @prop {string} [cron]
 */
pub struct AoProcess {
    sql_client: Arc<SqliteClient>,
    process_memory_cache: Cache<String, CacheEntry>,
    /// the expiration time of a cache entry
    ttl: u64,
    drain_to_file_threshold: u64,
    drain_to_file_timers: Arc<RwLock<HashMap<String, AbortHandle>>>,
    clear_drain_to_file_timer: Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + 'static>> + std::marker::Sync + std::marker::Send + 'static>,
    write_process_memory_file: Box<dyn Fn(CacheEntry) -> Vec<u8> + std::marker::Sync + std::marker::Send + 'static>
}

impl AoProcess {
    pub fn create_process_memory_cache<Fa, Fb>(sql_client: Arc<SqliteClient>, max_size: u64, ttl: u64, drain_to_file_threshold: u64, on_eviction: Fa, write_process_memory_file: Fb) -> Self 
        where 
            Fa: Fn(String, CacheEntry) + std::marker::Sync + std::marker::Send + 'static,
            Fb: Fn(CacheEntry) -> Vec<u8> + std::marker::Sync + std::marker::Send + 'static {
        let drain_to_file_timers = Arc::new(RwLock::new(HashMap::new()));
        let ao_process = AoProcess { 
            sql_client,
            process_memory_cache: Cache::builder()
                .weigher(|_key, value: &CacheEntry| -> u32 {
                    if value.memory.is_some() { return value.memory.clone().unwrap().len() as u32; }

                    value.file.clone().unwrap().len() as u32
                })
                .max_capacity(max_size)
                .eviction_listener(move |key: Arc<String>, value, _cause| {
                    on_eviction(key.clone().as_ref().to_string(), value);
                })
                .expire_after(AoProcessCacheExpiry)
                .build(),
            ttl,
            drain_to_file_threshold,
            drain_to_file_timers: drain_to_file_timers.clone(),
            clear_drain_to_file_timer: AoProcess::clear_timer(drain_to_file_timers),
            write_process_memory_file: Box::new(write_process_memory_file)
        };
        
        ao_process
    }

    fn pluck_tag_value(name: String, tags: Vec<RawTagSchema>) -> Option<String> {
        let tag = tags.iter().find(|t| t.name == name);
        if let Some(tag) = tag {
            Some(tag.value.clone())
        } else {
            None
        }
    }
    
    /// resets timer duration
    fn clear_timer(map: Arc<RwLock<HashMap<String, AbortHandle>>>) -> Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + 'static>> + std::marker::Sync + std::marker::Send + 'static> {
        Box::new(move |key: String| {
            let cloned_map = map.clone();
            Box::pin(async move {          
                let mut _map = cloned_map.write().await;
                if _map.contains_key(&key) {
                    _map.get(&key).unwrap().abort();
                    _map.remove(&key);            
                }
            })
        })
    }

    pub async fn get(&self, key: &str) -> Option<CacheEntry> {
        if !self.process_memory_cache.contains_key(key) {
            return None;
        }

        // /**
        // * Will subsequently renew the age
        // * and recency of the cached value
        // */
        let value = self.process_memory_cache.get(key).await;
        self.process_memory_cache.insert(key.to_string(), value.clone().unwrap()).await;

        Some(value.unwrap())
    }

    pub async fn set(&mut self, key: &str, value: CacheEntry) {
        // /**
        // * Set up timer to drain Process memory to a file, if not accessed
        // * within the DRAIN_TO_FILE period ie. 30 seconds
        // *
        // * This keeps the cache entry in the LRU In-Memory cache, but removes
        // * the process Memory from the heap, clearing up space
        // *
        // * On subsequent read, client may need to read the memory back in from
        // * a file
        // */
        if value.memory.is_some() && self.drain_to_file_threshold > 0 {
            self.clear_drain_to_file_timer.as_mut()(key.to_string()).await;

            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            let drain_to_file_abortable = Abortable::new(async {
                tokio::time::sleep(Duration::from_millis(self.drain_to_file_threshold)).await;

                let file = self.write_process_memory_file.as_ref()(value.clone());
                // /**
                //  * Update the cache entry with the file reference containing the memory
                //  * and remove the reference to the Memory, so that it can be GC'd.
                //  *
                //  * Since we are setting on the underlying data store directly,
                //  * this won't reset the ttl
                //  *
                //  * Note we do not mutate the old object, and instead cache a new one,
                //  * in case the old object containing the memory is in use elsewhere
                //  */
                self.process_memory_cache.insert(key.to_string(), CacheEntry {
                    evaluation: value.clone().evaluation,
                    file: Some(file),
                    memory: None,
                    expiration: Expiration::get_expiration_from_ms(self.ttl)
                }).await;
                self.drain_to_file_timers.clone().write().await.remove(key);
            }, abort_registration);
            _ = drain_to_file_abortable.await;

            self.drain_to_file_timers.clone().write().await.insert(key.to_string(), abort_handle);
        }

        self.process_memory_cache.insert(key.to_string(), value).await;
    }

    pub fn load_process_cache_usage(&self) -> ProcessesToLoad {
        ProcessesToLoad {
            size: self.process_memory_cache.entry_count(),
            calculated_size: self.process_memory_cache.weighted_size(),
            processes: self.process_memory_cache.iter().map(|(k, v)| {
                Process {
                    process: k.to_string(),
                    size: if v.memory.is_some() {
                        v.memory.unwrap().len() as u64
                    } else {
                        v.file.unwrap().len() as u64
                    }
                }
            }).collect::<Vec<Process>>()
        }
    }

    /// allow_owners list of process owners allowed to run (whitelist)
    /// an empty allow_owners list means all allowed
    pub fn is_process_owner_supported(allow_owners: Vec<String>, id: String) -> bool {
        if allow_owners.len() == 0 || allow_owners.iter().find(|owner| **owner == id).is_some() {
            return true;
        }
        return false;
    }

    fn latest_checkpoint_before(destination: &Destination, cur_latest: &Option<EvaluationSchema>, checkpoint: &EvaluationSchema) -> EvaluationSchema {
        // /**
        // * Often times, we are just interested in the latest checkpoint --
        // * the latest point we can start evaluating from, up to the present.
        // *
        // * So we have a special case where instead of passing criteria
        // * such as { timestamp, ordinate, cron } to serve as the right-most limit,
        // * the caller can simply pass the string 'latest'.
        // *
        // * Our destination is the latest, so we should
        // * just find the latest checkpoint
        // */
        if let Destination::IsString(destination) = destination {
            if destination == LATEST {
                if cur_latest.is_none() {
                    return checkpoint.clone();
                }
                if is_earlier_than(PartialEvaluationSchema {
                        timestamp: cur_latest.clone().unwrap().timestamp,
                        cron: cur_latest.clone().unwrap().cron
                    }, PartialEvaluationSchema {
                        timestamp: checkpoint.timestamp,
                        cron: checkpoint.cron.clone()
                    }) {
                    return cur_latest.clone().unwrap();
                } else {
                    return checkpoint.clone();
                }
            }
        }

        if let Destination::IsEvaluation(destination) = destination {
            // /**
            //  * We need to use our destination as the right-most (upper) limit
            //  * for our comparisons.
            //  *
            //  * In other words, we're only interested in checkpoints before
            //  * our destination
            //  */
            let EvaluationSchema { timestamp, cron, .. } = destination;
            if is_later_then(PartialEvaluationSchema { timestamp: timestamp.clone(), cron: cron.clone() }, PartialEvaluationSchema { timestamp: checkpoint.timestamp, cron: checkpoint.cron.clone() })
                || is_equal_to(PartialEvaluationSchema { timestamp: timestamp.clone(), cron: cron.clone() }, PartialEvaluationSchema { timestamp: checkpoint.timestamp, cron: checkpoint.cron.clone() })
                || (cur_latest.is_some() && is_earlier_than(PartialEvaluationSchema {
                    timestamp: cur_latest.clone().unwrap().timestamp,
                    cron: cur_latest.clone().unwrap().cron
                }, PartialEvaluationSchema { timestamp: checkpoint.timestamp, cron: checkpoint.cron.clone() })) {
                    return cur_latest.clone().unwrap();
            }            
        }
        checkpoint.clone()
    } 

    fn create_find_process_query(process_id: &str) -> SelectQuery {
        SelectQuery {
            sql: format!(r"
                SELECT id, signature, data, anchor, owner, tags, block
                FROM {}
                WHERE
                id = ?;
            ", PROCESSES_TABLE),
            parameters: vec![process_id.to_string()]
        }
    }

    fn create_save_process_query(process_schema: &ProcessSchema) -> InsertQuery {
        InsertQuery {
            sql: format!(r"
                INSERT OR IGNORE INTO {}
                (id, signature, data, anchor, owner, tags, block)
                VALUES (?, ?, ?, ?, ?, ?, ?);
            ", PROCESSES_TABLE),
            parameters: (
                process_schema.id.clone(),
                if let Some(signature) = process_schema.signature.clone() {
                    signature
                } else {
                    "".to_string()
                },
                process_schema.data.clone(),
                if let Some(anchor) = process_schema.anchor.clone() {
                    anchor
                } else {
                    "".to_string()
                },
                process_schema.owner.clone(),
                serde_json::to_string(&process_schema.tags).unwrap(),
                serde_json::to_string(&process_schema.block).unwrap()
            )
        }
    }
}

#[async_trait]
impl FindProcessSchema for AoProcess {
    async fn find_process(&self, process_id: &str) -> Result<ProcessSchema, CuErrors> {
        let query = AoProcess::create_find_process_query(process_id);
        let mut raw_query = sqlx::query_as::<_, ProcessQuerySchema>(&query.sql);
        for param in query.parameters.iter() {
            raw_query = raw_query.bind(param);
        }

        match raw_query.fetch_optional(self.sql_client.clone().get_conn()).await {
            Ok(res) => {
                println!("find_process res: {:?}", res);
                match res {
                    Some(res) => {
                        let process_schema = ProcessSchema {
                            id: res.id,
                            signature: res.signature,
                            data: res.data,
                            anchor: res.anchor,
                            owner: res.owner,
                            tags: serde_json::from_str(&res.tags).unwrap(),
                            block: serde_json::from_str(&res.block).unwrap()
                        };
                        match process_schema.validate() {
                            Ok(_) => Ok(process_schema),
                            Err(_) => Err(CuErrors::SchemaValidation(SchemaValidationError { message: "ProcessSchema validation failed".to_string()}))
                        }                        
                    },
                    None => Err(CuErrors::HttpStatus(HttpError { status: 404, message: "Process not found".to_string() }))
                }
            },
            Err(e) => Err(CuErrors::DatabaseError(e))
        }
    }
}

#[async_trait]
impl SaveProcessSchema for AoProcess {
    async fn save_process(&self, process_schema: ProcessSchema) -> Result<(), CuErrors> {
        let query = AoProcess::create_save_process_query(&process_schema);
        let mut raw_query = sqlx::query::<Sqlite>(&query.sql);
        let (
            id,
            signature,
            data,
            anchor,
            owner,
            tags,
            block
        ) = query.parameters;
        raw_query = raw_query
            .bind(id)
            .bind(signature)
            .bind(data)
            .bind(anchor)
            .bind(owner)
            .bind(tags)
            .bind(block);

        match raw_query.execute(self.sql_client.clone().get_conn()).await {
            Ok(res) => {
                println!("save_process res: {:?}", res);
                Ok(())
            },
            Err(e) => Err(CuErrors::DatabaseError(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod ao_process {
        use chrono::Utc;
        use once_cell::sync::Lazy;
        use super::*;

        static NOW: Lazy<i64> = Lazy::new(|| {
            Utc::now().timestamp_millis()
        });

        mod find_process {
            use crate::domain::model::model::BlockSchema;
            use super::*;

            mod find_the_process {
                use super::*;

                struct MockFindProcess;
                #[async_trait]
                impl FindProcessSchema for MockFindProcess {
                    async fn find_process(&self, _process_id: &str) -> Result<ProcessSchema, CuErrors> {
                        Ok(                            
                            ProcessSchema {
                                id: "process-123".to_string(),
                                owner: "woohoo".to_string(),
                                tags: vec![
                                    RawTagSchema { name: "foo".to_string(), value: "bar".to_string() }
                                ],
                                signature: Some("sig-123".to_string()),
                                anchor: None,
                                data: "data-123".to_string(),
                                block: BlockSchema {
                                    height: 123,
                                    timestamp: *NOW
                                }
                            }                            
                        )
                    }
                }

                #[tokio::test]
                async fn test_find_the_process() {
                    let mock = MockFindProcess;
                    match mock.find_process("process-123").await {
                        Ok(res) => {
                            assert!(res.id == "process-123");
                            assert!(res.owner == "woohoo");
                            assert!(res.tags[0].name == "foo");
                            assert!(res.tags[0].value == "bar");
                            assert!(res.signature == Some("sig-123".to_string()));
                            assert!(res.anchor == None);
                            assert!(res.data == "data-123".to_string());
                            assert!(res.block.height == 123);
                            assert!(res.block.timestamp == *NOW);
                        },
                        Err(e) => panic!("{}", e)
                    }
                }
            }

            mod return_404_if_not_found {
                use super::*;

                struct MockReturn404IfNotFound;
                #[async_trait]
                impl FindProcessSchema for MockReturn404IfNotFound {
                    async fn find_process(&self, _process_id: &str) -> Result<ProcessSchema, CuErrors> {
                        Err(CuErrors::HttpStatus(HttpError { status: 404, message: "failed".to_string() }))
                    }
                }

                #[tokio::test]
                async fn test_return_404_if_not_found() {
                    let mock = MockReturn404IfNotFound;
                    match mock.find_process("process-123").await {
                        Ok(_) => panic!("Should not return value"),
                        Err(e) => if let CuErrors::HttpStatus(e) = e {
                            assert!(e.status == 404);
                        } else {
                            panic!("Wrong error provided")
                        }
                    }
                }
            }

            mod bubble_process {
                use super::*;

                struct MockBubbleProcess;
                #[async_trait]
                impl FindProcessSchema for MockBubbleProcess {
                    async fn find_process(&self, _process_id: &str) -> Result<ProcessSchema, CuErrors> {
                        Err(CuErrors::HttpStatus(HttpError { status: 500, message: "failed".to_string() }))
                    }
                }

                #[tokio::test]
                async fn test_bubble_process() {
                    let mock = MockBubbleProcess;
                    match mock.find_process("process-123").await {
                        Ok(_) => panic!("Should not return value"),
                        Err(e) => if let CuErrors::HttpStatus(e) = e {
                            assert!(e.status == 500);
                        } else {
                            panic!("Wrong error provided")
                        }
                    }
                }
            }
        }

        mod save_process {
            use super::*;

            mod save_the_process {
                use crate::domain::model::model::BlockSchema;

                use super::*;

                struct MockSaveTheProcess;
                #[async_trait]
                impl SaveProcessSchema for MockSaveTheProcess {
                    async fn save_process(&self, process_schema: ProcessSchema) -> Result<(), CuErrors> {
                        let query = AoProcess::create_save_process_query(&process_schema);

                        let (
                            id,
                            signature,
                            data,
                            anchor,
                            owner,
                            tags,
                            block
                        ) = query.parameters;
                        assert!(id == "process-123");
                        assert!(signature == "sig-123");
                        assert!(data == "data-123");
                        assert!(anchor == "");
                        assert!(owner == "woohoo");
                        let _tags: Vec<RawTagSchema> = serde_json::from_str(&tags).unwrap();
                        assert!(_tags[0].name == "foo");
                        assert!(_tags[0].value == "bar");
                        let _block: BlockSchema = serde_json::from_str(&block).unwrap();
                        assert!(_block.height == 123);
                        assert!(_block.timestamp == *NOW);

                        Ok(())
                    }
                }

                #[tokio::test]
                async fn test_save_the_process() {
                    let mock = MockSaveTheProcess;
                    match mock.save_process(
                        ProcessSchema {
                            id: "process-123".to_string(),
                            owner: "woohoo".to_string(),
                            signature: Some("sig-123".to_string()),
                            anchor: None,
                            data: "data-123".to_string(),
                            tags: vec![RawTagSchema { name: "foo".to_string(), value: "bar".to_string() }],
                            block: BlockSchema {
                              height: 123,
                              timestamp: *NOW
                            }
                        }
                    ).await {
                        Ok(_) => (),
                        Err(e) => panic!("{}", e)
                    }
                }
            }

            mod noop_if_the_process_already_exists {
                use crate::domain::model::model::BlockSchema;

                use super::*;

                struct NoopIfTheProcessExists;
                #[async_trait]
                impl SaveProcessSchema for NoopIfTheProcessExists {
                    async fn save_process(&self, process_schema: ProcessSchema) -> Result<(), CuErrors> {
                        let query = AoProcess::create_save_process_query(&process_schema);

                        assert!(query.sql.trim().starts_with("INSERT OR IGNORE"));

                        Ok(())
                    }
                }

                #[tokio::test]
                async fn test_noop_if_the_process_already_exists() {
                    let mock = NoopIfTheProcessExists;
                    match mock.save_process(
                        ProcessSchema {
                            id: "process-123".to_string(),
                            owner: "woohoo".to_string(),
                            signature: Some("sig-123".to_string()),
                            anchor: None,
                            data: "data-123".to_string(),
                            tags: vec![RawTagSchema { name: "foo".to_string(), value: "bar".to_string() }],
                            block: BlockSchema {
                              height: 123,
                              timestamp: *NOW
                            }
                        }
                    ).await {
                        Ok(_) => (),
                        Err(e) => panic!("{}", e)
                    }
                }
            }
        }
    }
}