use std::sync::Arc;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::sqlite::SqliteRow;
use sqlx::{Sqlite, FromRow, Row};
use crate::domain::model::model::Output;
use crate::domain::utils::error::{CuErrors, HttpError, SchemaValidationError};

use super::sqlite::{SqliteClient, ConnGetter, EVALUATIONS_TABLE};

#[allow(unused)]
pub struct Query {
    pub sql: String,
    pub parameters: Vec<String>
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationDocSchema {
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
    output: Output
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
    ordinate: String,
    block_height: i64,
    cron: Option<String>,
    evaluated_at: DateTime<Utc>,
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

pub enum CreateMessageIdResult {
    MessageId(String),
    DeepHash(Vec<u8>)
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

    fn create_evaluation_id(process_id: &str, timestamp: i64, ordinate: &str, cron: Option<String>) -> String {
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
    fn create_message_id (message_id: &str, deep_hash: Option<Vec<u8>>, is_assignment: bool) -> CreateMessageIdResult {
        if is_assignment {
            return CreateMessageIdResult::MessageId(message_id.to_string());
        }
        if deep_hash.is_some() {
            return CreateMessageIdResult::DeepHash(deep_hash.unwrap());
        }
        CreateMessageIdResult::MessageId(message_id.to_string())
    }

    fn create_select_query (process_id: &str, timestamp: i64, ordinate: &str, cron: Option<String>) -> Query {
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

    fn from_evaluation_doc(evaluation_query_schema: &EvaluationQuerySchema) -> EvaluationDocSchema {
        EvaluationDocSchema {
            id: evaluation_query_schema.id.clone(), 
            process_id: evaluation_query_schema.process_id.clone(),
            message_id: evaluation_query_schema.message_id.clone(),
            deep_hash: evaluation_query_schema.deep_hash.clone(),
            timestamp: evaluation_query_schema.timestamp,
            epoch: evaluation_query_schema.epoch,
            nonce: evaluation_query_schema.nonce,
            ordinate: evaluation_query_schema.ordinate.clone(),
            block_height: evaluation_query_schema.block_height,
            cron: evaluation_query_schema.cron.clone(),
            evaluated_at: evaluation_query_schema.evaluated_at,
            output: serde_json::from_str(&evaluation_query_schema.output).unwrap()
        }
    }

    pub async fn find_evaluation(&self, process_id: &str, timestamp: i64, ordinate: &str, cron: Option<String>) -> Result<EvaluationDocSchema, CuErrors> {
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
}
