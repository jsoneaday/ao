use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};
use moka::{future::Cache, Expiry};
use tokio::sync::RwLock;
use crate::domain::{model::model::{EvaluationSchema, RawTagSchema}, schema_utils::{is_earlier_than, is_equal_to, is_later_then, PartialEvaluationSchema}};
use futures::{future::{AbortHandle, Abortable}, Future};

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
struct AoProcess {
    process_memory_cache: Cache<String, CacheEntry>,
    /// the expiration time of a cache entry
    ttl: u64,
    drain_to_file_threshold: u64,
    drain_to_file_timers: Arc<RwLock<HashMap<String, AbortHandle>>>,
    clear_drain_to_file_timer: Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + 'static>> + 'static>,
    write_process_memory_file: Box<dyn Fn(CacheEntry) -> Vec<u8> + std::marker::Sync + std::marker::Send + 'static>
}

impl AoProcess {
    pub fn create_process_memory_cache<Fa, Fb>(max_size: u64, ttl: u64, drain_to_file_threshold: u64, on_eviction: Fa, write_process_memory_file: Fb) -> Self 
        where 
            Fa: Fn(String, CacheEntry) + std::marker::Sync + std::marker::Send + 'static,
            Fb: Fn(CacheEntry) -> Vec<u8> + std::marker::Sync + std::marker::Send + 'static {
        let drain_to_file_timers = Arc::new(RwLock::new(HashMap::new()));
        let ao_process = AoProcess { 
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
    fn clear_timer(map: Arc<RwLock<HashMap<String, AbortHandle>>>) -> Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + 'static>> + 'static> {
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
}

pub struct AoProcessCacheExpiry;
impl Expiry<String, CacheEntry> for AoProcessCacheExpiry {
    fn expire_after_create(&self, _key: &String, value: &CacheEntry, _created_at: std::time::Instant) -> Option<Duration> {
        let duration = value.expiration.as_duration();
        duration
    }
}