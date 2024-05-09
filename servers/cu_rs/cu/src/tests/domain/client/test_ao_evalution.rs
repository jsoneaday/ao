#[allow(unused)]
use std::sync::Arc;
#[allow(unused)]
use chrono::Utc;
#[allow(unused)]
use crate::{
    config::get_server_config_schema, 
    domain::{client::{ao_evaluation::AoEvaluation, sqlite::{Repository, SqliteClient}}, dal::SaveEvaluationSchema, model::model::EvaluationSchemaExtended}, 
    tests::fixtures::log::get_logger
};

#[tokio::test]
async fn test_save_evaluation() {
    let db_file = "aoblockunit1.db";
    let db_url = format!("sqlite://{}", db_file);

    let evaluater = AoEvaluation::new(Arc::new(SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await));
    match evaluater.save_evaluation(EvaluationSchemaExtended {
        is_assignment: true,
        deep_hash: Some("deepHash-123".to_string()),
        timestamp: 1702677252111,
        nonce: Some(1),
        epoch: Some(0),
        ordinate: 1,
        block_height: 1234,
        cron: None,
        process_id: "process-123".to_string(),
        message_id: Some("message-123".to_string()),
        output: None,
        evaluated_at: Utc::now()
    }).await {
        Ok(_) => (),
        Err(e) => panic!("{}", e)
    }
}