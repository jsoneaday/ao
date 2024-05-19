use std::fmt::Display;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::prelude::{Row,FromRow};
use sqlx::sqlite::SqliteRow;
use valid::ValidationError;
use once_cell::sync::OnceCell;
use validator::Validate;
use super::domain_config_schema::{DomainConfigSchema, StartDomainConfigSchema};
use super::parse_schema::StartSchemaParser;
use serde::{Deserialize, Serialize};

// todo: need to add validation and constraints into Schema defs

static DOMAIN_CONFIG_SCHEMA: OnceCell<Result<DomainConfigSchema, ValidationError>> = OnceCell::new();

pub enum Sort {
    Asc,
    Desc
}

impl Display for Sort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sort::Asc => write!(f, "{}", "ASC"),
            Sort::Desc => write!(f, "{}", "DESC")
        }
    }
}

pub fn domain_config_schema<'a>(start_schema: StartDomainConfigSchema) -> &'a Result<DomainConfigSchema, ValidationError> {
    DOMAIN_CONFIG_SCHEMA.get_or_init(|| {
        start_schema.parse()
    })
}

#[derive(Debug)]
pub struct EntityId {
    pub id: String
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[allow(unused)]
pub struct RawTagSchema {
    pub name: String,
    pub value: String
}

impl<'r> FromRow<'r, SqliteRow> for RawTagSchema {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(
            RawTagSchema { 
                name: row.try_get("name")?, 
                value: row.try_get("value")?
            }
        )
    }
}

#[derive(Clone)]
#[allow(unused)]
pub struct Owner {
    pub address: String,
    pub key: String
}

#[derive(Validate, Clone, Debug)]
#[allow(unused)]
pub struct ModuleSchema {
    #[validate(length(min = 1))]
    pub id: String, 
    /// A json string (saved as string in db)   
    pub tags: Vec<RawTagSchema>,    
    #[validate(length(min = 1))]
    pub owner: String
}

#[derive(FromRow, Clone, Deserialize, Serialize)]
pub struct BlockSchema {
    pub height: i64,
    pub timestamp: i64
}

pub type TimestampSchema = BlockSchema;

#[allow(unused)]
pub struct MessageMetaSchema {
    height: i64,
    timestamp: i64,
    nonce: i64
}

#[derive(FromRow)]
#[allow(unused)]
pub struct MessageBeforeSchema {
    pub id: String,
    pub seq: String
}

#[derive(Validate, Clone)]
#[allow(unused)]
pub struct ProcessSchema {
    #[validate(length(min = 1))]
    pub id: String,
    pub signature: Option<String>,
    // todo: this might be an Any
    pub data: String,
    pub anchor: Option<String>,
    #[validate(length(min = 1))]
    pub owner: String,
    pub tags: Vec<RawTagSchema>,
    pub block: BlockSchema
}

#[allow(unused)]
pub struct ProcessSchemaWithoutId {    
    signature: Option<String>,
    data: Option<Vec<u8>>,
    anchor: Option<String>,
    /// min 1
    owner: String,
    tags: Vec<RawTagSchema>,
    block: BlockSchema
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Output {
    memory: Option<String>,
    messages: Option<Vec<u8>>,
    assignments: Option<Vec<u8>>,
    spawns: Option<Vec<u8>>,
    output: Option<Vec<u8>>,
    gas_used: Option<i64>,
    error: Option<Vec<u8>>
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationSchema {
    /**
    * the id of the process that the message was performed upon
    */
    pub process_id: String,
    /**
    * Cron messages do not have a messageId
    * and so can be undefined
    */
    pub message_id: Option<String>,
    /**
    * Only forwarded messages have a deepHash
    */
    pub deep_hash: Option<String>,
    pub timestamp: i64,
    /**
    * Cron messages do not have an epoch
    */
    pub epoch: Option<i64>,
    /**
    * Cron messages do not have a nonce
    */
    pub nonce: Option<i64>,
    /**
    * Used for ordering the evaluation stream and results in the CU
    *
    * For a Scheduled Message, this will always simply be it's nonce.
    * For a Cron Message, this will be the nonce of the most recent Scheduled Message.
    */
    pub ordinate: String,
    pub block_height: i64,
    /**
    * Scheduled messages do not have a cron,
    * and so can be undefined
    */
    pub cron: Option<String>,
    /**
    * The date when this record was created, effectively
    * when this record was evaluated
    *
    * not to be confused with when the transaction was placed on chain
    */
    pub evaluated_at: DateTime<Utc>,
    pub output: Value
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationSchemaExtended {
    pub process_id: String,
    pub message_id: Option<String>,
    pub deep_hash: Option<String>,
    pub timestamp: i64,
    pub epoch: Option<i64>,
    pub nonce: Option<i64>,
    pub ordinate: String,
    pub block_height: i64,
    pub cron: Option<String>,
    pub evaluated_at: DateTime<Utc>,
    pub output: Value,
    pub is_assignment: bool
}

#[derive(Clone)]
#[allow(unused)]
pub struct FromOrToEvaluationSchema {
    pub timestamp: Option<i64>,
    pub ordinate: Option<String>,
    pub cron: Option<String>
}

pub trait StreamSchema {
    fn pipe(&self);
}

#[allow(unused)]
pub struct ProcessUrl {
    url: String
}

pub mod gql_return_types {
    use ao_common::models::gql_models::PageInfo;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    pub struct DataBlocks {
        pub data: Blocks
    }

    #[derive(Deserialize, Debug)]
    pub struct Blocks {
        pub blocks: BlocksTransactions
    }

    #[derive(Deserialize, Debug)]
    pub struct BlocksTransactions {
        #[serde(rename = "pageInfo")]
        pub page_info: Option<PageInfo>,
        pub edges: Vec<Edge>
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct Edge {
        pub node: Node
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct Node {
        pub timestamp: i64,
        pub height: i64
    }
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Timestamp")]
    pub timestamp: i64,
    #[serde(rename = "Owner")]
    pub owner: String,
    #[serde(rename = "Tags")]
    pub tags: Vec<RawTagSchema>,
    #[serde(rename = "Block-Height")]
    pub block_height: i64
}

#[derive(Serialize, Deserialize)]
pub struct Process {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Tags")]
    pub tags: Vec<RawTagSchema>
}

#[derive(Serialize, Deserialize)]
pub struct AoGlobal {
    #[serde(rename = "Process")]
    pub process: Process
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateArgs {
    pub name: String,
    pub no_save: bool,
    pub deep_hash: Option<String>,
    pub cron: Option<String>,
    pub ordinate: String,
    pub is_assignment: bool,
    pub process_id: String,
    #[serde(rename = "Memory")]
    pub memory: Option<i64>,
    pub message: Message,
    #[serde(rename = "AoGlobal")]
    pub ao_global: AoGlobal,
    pub stream_id: [u8; 8],
    pub module_id: String,
    pub module_options: Option<ModuleOptions>
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleOptions {
    pub format: String,
    pub input_encoding: String,
    pub output_encoding: String,
    pub memory_limit: i64,
    pub compute_limit: i64,
    pub extensions: Vec<String>
}