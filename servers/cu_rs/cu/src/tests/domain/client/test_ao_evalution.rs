#[allow(unused)]
use std::sync::Arc;
#[allow(unused)]
use chrono::Utc;
#[allow(unused)]
use crate::domain::client::ao_evaluation::EvaluationQuerySchema;
#[allow(unused)]
use crate::domain::model::model::{FromOrToEvaluationSchema, Sort};
#[allow(unused)]
use crate::{
    config::get_server_config_schema, 
    domain::{
        client::{ao_evaluation::AoEvaluation, sqlite::{Repository, ConnGetter, SqliteClient}}, 
        dal::{FindEvaluationSchema, FindEvaluationsSchema, SaveEvaluationSchema}, 
        model::model::EvaluationSchemaExtended
    }, 
    tests::fixtures::log::get_logger,
    tests::domain::client::test_sqlite::delete_db_files
};
#[allow(unused)]
use serde_json::json;

#[tokio::test]
async fn test_save_evaluation() {
    let db_file = "aoevaluation1.db";
    let db_url = format!("sqlite://{}", db_file);
    let sql_client_arc = Arc::new(SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await);

    let evaluater = AoEvaluation::new(sql_client_arc.clone());
    let mut err_msg = "".to_string();
    match evaluater.save_evaluation(EvaluationSchemaExtended {
        is_assignment: true,
        deep_hash: Some("deepHash-123".to_string()),
        timestamp: 1702677252111,
        nonce: Some(1),
        epoch: Some(0),
        ordinate: "1".to_string(),
        block_height: 1234,
        cron: None,
        process_id: "process-111".to_string(),
        message_id: Some("message-123".to_string()),
        output: json!({ "Messages": [{ "foo": "bar" }], "Memory": "foo" }),
        evaluated_at: Utc::now()
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
async fn test_find_evaluation() {
    let db_file = "aoevaluation2.db";
    let db_url = format!("sqlite://{}", db_file);
    let sql_client_arc = Arc::new(SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await);
    let process_id: &str = "process-222";
    let timestamp = 1702677252111;

    let evaluater = AoEvaluation::new(sql_client_arc.clone());

    let mut err_msg = "".to_string();
    match evaluater.save_evaluation(EvaluationSchemaExtended {
        is_assignment: true,
        deep_hash: Some("deepHash-123".to_string()),
        timestamp,
        nonce: Some(1),
        epoch: Some(0),
        ordinate: "1".to_string(),
        block_height: 1234,
        cron: None,
        process_id: process_id.to_string(),
        message_id: Some("message-123".to_string()),
        output: json!({ "Messages": [{ "foo": "bar" }], "Memory": "foo" }),
        evaluated_at: Utc::now()
    }).await {
        Ok(_) => {
            match evaluater.find_evaluation(process_id, timestamp, "1", None).await {
                Ok(res) => match res {
                    Some(_) => (),
                    None => err_msg = "evaluation not found".to_string()
                },
                Err(e) => err_msg = format!("{}", e)
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

#[tokio::test]
async fn test_find_evaluations() {
    let db_file = "aoevaluation3.db";
    let db_url = format!("sqlite://{}", db_file);
    let sql_client_arc = Arc::new(SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await);
    let process_id: &str = "process-222";
    let timestamp = 1702677252111;

    let evaluater = AoEvaluation::new(sql_client_arc.clone());

    let mut err_msg = "".to_string();
    match evaluater.save_evaluation(EvaluationSchemaExtended {
        is_assignment: true,
        deep_hash: Some("deepHash-123".to_string()),
        timestamp,
        nonce: Some(1),
        epoch: Some(0),
        ordinate: "1".to_string(),
        block_height: 1234,
        cron: None,
        process_id: process_id.to_string(),
        message_id: Some("message-123".to_string()),
        output: json!({ "Messages": [{ "foo": "bar" }], "Memory": "foo" }),
        evaluated_at: Utc::now()
    }).await {
        Ok(_) => {
            match evaluater.find_evaluations(
                process_id.to_string(), 
                Some(FromOrToEvaluationSchema { timestamp: Some(timestamp), ordinate: Some("1".to_string()), cron: None }), 
                Some(FromOrToEvaluationSchema { timestamp: Some(timestamp), ordinate: None, cron: None }), 
                Some(Sort::Asc), 
                10, 
                None
            ).await {
                Ok(res) => assert!(res.len() == 1),
                Err(e) => err_msg = format!("{}", e)
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