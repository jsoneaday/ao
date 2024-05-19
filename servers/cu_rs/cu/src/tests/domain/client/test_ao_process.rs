#[allow(unused)]
use std::sync::Arc;
#[allow(unused)]
use crate::config::get_server_config_schema;
#[allow(unused)]
use crate::domain::client::ao_process::AoProcess;
#[allow(unused)]
use crate::domain::dal::SaveProcessSchema;
#[allow(unused)]
use crate::tests::fixtures::log::get_logger;
use crate::{domain::client::sqlite::{ConnGetter, Repository, SqliteClient}, tests::domain::client::test_sqlite::delete_db_files};

#[tokio::test]
async fn test_find_process() {
    use super::*;

    // let db_file = "aoprocess1.db";
    // let db_url = format!("sqlite://{}", db_file);
    // let sql_client_arc = Arc::new(SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await);
    // let config = get_server_config_schema(true).unwrap();

    // let ao = AoProcess::create_process_memory_cache(
    //     sql_client_arc.clone(), 
    //     config.PROCESS_MEMORY_CACHE_MAX_SIZE, 
    //     config.PROCESS_MEMORY_CACHE_TTL,
    //     config.PROCESS_MEMORY_CACHE_DRAIN_TO_FILE_THRESHOLD,
    // );
    // let mut err_msg = "".to_string();
    // match ao.save_process(ModuleSchema {
    //     id: "mod-123".to_string(),
    //     owner: "owner-123".to_string(),
    //     tags: vec![
    //         RawTagSchema {
    //             name: "Module-Format".to_string(),
    //             value: "wasm32-unknown-emscripten".to_string()
    //         }
    //     ]
    // }).await {
    //     Ok(module_id) => {
    //         match ao.find_module(&module_id).await {
    //             Ok(res) => {
    //                 println!("res {:?}", res.clone().unwrap());   
    //                 assert!(res.unwrap().id == module_id)
    //             },
    //             Err(e) => panic!("{}", e)
    //         }
    //     },
    //     Err(e) => err_msg = format!("{}", e)
    // };

    // sql_client_arc.clone().get_conn().close().await;
    // delete_db_files(db_file);

    // if !err_msg.is_empty() {
    //     panic!("{}", err_msg);
    // }
}