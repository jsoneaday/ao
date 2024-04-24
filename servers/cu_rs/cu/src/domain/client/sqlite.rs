use sqlx::{Pool, Sqlite, SqlitePool};
use ao_common::domain::dal::Log;
use ao_common::domain::UnitLog;
use std::os::unix::fs::MetadataExt;
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

#[async_trait]
pub trait Repository {
    async fn init(url: &str, bootstrap: Option<bool>, wal_limit: Option<u64>) -> Self;
}

pub trait ConnGetter: Repository {
    type Output;

    fn get_conn(&self) -> &Self::Output;
}

pub struct SqliteClient {
    conn: Pool<Sqlite>,
    logger: Arc<dyn Log>
}

#[async_trait]
impl Repository for SqliteClient {
    async fn init(url: &str, bootstrap: Option<bool>, wal_limit: Option<u64>) -> Self {
        let conn = create_sqlite_client(url, bootstrap, wal_limit).await.unwrap();

        let client = SqliteClient {
            conn,
            logger: UnitLog::init()
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
        match sqlx::query::<_>(r"
            CREATE TABLE IF NOT EXISTS $1 (
                id TEXT PRIMARY KEY,
                signature TEXT,
                data TEXT,
                anchor TEXT,
                owner TEXT,
                tags JSONB,
                block JSONB
            ) WITHOUT ROWID;
        ")
        .bind(PROCESSES_TABLE)
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Processes Table {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Processes Table {:?}", e))
        }
    }

    pub async fn create_blocks(&self) {
        match sqlx::query::<_>(r"
            CREATE TABLE IF NOT EXISTS $1(
                id INTEGER PRIMARY KEY,
                height INTEGER,
                timestamp INTEGER
            ) WITHOUT ROWID;
        ")
        .bind(BLOCKS_TABLE)
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Blocks Table {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Blocks Table {:?}", e))
        }
    }

    pub async fn create_modules(&self) {
        match sqlx::query::<_>(r"
            CREATE TABLE IF NOT EXISTS $1(
                id TEXT PRIMARY KEY,
                owner TEXT,
                tags JSONB
            ) WITHOUT ROWID;
        ")
        .bind(MODULES_TABLE)
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Modules Table {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Modules Table {:?}", e))
        }
    }

    pub async fn create_evaluations(&self) {
        match sqlx::query::<_>(r"
            CREATE TABLE IF NOT EXISTS $1(
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
        ")
        .bind(EVALUATIONS_TABLE)
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Evaluations Table {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Evaluations Table {:?}", e))
        }
    }

    pub async fn create_messages(&self) {
        match sqlx::query::<_>(r"
            CREATE TABLE IF NOT EXISTS ${}(
                id TEXT,
                processId TEXT,
                seq TEXT,
                PRIMARY KEY (id, processId)
            ) WITHOUT ROWID;
        ")
        .bind(MESSAGES_TABLE)
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Messages Table {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Messages Table {:?}", e))
        }
    }

    pub async fn create_block_indexes(&self) {
        match sqlx::query::<_>(format!(r"
            CREATE INDEX IF NOT EXISTS idx_{}_height_timestamp
            ON $1
            (height, timestamp);
        ", BLOCKS_TABLE).as_str())
        .bind(BLOCKS_TABLE)
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Block Indexes {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Block Indexes {:?}", e))
        }
    }

    pub async fn create_message_indexes(&self) {
        match sqlx::query::<_>(format!(r"
            CREATE INDEX IF NOT EXISTS idx_{}_height_timestamp
            ON $1
            (height, timestamp);
        ", MESSAGES_TABLE).as_str())
        .bind(MESSAGES_TABLE)
        .execute(self.get_conn())
        .await {
            Ok(res) => self.logger.log(format!("Created Message Indexes {:?}", res)),
            Err(e) => self.logger.error(format!("Failed to Create Message Indexes {:?}", e))
        }
    }
}

async fn create_sqlite_client(url: &str, bootstrap: Option<bool>, wal_limit: Option<u64>) -> Result<Pool<Sqlite>, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(url)?
        .pragma("encoding", "UTF-8")
        .journal_mode(SqliteJournalMode::Wal);

    setup_wal_checkpoint(options.clone(), url, bootstrap, wal_limit).await;

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
            let result = metadata(format!("{}-wal", _url)).unwrap();
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