use sqlx::migrate::MigrateDatabase;
use sqlx::{FromRow, Pool, Sqlite, SqlitePool};
use ao_common::domain::dal::Log;
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use sqlx::Executor;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode};
use std::str::FromStr;
use std::fs::metadata;

pub const PROCESSES_TABLE: &str = "processes";
pub const BLOCKS_TABLE: &str = "blocks"; 
pub const MODULES_TABLE: &str = "modules"; 
pub const EVALUATIONS_TABLE: &str = "evaluations"; 
pub const MESSAGES_TABLE: &str = "messages";

#[derive(FromRow, Debug)]
pub struct SqliteMasterEntry {
  pub type_: String,
  pub name: String,
  pub tbl_name: Option<String>,
  pub rootpage: i64,
  pub sql: Option<String>,
}

#[allow(unused)]
#[derive(FromRow, Debug)]
pub struct PragmaFunctionList {
    name: String,
    type_: String
}

#[async_trait]
pub trait Repository {
    async fn init(url: &str, logger: Arc<dyn Log>, bootstrap: Option<bool>, wal_limit: Option<u64>) -> Self;
}

pub trait ConnGetter: Repository {
    type Output;

    fn get_conn(&self) -> &Self::Output;
}

pub struct SqliteClient {
    conn: Pool<Sqlite>,
    pub logger: Arc<dyn Log>
}

#[async_trait]
impl Repository for SqliteClient {
    async fn init(url: &str, logger: Arc<dyn Log>, bootstrap: Option<bool>, wal_limit: Option<u64>) -> Self {
        let conn = create_sqlite_client(url, bootstrap, wal_limit).await.unwrap();

        let client = SqliteClient {
            conn,
            logger
        };

        client.create_processes().await;
        client.create_blocks().await;
        client.create_modules().await;
        client.create_evaluations().await;
        client.create_messages().await;
        client.create_block_indexes().await;
        client.create_message_indexes().await;

        client
    }
}

impl ConnGetter for SqliteClient {
    type Output = Pool<Sqlite>;

    fn get_conn(&self) -> &Self::Output {
        &self.conn
    }
}

impl SqliteClient {
    pub async fn create_processes(&self) {
        let query = format!(r"
                CREATE TABLE IF NOT EXISTS {} (
                    id TEXT PRIMARY KEY,
                    signature TEXT,
                    data TEXT,
                    anchor TEXT,
                    owner TEXT,
                    tags JSONB,
                    block JSONB
                ) WITHOUT ROWID;
            ", PROCESSES_TABLE);
        match self.get_conn().execute(query.as_str()).await {
            Ok(res) => self.logger.log(format!("Created Processes Table {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Processes Table {:?}", e))
        }
    }

    pub async fn create_blocks(&self) {
        match sqlx::query::<_>(format!(r"
            CREATE TABLE IF NOT EXISTS {} (
                id INTEGER PRIMARY KEY,
                height INTEGER,
                timestamp INTEGER
            ) WITHOUT ROWID;
        ", BLOCKS_TABLE).as_str())
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Blocks Table {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Blocks Table {:?}", e))
        }
    }

    pub async fn create_modules(&self) {
        match sqlx::query::<_>(format!(r"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                owner TEXT,
                tags JSONB
            ) WITHOUT ROWID;
        ", MODULES_TABLE).as_str())
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Modules Table {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Modules Table {:?}", e))
        }
    }

    pub async fn create_evaluations(&self) {
        match sqlx::query::<_>(format!(r"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT PRIMARY KEY,
                processId TEXT,
                messageId TEXT,
                deepHash TEXT,
                nonce INTEGER,
                epoch INTEGER,
                timestamp INTEGER,
                ordinate TEXT,
                blockHeight INTEGER,
                cron TEXT,
                output JSONB,
                evaluatedAt INTEGER
            ) WITHOUT ROWID;
        ", EVALUATIONS_TABLE).as_str())
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Evaluations Table {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Evaluations Table {:?}", e))
        }
    }

    pub async fn create_messages(&self) {
        match sqlx::query::<_>(format!(r"
            CREATE TABLE IF NOT EXISTS {} (
                id TEXT,
                processId TEXT,
                seq TEXT,
                PRIMARY KEY (id, processId)
            ) WITHOUT ROWID;
        ", MESSAGES_TABLE).as_str())
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Messages Table {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Messages Table {:?}", e))
        }
    }

    pub async fn create_block_indexes(&self) {
        match sqlx::query::<_>(format!(r"
            CREATE INDEX IF NOT EXISTS idx_{}_height_timestamp
            ON {}
            (height, timestamp);
        ", BLOCKS_TABLE, BLOCKS_TABLE).as_str())        
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Block Indexes {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Block Indexes {:?}", e))
        }
    }

    pub async fn create_message_indexes(&self) {
        match sqlx::query::<_>(format!(r"
            CREATE INDEX IF NOT EXISTS idx_{}_id_processId_seq
            ON {}
            (id, processId, seq);
        ", MESSAGES_TABLE, MESSAGES_TABLE).as_str())
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Message Indexes {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Message Indexes {:?}", e))
        }
    }
}

async fn create_sqlite_client(url: &str, bootstrap: Option<bool>, wal_limit: Option<u64>) -> Result<Pool<Sqlite>, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(url)?
        .journal_mode(SqliteJournalMode::Wal);

    setup_wal_checkpoint(options.clone(), url, bootstrap, wal_limit).await;

    if !Sqlite::database_exists(url).await.unwrap_or_else(|_| false) {
        _ = Sqlite::create_database(url).await;
    }
    Ok(SqlitePool::connect_with(options).await.unwrap())
}

async fn setup_wal_checkpoint(options: SqliteConnectOptions, url: &str, bootstrap: Option<bool>, wal_limit: Option<u64>) {
    let _bootstrap = if let Some(bootstrap) = bootstrap {
        bootstrap
    } else {
        false
    };
    let _wal_limit = if let Some(wal_limit) = wal_limit {
        wal_limit
    } else {
        100
    };

    if _bootstrap {
        let _url = url.to_string();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            
            let conn = SqlitePool::connect_with(options).await.unwrap();
            _ = conn.execute("PRAGMA encoding = 'UTF-8';").await;

            let mut path = PathBuf::new();
            path.push(_url.replace("sqlite://", ""));
            let current_dir = std::env::current_dir().unwrap();
            path = current_dir.join(path);
            let result = metadata(format!("{}-wal", path.to_string_lossy())).unwrap();
            if result.size() > _wal_limit {
                let mut tx = conn.begin().await.unwrap();
                _ = tx.execute("PRAGMA wal_checkpoint(RESTART)").await;
                _ = tx.commit().await;
            }
            
            conn.close().await;
        });
    }
}

/**
 * Use a high value unicode character to terminate a range query prefix.
 * This will cause only string with a given prefix to match a range query
 */
pub const COLLATION_SEQUENCE_MAX_CHAR: &str = "\u{10FFFF}";

/**
 * This technically isn't the smallest char, but it's small enough for our needs
 */
pub const COLLATION_SEQUENCE_MIN_CHAR: &str = "0";