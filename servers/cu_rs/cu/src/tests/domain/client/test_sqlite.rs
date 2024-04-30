#[allow(unused)]
use crate::domain::client::sqlite::{ConnGetter, SqliteMasterEntry, BLOCKS_TABLE, EVALUATIONS_TABLE, MESSAGES_TABLE, MODULES_TABLE, PROCESSES_TABLE};
#[allow(unused)]
use crate::domain::client::sqlite::{Repository, SqliteClient};
#[allow(unused)]
use crate::tests::fixtures::log::get_logger;

#[allow(unused)]
fn setup_url() -> String {
    use std::env;
    use dotenv::dotenv;
    
    dotenv().ok();
    env::var("DB_URL").unwrap()
}

#[allow(unused)]
pub fn delete_db_files(db_file: &str) {
    _ = std::fs::remove_file(db_file);
    _ = std::fs::remove_file(format!("{}-shm", db_file));
    _ = std::fs::remove_file(format!("{}-wal", db_file));
}

// Give each test its own database

#[tokio::test]
async fn test_sqlclient_init() {      
    let db_file = "cu1.db";
    let db_url = format!("sqlite://{}", db_file);

    let client = SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await;

    client.get_conn().close().await;
    delete_db_files(db_file);
}

#[tokio::test]
async fn test_base_tables_created() {    
    let db_file = "cu2.db";
    let db_url = format!("sqlite://{}", db_file);

    let client = SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await;
    let query = "SELECT type AS type_, name, tbl_name, rootpage, sql FROM sqlite_master WHERE type='table' AND name=?;";

    let result = sqlx::query_as::<_, SqliteMasterEntry>(query)
        .bind(PROCESSES_TABLE)
        .fetch_optional(client.get_conn())
        .await.unwrap();
    match result {
        Some(row) => assert!(row.name == PROCESSES_TABLE.to_string()),
        None => panic!("table {} not found", PROCESSES_TABLE)
    };
    let result = sqlx::query_as::<_, SqliteMasterEntry>(query)
        .bind(BLOCKS_TABLE)
        .fetch_optional(client.get_conn())
        .await.unwrap();
    match result {
        Some(row) => assert!(row.name == BLOCKS_TABLE.to_string()),
        None => panic!("table {} not found", BLOCKS_TABLE)
    };
    let result = sqlx::query_as::<_, SqliteMasterEntry>(query)
        .bind(MODULES_TABLE)
        .fetch_optional(client.get_conn())
        .await.unwrap();
    match result {
        Some(row) => assert!(row.name == MODULES_TABLE.to_string()),
        None => panic!("table {} not found", MODULES_TABLE)
    };
    let result = sqlx::query_as::<_, SqliteMasterEntry>(query)
    .bind(EVALUATIONS_TABLE)
    .fetch_optional(client.get_conn())
    .await.unwrap();
    match result {
        Some(row) => assert!(row.name == EVALUATIONS_TABLE.to_string()),
        None => panic!("table {} not found", EVALUATIONS_TABLE)
    };
    let result = sqlx::query_as::<_, SqliteMasterEntry>(query)
    .bind(MESSAGES_TABLE)
    .fetch_optional(client.get_conn())
    .await.unwrap();
    match result {
        Some(row) => assert!(row.name == MESSAGES_TABLE.to_string()),
        None => panic!("table {} not found", MESSAGES_TABLE)
    };

    client.get_conn().close().await;
    delete_db_files(db_file);
}
