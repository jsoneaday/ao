use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqliteRow;
use sqlx::{Sqlite, FromRow, Row};
use crate::domain::dal::SaveEvaluationSchema;
use crate::domain::model::model::{EvaluationSchema, EvaluationSchemaExtended, Output};
use crate::domain::strings::{get_empty_string, get_number_string};
use crate::domain::utils::error::{CuErrors, HttpError, SchemaValidationError};
use super::sqlite::{SqliteClient, ConnGetter, EVALUATIONS_TABLE, MESSAGES_TABLE};
use validator::Validate;
use async_trait::async_trait;

#[allow(unused)]
pub struct Query {
    pub sql: String,
    pub parameters: Vec<String>
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationDocSchema {
    /// Comma delimited list: process_id, timestamp, ordinate, cron
    id: String,
    process_id: String,
    message_id: Option<String>,
    deep_hash: Option<String>,
    timestamp: i64,
    epoch: Option<i64>,
    nonce: Option<i64>,
    ordinate: i64,
    block_height: i64,
    cron: Option<String>,
    evaluated_at: DateTime<Utc>,
    output: Option<Output>
}

/// Note: each field should have min length 1
#[derive(Deserialize, Debug, Validate)]
#[serde(rename_all = "camelCase")]
pub struct MessageDocParamsSchema {
    /// Comma delimited list: process_id, timestamp, ordinate, cron
    #[validate(length(min = 1))]
    id: String,
    #[validate(length(min = 1))]
    process_id: String,
    #[validate(length(min = 1))]
    epoch_nonce: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct EvaluationQuerySchema {
    id: String,
    process_id: String,
    message_id: Option<String>,
    deep_hash: Option<String>,
    timestamp: i64,
    epoch: Option<i64>,
    nonce: Option<i64>,
    ordinate: i64,
    block_height: i64,
    cron: Option<String>,
    /// in milliseconds
    evaluated_at: i64,
    /// A json string
    output: String
}

impl<'r> FromRow<'r, SqliteRow> for EvaluationQuerySchema {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(
            EvaluationQuerySchema { 
                id: row.try_get("id")?, 
                process_id: row.try_get("processId")?,
                message_id: row.try_get("message_id")?,
                deep_hash: row.try_get("deepHash")?,
                timestamp: row.try_get("timestamp")?,
                epoch: row.try_get("epoch")?,
                nonce: row.try_get("nonce")?,
                ordinate: row.try_get("ordinate")?,
                block_height: row.try_get("blockHeight")?,
                cron: row.try_get("cron")?,
                evaluated_at: row.try_get("evaluatedAt")?,
                output: row.try_get("output")?
            }
        )
    }
}

pub struct AoEvaluation {
    sql_client: Arc<SqliteClient>
}

impl AoEvaluation {
    pub fn new(sql_client: Arc<SqliteClient>) -> Self {
        AoEvaluation {
            sql_client
        }
    }

    fn create_evaluation_id(process_id: &str, timestamp: i64, ordinate: i64, cron: Option<String>) -> String {
        if cron.is_none() {
            format!("{},{},{}", process_id, timestamp, ordinate)
        } else {
            format!("{},{},{},{}", process_id, timestamp, ordinate, cron.unwrap())
        }        
    }

    /**
     * Each message evaluated by the CU must have a unique idenfier. Messages can be:
     * - an "end-user" message (signed by a "end-user" wallet)
     * - an assignment (either signed by an "end-user" wallet or pushed from a MU)
     * - a pushed message (from a MU)
     *
     * If the message is an assignment, then we know that its unique identifier
     * is always the messageId.
     *
     * Otherwise, we must check if a deepHash was calculated by the CU (ie. for a pushed message)
     * and use that as the unique identifier
     *
     * Finally, if it is not an assignment and also not pushed from a MU, then it MUST
     * be a "end-user" message, and therefore its unique identifier is, once again, the messageId
     */
    fn create_message_id (message_id: Option<String>, deep_hash: Option<String>, is_assignment: bool) -> Result<String, CuErrors> {
        if is_assignment {
            if message_id.is_none() {  
                return Err(CuErrors::SchemaValidation(SchemaValidationError { message: "message_id must have a value if is_assignment is true!".to_string() }))
            }

            return Ok(message_id.unwrap());
        }
        if deep_hash.is_some() {
            return Ok(deep_hash.unwrap());
        } 
        Ok(message_id.unwrap())
    }

    fn create_select_query (process_id: &str, timestamp: i64, ordinate: i64, cron: Option<String>) -> Query {
        Query {
          sql: format!(r"
            SELECT
              id, processId, messageId, deepHash, nonce, epoch, timestamp,
              ordinate, blockHeight, cron, evaluatedAt, output
            FROM {}
            WHERE
              id = ?;
          ", EVALUATIONS_TABLE),
          parameters: vec![AoEvaluation::create_evaluation_id(process_id, timestamp, ordinate, cron)]
        }
    }

    fn from_evaluation_doc(evaluation_query_schema: &EvaluationQuerySchema) -> EvaluationSchema {
        EvaluationSchema {
            process_id: evaluation_query_schema.process_id.clone(),
            message_id: evaluation_query_schema.message_id.clone(),
            deep_hash: evaluation_query_schema.deep_hash.clone(),
            timestamp: evaluation_query_schema.timestamp,
            epoch: evaluation_query_schema.epoch,
            nonce: evaluation_query_schema.nonce,
            ordinate: evaluation_query_schema.ordinate.clone(),
            block_height: evaluation_query_schema.block_height,
            cron: evaluation_query_schema.cron.clone(),
            evaluated_at: DateTime::from_timestamp_millis(evaluation_query_schema.evaluated_at).unwrap(),
            output: serde_json::from_str(&evaluation_query_schema.output).unwrap()
        }
    }

    pub async fn find_evaluation(&self, process_id: &str, timestamp: i64, ordinate: i64, cron: Option<String>) -> Result<EvaluationSchema, CuErrors> {
        let query = AoEvaluation::create_select_query(process_id, timestamp, ordinate, cron);
        let mut raw_query = sqlx::query_as::<Sqlite, EvaluationQuerySchema>(&query.sql);
        for param in query.parameters.iter() {
            let _raw_query = raw_query.bind(param);
            raw_query = _raw_query;
        }
        match raw_query.fetch_all(self.sql_client.get_conn()).await {
            Ok(res) => {
                match res.first() {
                    Some(row) => Ok(AoEvaluation::from_evaluation_doc(row)),
                    None => Err(CuErrors::HttpStatus(HttpError { status: 404, message: "Evaluation result not found".to_string() }))
                }
            },
            Err(e) => Err(CuErrors::SchemaValidation(SchemaValidationError { message: e.to_string() }))
        }
    }

    fn get_evaluation_id(evaluation: &EvaluationSchemaExtended) -> String {
        AoEvaluation::create_evaluation_id(&evaluation.process_id, evaluation.timestamp, evaluation.ordinate, evaluation.cron.clone())
    }

    fn to_evaluation_doc(evaluation: &EvaluationSchemaExtended) -> EvaluationDocSchema {
        EvaluationDocSchema { 
            id: AoEvaluation::get_evaluation_id(&evaluation), 
            process_id: evaluation.process_id.clone(), 
            message_id: evaluation.message_id.clone(), 
            deep_hash: evaluation.deep_hash.clone(), 
            timestamp: evaluation.timestamp, 
            epoch: evaluation.epoch, 
            nonce: evaluation.nonce, 
            ordinate: evaluation.ordinate, 
            block_height: evaluation.block_height, 
            cron: evaluation.cron.clone(), 
            evaluated_at: evaluation.evaluated_at, 
            output: evaluation.output.clone()
        }
    }

    fn create_insert_queries(evaluation: EvaluationSchemaExtended) -> Result<Vec<Query>, CuErrors> {
        let eval_doc = AoEvaluation::to_evaluation_doc(&evaluation);
        let mut statements = vec![
            Query {
                sql: format!(r"
                    INSERT OR IGNORE INTO {}
                    (id, processId, messageId, deepHash, nonce, epoch, timestamp, ordinate, blockHeight, cron, evaluatedAt, output)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?);
                ", EVALUATIONS_TABLE),
                parameters: vec![
                    eval_doc.id,
                    eval_doc.process_id,
                    get_empty_string(eval_doc.message_id),
                    get_empty_string(eval_doc.deep_hash),
                    if eval_doc.nonce.is_some() { eval_doc.nonce.unwrap().to_string() } else { "".to_string() },
                    if eval_doc.epoch.is_some() { eval_doc.epoch.unwrap().to_string() } else { "".to_string() },
                    eval_doc.timestamp.to_string(),
                    eval_doc.ordinate.to_string(),
                    eval_doc.block_height.to_string(),
                    get_empty_string(eval_doc.cron),
                    eval_doc.evaluated_at.timestamp_millis().to_string(),
                    serde_json::to_string(&eval_doc.output).unwrap()
                ]
            }
        ];

        if evaluation.cron.is_none() {
            let id_result = AoEvaluation::create_message_id(evaluation.message_id, evaluation.deep_hash, evaluation.is_assignment);
            if let Err(e) = id_result {
                return Err(e)
            }
            let message_doc_params_schema = MessageDocParamsSchema {
                id: id_result.unwrap(),
                process_id: evaluation.process_id,
                epoch_nonce: format!("{}:{}", get_number_string(evaluation.epoch), get_number_string(evaluation.nonce))
            };
            match message_doc_params_schema.validate() {
                Ok(_) => {
                    statements.push(
                        Query {
                            sql: format!(r"INSERT OR IGNORE INTO {} (id, processId, seq) VALUES (?, ?, ?);", MESSAGES_TABLE),
                            parameters: vec![
                                message_doc_params_schema.id,
                                message_doc_params_schema.process_id,
                                message_doc_params_schema.epoch_nonce
                            ]
                        }
                    );
                },
                Err(e) => {
                    return Err(CuErrors::SchemaValidation(SchemaValidationError { message: e.to_string() }));
                }
            }            
        }

        Ok(statements)
    }    
}

#[async_trait]
impl SaveEvaluationSchema for AoEvaluation {
    async fn save_evaluation(&self, evaluation: EvaluationSchemaExtended) -> Result<(), CuErrors> {
        match AoEvaluation::create_insert_queries(evaluation) {
            Ok(statements) => {
                let mut tx = self.sql_client.get_conn().begin().await.unwrap();
        
                for statement in statements.iter() {
                    let mut query = sqlx::query::<Sqlite>(&statement.sql);
                    for param in statement.parameters.iter() {
                        query = query.bind(param);
                    }
                    match query.fetch_all(&mut *tx).await {
                        Ok(res) => {
                            println!("res 0{:?}", res[0].columns());
                            println!("res 1{:?}", res[1].columns());
                            return Ok(())
                        },
                        Err(e) => {
                            _ = tx.rollback().await;
                            return Err(CuErrors::DatabaseError(e));
                        }
                    }
                }
        
                match tx.commit().await {
                    Ok(()) => Ok(()),
                    Err(e) => Err(CuErrors::DatabaseError(e))
                }
            },
            Err(e) => Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod ao_evaluation {
        use super::*;

        mod save_evaluation {
            use super::*;

            mod save_the_evaluation_and_message {
                use super::*;
                use lazy_static::lazy_static;

                lazy_static! {
                    static ref EVALUATED_AT: Arc<DateTime<Utc>> = {
                        Arc::new(Utc::now())
                    };
                }

                struct MockUseDeepHashAsId;
                #[async_trait]
                impl SaveEvaluationSchema for MockUseDeepHashAsId {
                    async fn save_evaluation(&self, evaluation: EvaluationSchemaExtended) -> Result<(), CuErrors> {
                        match AoEvaluation::create_insert_queries(evaluation) {
                            Ok(statements) => {
                                let first_query = &statements[0];
                                assert!(first_query.parameters[0].as_str() == "process-123,1702677252111,1");
                                assert!(first_query.parameters[1].as_str() == "process-123");
                                assert!(first_query.parameters[2].as_str() == "message-123");
                                assert!(first_query.parameters[3].as_str() == "deepHash-123");
                                assert!(first_query.parameters[4].as_str() == "1");
                                assert!(first_query.parameters[5].as_str() == "0");
                                assert!(first_query.parameters[6].as_str() == "1702677252111");
                                assert!(first_query.parameters[7].as_str() == "1");
                                assert!(first_query.parameters[8].as_str() == "1234");
                                assert!(first_query.parameters[9].as_str() == "");
                                assert!(first_query.parameters[10] == EVALUATED_AT.clone().timestamp_millis().to_string());
                                // todo: skipping output as not sure how to test something that effectively is of type any
        
                                let second_query = &statements[1];
                                assert!(second_query.parameters[0].as_str() == "deepHash-123");
                                assert!(second_query.parameters[1].as_str() == "process-123");
                                assert!(second_query.parameters[2].as_str() == "0:1");
                                
                                Ok(())
                            },
                            Err(e) => panic!("{}", e)
                        }                        
                    }
                }

                #[tokio::test]
                async fn test_use_deep_hash_as_the_message_doc_id() {
                    let args = EvaluationSchemaExtended {
                        is_assignment: false,
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
                        evaluated_at: *EVALUATED_AT.clone()
                    };

                    let mock = MockUseDeepHashAsId;
                    _ = mock.save_evaluation(args).await;
                }

                struct MockUseMessageIdAsId;
                #[async_trait]
                impl SaveEvaluationSchema for MockUseMessageIdAsId {
                    async fn save_evaluation(&self, evaluation: EvaluationSchemaExtended) -> Result<(), CuErrors> {
                        match AoEvaluation::create_insert_queries(evaluation) {
                            Ok(statements) => {
                                let first_query = &statements[0];
                                assert!(first_query.parameters[0].as_str() == "process-123,1702677252111,1");
                                assert!(first_query.parameters[1].as_str() == "process-123");
                                assert!(first_query.parameters[2].as_str() == "message-123");
                                assert!(first_query.parameters[3].as_str() == "deepHash-123");
                                assert!(first_query.parameters[4].as_str() == "1");
                                assert!(first_query.parameters[5].as_str() == "0");
                                assert!(first_query.parameters[6].as_str() == "1702677252111");
                                assert!(first_query.parameters[7].as_str() == "1");
                                assert!(first_query.parameters[8].as_str() == "1234");
                                assert!(first_query.parameters[9].as_str() == "");
                                assert!(first_query.parameters[10] == EVALUATED_AT.clone().timestamp_millis().to_string());
                                // todo: skipping output as not sure how to test something that effectively is of type any
        
                                let second_query = &statements[1];
                                assert!(second_query.parameters[0].as_str() == "message-123");
                                assert!(second_query.parameters[1].as_str() == "process-123");
                                assert!(second_query.parameters[2].as_str() == "0:1");
                                
                                Ok(())
                            },
                            Err(e) => panic!("{}", e)
                        }                        
                    }
                }

                #[tokio::test]
                async fn test_use_message_id_as_the_message_doc_id_if_assignment() {
                    let args = EvaluationSchemaExtended {
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
                        evaluated_at: *EVALUATED_AT.clone()
                    };

                    let mock = MockUseMessageIdAsId;
                    _ = mock.save_evaluation(args).await;
                }

                struct MockUseMessageIdIfNoDeepHash;
                #[async_trait]
                impl SaveEvaluationSchema for MockUseMessageIdIfNoDeepHash {
                    async fn save_evaluation(&self, evaluation: EvaluationSchemaExtended) -> Result<(), CuErrors> {
                        match AoEvaluation::create_insert_queries(evaluation) {
                            Ok(statements) => {
                                let first_query = &statements[0];
                                assert!(first_query.parameters[0].as_str() == "process-123,1702677252111,1");
                                assert!(first_query.parameters[1].as_str() == "process-123");
                                assert!(first_query.parameters[2].as_str() == "message-123");
                                assert!(first_query.parameters[3].as_str() == "");
                                assert!(first_query.parameters[4].as_str() == "1");
                                assert!(first_query.parameters[5].as_str() == "0");
                                assert!(first_query.parameters[6].as_str() == "1702677252111");
                                assert!(first_query.parameters[7].as_str() == "1");
                                assert!(first_query.parameters[8].as_str() == "1234");
                                assert!(first_query.parameters[9].as_str() == "");
                                assert!(first_query.parameters[10] == EVALUATED_AT.clone().timestamp_millis().to_string());
                                // todo: skipping output as not sure how to test something that effectively is of type any
        
                                let second_query = &statements[1];
                                assert!(second_query.parameters[0].as_str() == "message-123");
                                assert!(second_query.parameters[1].as_str() == "process-123");
                                assert!(second_query.parameters[2].as_str() == "0:1");
                                
                                Ok(())
                            },
                            Err(e) => panic!("{}", e)
                        }                        
                    }
                }

                #[tokio::test]
                async fn test_use_message_id_as_the_message_doc_id_if_no_deep_hash() {
                    let args = EvaluationSchemaExtended {
                        is_assignment: false,
                        deep_hash: None,
                        timestamp: 1702677252111,
                        nonce: Some(1),
                        epoch: Some(0),
                        ordinate: 1,
                        block_height: 1234,
                        cron: None,
                        process_id: "process-123".to_string(),
                        message_id: Some("message-123".to_string()),
                        output: None,
                        evaluated_at: *EVALUATED_AT.clone()
                    };

                    let mock = MockUseMessageIdIfNoDeepHash;
                    _ = mock.save_evaluation(args).await;
                }

                struct MockNoopInsertEvaluationOrMessage;
                #[async_trait]
                impl SaveEvaluationSchema for MockNoopInsertEvaluationOrMessage {
                    async fn save_evaluation(&self, evaluation: EvaluationSchemaExtended) -> Result<(), CuErrors> {
                        match AoEvaluation::create_insert_queries(evaluation) {
                            Ok(statements) => {
                                let first_query = &statements[0];
                                assert!(first_query.sql.trim().starts_with(format!("INSERT OR IGNORE INTO {}", EVALUATIONS_TABLE).as_str()));
        
                                let second_query = &statements[1];
                                assert!(second_query.sql.trim().starts_with(format!("INSERT OR IGNORE INTO {}", MESSAGES_TABLE).as_str()));
                                
                                Ok(())
                            },
                            Err(e) => panic!("{}", e)
                        }                        
                    }
                }

                #[tokio::test]
                async fn test_noop_insert_evaluation_or_message_if_it_already_exists() {
                    let args = EvaluationSchemaExtended {
                        is_assignment: false,
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
                        evaluated_at: *EVALUATED_AT.clone()
                    };

                    let mock = MockNoopInsertEvaluationOrMessage;
                    _ = mock.save_evaluation(args).await;
                }
            }
        }
    }
}