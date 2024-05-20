#[allow(unused)]
use std::sync::Arc;
#[allow(unused)]
use chrono::Utc;
#[allow(unused)]
use crate::config::get_server_config_schema;
#[allow(unused)]
use crate::domain::client::ao_process::AoProcess;
#[allow(unused)]
use crate::domain::dal::{FindProcessSchema, SaveProcessSchema};
#[allow(unused)]
use crate::tests::fixtures::log::get_logger;
#[allow(unused)]
use crate::{domain::{client::sqlite::{ConnGetter, Repository, SqliteClient}, model::model::{BlockSchema, ProcessSchema, RawTagSchema}}, tests::domain::client::test_sqlite::delete_db_files};

#[tokio::test]
async fn test_find_process() {
    let db_file = "aoprocess1.db";
    let db_url = format!("sqlite://{}", db_file);
    let sql_client_arc = Arc::new(SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await);
    let config = get_server_config_schema(true).as_ref().unwrap();

    let ao = AoProcess::create_process_memory_cache(
        sql_client_arc.clone(), 
        config.PROCESS_MEMORY_CACHE_MAX_SIZE as u64, 
        config.PROCESS_MEMORY_CACHE_TTL as u64,
        config.PROCESS_MEMORY_CACHE_DRAIN_TO_FILE_THRESHOLD as u64,
        |_key, _value| {},
        |_value| -> Vec<u8> { vec![] }
    );
    let mut err_msg = "".to_string();
    let process_id = "process-123".to_string();
    match ao.save_process(ProcessSchema {
        id: process_id.clone(),
        owner: "woohoo".to_string(),
        signature: Some("sig-123".to_string()),
        anchor: None,
        data: "data-123".to_string(),
        tags: vec![RawTagSchema { name: "foo".to_string(), value: "bar".to_string() }],
        block: BlockSchema {
          height: 123,
          timestamp: Utc::now().timestamp_millis()
        }
    }).await {
        Ok(_) => {
            match ao.find_process(&process_id).await {
                Ok(res) => {
                    println!("res {:?}", res.clone());   
                    assert!(res.id == process_id)
                },
                Err(e) => panic!("{}", e)
            }
        },
        Err(e) => err_msg = format!("{}", e)
    };

    sql_client_arc.clone().get_conn().close().await;
    delete_db_files(db_file);

    if !err_msg.is_empty() {
        panic!("{}", err_msg);
    }
}