#[allow(unused)]
use std::sync::Arc;
#[allow(unused)]
use crate::domain::dal::FindModuleSchema;
#[allow(unused)]
use crate::tests::domain::client::test_sqlite::delete_db_files;
#[allow(unused)]
use crate::{
    domain::{
        client::{ao_module::AoModule, sqlite::{Repository, SqliteClient, ConnGetter}}, 
        dal::SaveModuleSchema, 
        model::model::{ModuleSchema, Owner, RawTagSchema},
    }, 
    tests::fixtures::log::get_logger
};

#[tokio::test]
async fn test_save_module() {
    let db_file = "aomodule1.db";
    let db_url = format!("sqlite://{}", db_file);
    let sql_client_arc = Arc::new(SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await);

    let ao = AoModule::new(sql_client_arc.clone());
    let mut err_msg = "".to_string();
    match ao.save_module(ModuleSchema {
        id: "mod-123".to_string(),
        owner: "owner-123".to_string(),
        tags: vec![
            RawTagSchema {
                name: "Module-Format".to_string(),
                value: "wasm32-unknown-emscripten".to_string()
            }
        ]
    }).await {
        Ok(_) => (),
        Err(e) => err_msg = format!("{}", e)
    };

    sql_client_arc.clone().get_conn().close().await;
    delete_db_files(db_file);

    if !err_msg.is_empty() {
        panic!("{}", err_msg);
    }
}

#[tokio::test]
async fn test_find_module() {
    let db_file = "aomodule2.db";
    let db_url = format!("sqlite://{}", db_file);
    let sql_client_arc = Arc::new(SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await);

    let ao = AoModule::new(sql_client_arc.clone());
    let mut err_msg = "".to_string();
    match ao.save_module(ModuleSchema {
        id: "mod-123".to_string(),
        owner: "owner-123".to_string(),
        tags: vec![
            RawTagSchema {
                name: "Module-Format".to_string(),
                value: "wasm32-unknown-emscripten".to_string()
            }
        ]
    }).await {
        Ok(module_id) => {
            match ao.find_module(module_id.clone()).await {
                Ok(res) => {
                    println!("res {:?}", res.clone().unwrap());   
                    assert!(res.unwrap().id == module_id)
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