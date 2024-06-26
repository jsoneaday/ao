use std::sync::Arc;
use bevy_reflect::Tuple;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::sqlite::SqliteRow;
use sqlx::{FromRow, Row, Sqlite};
use crate::domain::dal::{FindEvaluationSchema, FindEvaluationsSchema, FindMessageBeforeSchema, SaveEvaluationSchema};
use crate::domain::model::model::{EntityId, EvaluationSchema, EvaluationSchemaExtended, FromOrToEvaluationSchema, MessageBeforeSchema, Sort};
use crate::domain::strings::{get_empty_string, get_number_string};
use crate::domain::utils::error::{CuErrors, HttpError, SchemaValidationError};
use super::sqlite::{ConnGetter, SqliteClient, COLLATION_SEQUENCE_MAX_CHAR_AS_I64, EVALUATIONS_TABLE, MESSAGES_TABLE};
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
    ordinate: String,
    block_height: i64,
    cron: Option<String>,
    evaluated_at: DateTime<Utc>,
    output: Value
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationQuerySchema {
    pub id: String,
    pub process_id: String,
    pub message_id: Option<String>,
    pub deep_hash: Option<String>,
    pub timestamp: i64,
    pub epoch: Option<i64>,
    pub nonce: Option<i64>,
    pub ordinate: String,
    pub block_height: i64,
    pub cron: Option<String>,
    /// in milliseconds
    pub evaluated_at: i64,
    /// A json string
    pub output: String
}

impl<'r> FromRow<'r, SqliteRow> for EvaluationQuerySchema {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(
            EvaluationQuerySchema { 
                id: row.try_get("id")?, 
                process_id: row.try_get("processId")?,
                message_id: row.try_get("messageId")?,
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

    fn create_evaluation_id(process_id: String, timestamp: Option<i64>, ordinate: Option<String>, cron: Option<String>) -> String {
        let params = (process_id, timestamp, ordinate, cron);
        let mut final_string = "".to_string();
        for i in 0..params.field_len() {
            match i {
                0 => final_string.push_str(&format!("{},", params.0.clone())),
                1 => if let Some(timestamp) = params.1 {
                    final_string.push_str(&format!("{},", timestamp))
                },
                2 => if let Some(ref ordinate) = params.2 {
                    final_string.push_str(&format!("{},", ordinate.clone()))
                },
                3 => if let Some(ref cron) = params.3 {
                    final_string.push_str(&format!("{}", cron.clone()))
                },
                _ => ()
            }
        }
        final_string
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

    fn create_find_evaluation_query(process_id: &str, timestamp: i64, ordinate: &str, cron: Option<String>) -> Query {
        Query {
          sql: format!(r"
            SELECT
              id, processId, messageId, deepHash, nonce, epoch, timestamp,
              ordinate, blockHeight, cron, evaluatedAt, output
            FROM {}
            WHERE
              id = ?;
          ", EVALUATIONS_TABLE),
          parameters: vec![AoEvaluation::create_evaluation_id(process_id.to_string(), Some(timestamp), Some(ordinate.to_string()), cron)]
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

    fn to_evaluation_doc(evaluation: &EvaluationSchemaExtended) -> EvaluationDocSchema {
        EvaluationDocSchema { 
            id: AoEvaluation::create_evaluation_id(evaluation.process_id.clone(), Some(evaluation.timestamp), Some(evaluation.ordinate.clone()), evaluation.cron.clone()), 
            process_id: evaluation.process_id.clone(), 
            message_id: evaluation.message_id.clone(), 
            deep_hash: evaluation.deep_hash.clone(), 
            timestamp: evaluation.timestamp, 
            epoch: evaluation.epoch, 
            nonce: evaluation.nonce, 
            ordinate: evaluation.ordinate.clone(), 
            block_height: evaluation.block_height, 
            cron: evaluation.cron.clone(), 
            evaluated_at: evaluation.evaluated_at, 
            output: evaluation.output.clone()
        }
    }

    fn create_save_evaluation_queries(evaluation: EvaluationSchemaExtended) -> Result<Vec<Query>, CuErrors> {
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

    fn create_find_evaluations_query(process_id: &str, from: Option<FromOrToEvaluationSchema>, to: Option<FromOrToEvaluationSchema>, only_cron: bool, sort: Sort, limit: i64) -> Query {
        Query {
          sql: format!(r"
            SELECT
              id, processId, messageId, deepHash, nonce, epoch, timestamp,
              ordinate, blockHeight, cron, evaluatedAt, output
            FROM {}
            WHERE
              id >= ? AND id <= ?
              {}
            ORDER BY
              timestamp {},
              ordinate {},
              cron {}
            LIMIT ?;
          ", EVALUATIONS_TABLE, if only_cron { "AND cron IS NOT NULL" } else { "" }, sort, sort, sort),
          parameters: vec![
            // /**
            //  * trim range using criteria, if provided.
            //  *
            //  * from is exclusive, while to is inclusive
            //  */
            if from.is_none() {
                AoEvaluation::create_evaluation_id(process_id.to_string(), None, None, None)
            } else {
                AoEvaluation::create_evaluation_id(
                    process_id.to_string(), 
                    from.clone().unwrap().timestamp, 
                    from.clone().unwrap().ordinate,  
                    from.unwrap().cron
                )
            },
            if to.is_none() {
                AoEvaluation::create_evaluation_id(process_id.to_string(), Some(COLLATION_SEQUENCE_MAX_CHAR_AS_I64), None, None)
            } else {
                AoEvaluation::create_evaluation_id(
                    process_id.to_string(), 
                    to.clone().unwrap().timestamp, 
                    if let Some(ordinate) = to.clone().unwrap().ordinate { Some(ordinate) } else { Some(COLLATION_SEQUENCE_MAX_CHAR_AS_I64.to_string()) },
                    to.unwrap().cron
                )
            },
            limit.to_string()
          ]
        }
    }

    fn create_find_message_before_query (message_id: String, deep_hash: Option<String>, is_assignment: bool, process_id: String, nonce: i64, epoch: i64) -> Query {
        Query {
          sql: format!(r"
            SELECT
              id, seq
            FROM {}
            WHERE
              id = ?
              AND processId = ?
              AND seq < ?
            LIMIT 1;
          ", MESSAGES_TABLE),
          parameters: vec![
            AoEvaluation::create_message_id(Some(message_id), deep_hash, is_assignment).unwrap(),
            process_id,
            format!("{}:{}", epoch, nonce) // 0:13
          ]
        }
      }
}

#[async_trait]
impl SaveEvaluationSchema for AoEvaluation {
    async fn save_evaluation(&self, evaluation: EvaluationSchemaExtended) -> Result<(), CuErrors> {
        match AoEvaluation::create_save_evaluation_queries(evaluation.clone()) {
            Ok(statements) => {
                let mut tx = self.sql_client.get_conn().begin().await.unwrap();
                
                for statement in statements.iter() {
                    let mut query = sqlx::query::<Sqlite>(&statement.sql);
                    for param in statement.parameters.iter() {
                        query = query.bind(param);
                    }
                    let result = query.execute(&mut *tx).await;
                    match result {
                        Ok(_) => (),
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

#[async_trait]
impl FindEvaluationSchema for AoEvaluation {
    async fn find_evaluation(&self, process_id: &str, timestamp: i64, ordinate: &str, cron: Option<String>) -> Result<Option<EvaluationSchema>, CuErrors> {
        let query = AoEvaluation::create_find_evaluation_query(process_id, timestamp, ordinate, cron);
        let mut raw_query = sqlx::query_as::<Sqlite, EvaluationQuerySchema>(&query.sql);
        for param in query.parameters.iter() {
            raw_query = raw_query.bind(param);
        }
        match raw_query.fetch_all(self.sql_client.get_conn()).await {
            Ok(res) => {
                match res.first() {
                    Some(row) => Ok(Some(AoEvaluation::from_evaluation_doc(row))),
                    None => Err(CuErrors::HttpStatus(HttpError { status: 404, message: "Evaluation result not found".to_string() }))
                }
            },
            Err(e) => Err(CuErrors::DatabaseError(e))
        }
    }
}

#[async_trait]
impl FindEvaluationsSchema for AoEvaluation {
    async fn find_evaluations(
        &self,
        process_id: String, 
        from: Option<FromOrToEvaluationSchema>, 
        to: Option<FromOrToEvaluationSchema>, 
        sort: Option<Sort>, 
        limit: i64, 
        only_cron: Option<bool>
    ) -> Result<Vec<EvaluationSchema>, CuErrors> {
        let query = AoEvaluation::create_find_evaluations_query(
            process_id.as_str(), 
            from, 
            to, 
            if let Some(only_cron) = only_cron { only_cron } else { false }, 
            if let Some(sort) = sort { sort } else { Sort::Asc }, 
            limit
        );
        let mut raw_query = sqlx::query_as::<_, EvaluationQuerySchema>(&query.sql);
        for param in query.parameters.iter() {
            raw_query = raw_query.bind(param);
        }
        match raw_query.fetch_all(self.sql_client.get_conn()).await {
            Ok(res) => {
                let mut result: Vec<EvaluationSchema> = vec![];
                for eval in res.iter() {
                    result.push(AoEvaluation::from_evaluation_doc(eval));
                }
                Ok(result)
            },
            Err(e) => Err(CuErrors::DatabaseError(e))
        }
    }
}

#[async_trait]
impl FindMessageBeforeSchema for AoEvaluation {
    async fn find_message_before(
        &self,
        message_id: String,
        deep_hash: Option<String>,
        is_assignment: bool,
        process_id: String,
        epoch: i64,
        nonce: i64
    ) -> Result<EntityId, CuErrors> {
        let query = AoEvaluation::create_find_message_before_query(message_id, deep_hash, is_assignment, process_id, nonce, epoch);

        let mut raw_query = sqlx::query_as::<Sqlite, MessageBeforeSchema>(&query.sql);
        for param in query.parameters.iter() {
            raw_query = raw_query.bind(param);
        }
        match raw_query.fetch_optional(self.sql_client.get_conn()).await {
            Ok(res) => {
                match res {
                    Some(row) => Ok(EntityId { id: row.id }),
                    None => Err(CuErrors::HttpStatus(HttpError { status: 404, message: "Message not found".to_string() }))
                }
            },
            Err(e) => Err(CuErrors::DatabaseError(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod ao_evaluation {
        use super::*;

        mod find_evaluation {
            use super::*;
            use lazy_static::lazy_static;

            mod find_the_evaluation {                
                use super::*;

                lazy_static! {
                    static ref EVALUATED_AT: Arc<DateTime<Utc>> = {
                        Arc::new(Utc::now())
                    };
                }

                struct MockReturnListOfAllCronEvaluations;
                #[async_trait]
                impl FindEvaluationSchema for MockReturnListOfAllCronEvaluations {
                    async fn find_evaluation(&self, process_id: &str, timestamp: i64, ordinate: &str, cron: Option<String>) -> Result<Option<EvaluationSchema>, CuErrors> {
                        let query = AoEvaluation::create_find_evaluation_query(process_id, timestamp, ordinate, cron);

                        assert!(query.parameters[0] == "process-123,1702677252111,1,");

                        Ok(Some(EvaluationSchema {
                            process_id: "process-123".to_string(),
                            message_id: Some("message-123".to_string()),
                            deep_hash: Some("deepHash-123".to_string()),
                            nonce: Some(1),
                            epoch: Some(0),
                            timestamp: 1702677252111,
                            ordinate: "1".to_string(),
                            block_height: 1234,
                            cron: None,
                            evaluated_at: EVALUATED_AT.as_ref().clone(),
                            output: json!({ "Messages": [{ "foo": "bar" }] })
                        }))
                    }
                }

                #[tokio::test]
                async fn test_find_the_evaluation() {
                    let mock = MockReturnListOfAllCronEvaluations;
                    match mock.find_evaluation("process-123", 1702677252111, "1", None).await {
                        Ok(res) => {
                            match res {
                                Some(res) => {
                                    assert!(res.process_id == "process-123".to_string());
                                    assert!(res.message_id == Some("message-123".to_string()));
                                    assert!(res.deep_hash == Some("deepHash-123".to_string()));
                                    assert!(res.nonce == Some(1));
                                    assert!(res.epoch == Some(0));
                                    assert!(res.timestamp == 1702677252111);
                                    assert!(res.ordinate == "1".to_string());
                                    assert!(res.block_height == 1234);
                                    assert!(res.cron == None);
                                    assert!(res.evaluated_at == EVALUATED_AT.as_ref().clone());
                                    assert!(res.output == json!({ "Messages": [{ "foo": "bar" }] }));
                                },
                                None => panic!("evaluation not returned")
                            }
                        },
                        Err(e) => panic!("{}", e)
                    }
                }

                struct MockReturn404StatusIfNotFound;
                #[async_trait]
                impl FindEvaluationSchema for MockReturn404StatusIfNotFound {
                    async fn find_evaluation(&self, process_id: &str, timestamp: i64, ordinate: &str, cron: Option<String>) -> Result<Option<EvaluationSchema>, CuErrors> {
                        _ = AoEvaluation::create_find_evaluation_query(process_id, timestamp, ordinate, cron);

                        Ok(None)
                    }
                }

                #[tokio::test]
                async fn test_return_404_status_if_not_found() {
                    let mock = MockReturn404StatusIfNotFound;
                    match mock.find_evaluation("process-123", 1702677252111, "1", None).await {
                        Ok(res) => match res {
                            Some(_) => panic!("find_evaluation should not succeed"),
                            None => ()
                        },
                        Err(e) => match e {
                            CuErrors::HttpStatus(HttpError { status, message: _ }) => assert!(status == 404),
                            _ => panic!("find_evaluation invalid error")
                        }
                    }
                }
            }
        }

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
                        match AoEvaluation::create_save_evaluation_queries(evaluation) {
                            Ok(statements) => {
                                let first_query = &statements[0];
                                assert!(first_query.parameters[0].as_str() == "process-123,1702677252111,1,");
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
                                assert!(first_query.parameters[11] == serde_json::to_string(&json!({ "Messages": [{ "foo": "bar" }] })).unwrap());
        
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
                        ordinate: "1".to_string(),
                        block_height: 1234,
                        cron: None,
                        process_id: "process-123".to_string(),
                        message_id: Some("message-123".to_string()),
                        output: json!({ "Messages": [{ "foo": "bar" }] }),
                        evaluated_at: *EVALUATED_AT.clone()
                    };

                    let mock = MockUseDeepHashAsId;
                    _ = mock.save_evaluation(args).await;
                }

                struct MockUseMessageIdAsId;
                #[async_trait]
                impl SaveEvaluationSchema for MockUseMessageIdAsId {
                    async fn save_evaluation(&self, evaluation: EvaluationSchemaExtended) -> Result<(), CuErrors> {
                        match AoEvaluation::create_save_evaluation_queries(evaluation) {
                            Ok(statements) => {
                                let first_query = &statements[0];
                                assert!(first_query.parameters[0].as_str() == "process-123,1702677252111,1,");
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
                                assert!(first_query.parameters[11] == serde_json::to_string(&json!({ "Messages": [{ "foo": "bar" }] })).unwrap());
        
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
                        ordinate: "1".to_string(),
                        block_height: 1234,
                        cron: None,
                        process_id: "process-123".to_string(),
                        message_id: Some("message-123".to_string()),
                        output: json!({ "Messages": [{ "foo": "bar" }] }),
                        evaluated_at: *EVALUATED_AT.clone()
                    };

                    let mock = MockUseMessageIdAsId;
                    _ = mock.save_evaluation(args).await;
                }

                struct MockUseMessageIdIfNoDeepHash;
                #[async_trait]
                impl SaveEvaluationSchema for MockUseMessageIdIfNoDeepHash {
                    async fn save_evaluation(&self, evaluation: EvaluationSchemaExtended) -> Result<(), CuErrors> {
                        match AoEvaluation::create_save_evaluation_queries(evaluation) {
                            Ok(statements) => {
                                let first_query = &statements[0];
                                assert!(first_query.parameters[0].as_str() == "process-123,1702677252111,1,");
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
                                assert!(first_query.parameters[11] == serde_json::to_string(&json!({ "Messages": [{ "foo": "bar" }] })).unwrap());
        
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
                        ordinate: "1".to_string(),
                        block_height: 1234,
                        cron: None,
                        process_id: "process-123".to_string(),
                        message_id: Some("message-123".to_string()),
                        output: json!({ "Messages": [{ "foo": "bar" }] }),
                        evaluated_at: *EVALUATED_AT.clone()
                    };

                    let mock = MockUseMessageIdIfNoDeepHash;
                    _ = mock.save_evaluation(args).await;
                }

                struct MockNoopInsertEvaluationOrMessage;
                #[async_trait]
                impl SaveEvaluationSchema for MockNoopInsertEvaluationOrMessage {
                    async fn save_evaluation(&self, evaluation: EvaluationSchemaExtended) -> Result<(), CuErrors> {
                        match AoEvaluation::create_save_evaluation_queries(evaluation) {
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
                        ordinate: "1".to_string(),
                        block_height: 1234,
                        cron: None,
                        process_id: "process-123".to_string(),
                        message_id: Some("message-123".to_string()),
                        output: json!({ "Messages": [{ "foo": "bar" }], "Memory": "foo" }),
                        evaluated_at: *EVALUATED_AT.clone()
                    };

                    let mock = MockNoopInsertEvaluationOrMessage;
                    _ = mock.save_evaluation(args).await;
                }
            }
        }

        mod find_evaluations {
            use super::*;

            mod return_the_list_of_all_cron_evaluations {
                use super::*;

                struct MockReturnListOfAllCronEvaluations;
                #[async_trait]
                impl FindEvaluationsSchema for MockReturnListOfAllCronEvaluations {
                    async fn find_evaluations(
                        &self,
                        process_id: String, 
                        from: Option<FromOrToEvaluationSchema>, 
                        to: Option<FromOrToEvaluationSchema>, 
                        sort: Option<Sort>, 
                        limit: i64, 
                        only_cron: Option<bool>
                    ) -> Result<Vec<EvaluationSchema>, CuErrors> {
                        let mock_eval = EvaluationSchema {
                            timestamp: 1702677252111,
                            ordinate: "1".to_string(),
                            deep_hash: None,
                            block_height: 1234,
                            process_id: "process-123".to_string(),
                            message_id: Some("message-123".to_string()),
                            epoch: None,
                            nonce: None,
                            cron: None,
                            output: Value::Object(serde_json::Map::new()),
                            evaluated_at: Utc::now()
                        };

                        let query = AoEvaluation::create_find_evaluations_query(
                            process_id.as_str(), 
                            from, 
                            to, 
                            if let Some(only_cron) = only_cron { only_cron } else { false }, 
                            if let Some(sort) = sort { sort } else { Sort::Asc }, 
                            limit
                        );

                        assert!(query.sql.contains("AND cron IS NOT NULL"));
                        assert!(query.sql.contains("timestamp ASC"));
                        assert!(query.sql.contains("ordinate ASC"));
                        assert!(query.sql.contains("cron ASC"));

                        assert!(query.parameters[0] == "process-123,");
                        assert!(query.parameters[1] == format!("process-123,{},", COLLATION_SEQUENCE_MAX_CHAR_AS_I64));
                        assert!(query.parameters[2] == "10");

                        Ok(vec![
                            mock_eval.clone(),
                            mock_eval
                        ])
                    }
                }

                #[tokio::test]
                async fn test_return_the_list_of_all_cron_evaluations() {
                    let mock = MockReturnListOfAllCronEvaluations;
                    match mock.find_evaluations("process-123".to_string(), None, None, Some(Sort::Asc), 10, Some(true)).await {
                        Ok(res) => assert!(res.len() == 2),
                        Err(e) => panic!("{}", e)
                    }
                }
            }

            mod return_the_evaluations_between_from_and_to {
                use super::*;

                struct MockReturnEvalutionsBetweenFromAndTo;
                #[async_trait]
                impl FindEvaluationsSchema for MockReturnEvalutionsBetweenFromAndTo {
                    async fn find_evaluations(
                        &self,
                        process_id: String, 
                        from: Option<FromOrToEvaluationSchema>, 
                        to: Option<FromOrToEvaluationSchema>, 
                        sort: Option<Sort>, 
                        limit: i64, 
                        only_cron: Option<bool>
                    ) -> Result<Vec<EvaluationSchema>, CuErrors> {
                        let mock_eval = EvaluationSchema {
                            timestamp: 1702677252111,
                            ordinate: "3".to_string(),
                            deep_hash: None,
                            block_height: 1234,
                            process_id: "process-123".to_string(),
                            message_id: Some("message-123".to_string()),
                            epoch: None,
                            nonce: None,
                            cron: None,
                            output: json!({ "state": { "foo": "bar" } }),
                            evaluated_at: Utc::now()
                        };

                        let query = AoEvaluation::create_find_evaluations_query(
                            process_id.as_str(), 
                            from, 
                            to, 
                            if let Some(only_cron) = only_cron { only_cron } else { false }, 
                            if let Some(sort) = sort { sort } else { Sort::Asc }, 
                            limit
                        );

                        assert!(query.sql.contains("AND cron IS NOT NULL") != true);

                        assert!(query.parameters[0] == "process-123,1702677252111,3,");
                        assert!(query.parameters[1] == format!("process-123,1702677252111,{},", COLLATION_SEQUENCE_MAX_CHAR_AS_I64));
                        assert!(query.parameters[2] == "10");

                        Ok(vec![
                            mock_eval.clone(),
                            mock_eval
                        ])
                    }
                }

                #[tokio::test]
                async fn test_return_the_evaluations_between_from_and_to() {
                    let mock = MockReturnEvalutionsBetweenFromAndTo;
                    match mock.find_evaluations(
                        "process-123".to_string(), 
                        Some(FromOrToEvaluationSchema { timestamp: Some(1702677252111), ordinate: Some("3".to_string()), cron: None }), 
                        Some(FromOrToEvaluationSchema { timestamp: Some(1702677252111), ordinate: None, cron: None }), 
                        Some(Sort::Asc), 
                        10, 
                        None
                    ).await {
                        Ok(res) => assert!(res.len() == 2),
                        Err(e) => panic!("{}", e)
                    }
                }
            }
        }

        mod find_message_before {
            use super::*;

            mod find_prior_message_by_deep_hash {
                use super::*;

                struct MockFindPriorMsgByDeepHash;
                #[async_trait]
                impl FindMessageBeforeSchema for MockFindPriorMsgByDeepHash {
                    async fn find_message_before(
                        &self,
                        message_id: String,
                        deep_hash: Option<String>,
                        is_assignment: bool,
                        process_id: String,
                        epoch: i64,
                        nonce: i64
                    ) -> Result<EntityId, CuErrors> {
                        let query = AoEvaluation::create_find_message_before_query(message_id, deep_hash, is_assignment, process_id, nonce, epoch);
                        assert!(query.parameters[0] == "deepHash-123");
                        assert!(query.parameters[1] == "process-123");
                        assert!(query.parameters[2] == "0:3");

                        Ok(EntityId { id: "deepHash-123".to_string() })
                    }
                }

                #[tokio::test]
                async fn test_find_prior_message_by_deep_hash() {
                    let mock = MockFindPriorMsgByDeepHash;
                    match mock.find_message_before("message-123".to_string(), Some("deepHash-123".to_string()), false, "process-123".to_string(), 0, 3).await {
                        Ok(res) => assert!(res.id == "deepHash-123"),
                        Err(e) => panic!("{}", e)
                    }
                }
            }

            mod find_the_prior_message_by_message_id {
                use super::*;

                struct MockIfNoDeepHashWasCalculated;
                #[async_trait]
                impl FindMessageBeforeSchema for MockIfNoDeepHashWasCalculated {
                    async fn find_message_before(
                        &self,
                        message_id: String,
                        deep_hash: Option<String>,
                        is_assignment: bool,
                        process_id: String,
                        epoch: i64,
                        nonce: i64
                    ) -> Result<EntityId, CuErrors> {
                        let query = AoEvaluation::create_find_message_before_query(message_id, deep_hash, is_assignment, process_id, nonce, epoch);
                        assert!(query.parameters[0] == "message-123");
                        assert!(query.parameters[1] == "process-123");
                        assert!(query.parameters[2] == "0:3");

                        Ok(EntityId { id: "message-123".to_string() })
                    }
                }

                #[tokio::test]
                async fn test_if_no_deep_hash_was_calculated() {
                    let mock = MockIfNoDeepHashWasCalculated;
                    match mock.find_message_before("message-123".to_string(), None, false, "process-123".to_string(), 0, 3).await {
                        Ok(res) => assert!(res.id == "message-123"),
                        Err(e) => panic!("{}", e)
                    }
                }
            }

            mod if_it_is_an_assignment {
                use super::*;

                struct MockIsAnAssignment;
                #[async_trait]
                impl FindMessageBeforeSchema for MockIsAnAssignment {
                    async fn find_message_before(
                        &self,
                        message_id: String,
                        deep_hash: Option<String>,
                        is_assignment: bool,
                        process_id: String,
                        epoch: i64,
                        nonce: i64
                    ) -> Result<EntityId, CuErrors> {
                        let query = AoEvaluation::create_find_message_before_query(message_id, deep_hash, is_assignment, process_id, nonce, epoch);
                        assert!(query.parameters[0] == "message-123");
                        assert!(query.parameters[1] == "process-123");
                        assert!(query.parameters[2] == "0:3");

                        Ok(EntityId { id: "message-123".to_string() })
                    }
                }

                #[tokio::test]
                async fn test_if_it_is_an_assignment() {
                    let mock = MockIsAnAssignment;
                    match mock.find_message_before("message-123".to_string(), None, true, "process-123".to_string(), 0, 3).await {
                        Ok(res) => assert!(res.id == "message-123"),
                        Err(e) => panic!("{}", e)
                    }
                }
            }

            mod return_404_status_if_not_found {
                use super::*;

                struct MockIsAnAssignment;
                #[async_trait]
                impl FindMessageBeforeSchema for MockIsAnAssignment {
                    async fn find_message_before(
                        &self,
                        _message_id: String,
                        _deep_hash: Option<String>,
                        _is_assignment: bool,
                        _process_id: String,
                        _epoch: i64,
                        _nonce: i64
                    ) -> Result<EntityId, CuErrors> {
                        Err(CuErrors::HttpStatus(HttpError { status: 404, message: "".to_string() }))
                    }
                }

                #[tokio::test]
                async fn test_return_404_status_if_not_found() {
                    let mock = MockIsAnAssignment;
                    match mock.find_message_before("message-123".to_string(), Some("deepHash-123".to_string()), true, "process-123".to_string(), 0, 3).await {
                        Ok(_) => panic!("Should not return message"),
                        Err(e) => if let CuErrors::HttpStatus(e) = e {
                            assert!(e.status == 404);
                        } else {
                            panic!("Returned unexpected error")
                        }
                    }
                }
            }
        }
    }
}