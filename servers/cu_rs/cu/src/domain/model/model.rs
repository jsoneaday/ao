use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use valid::ValidationError;
use once_cell::sync::OnceCell;
use super::domain_config_schema::{DomainConfigSchema, StartDomainConfigSchema};
use super::parse_schema::StartSchemaParser;
use serde::Serialize;

static DOMAIN_CONFIG_SCHEMA: OnceCell<Result<DomainConfigSchema, ValidationError>> = OnceCell::new();

pub enum Sort {
    Asc,
    Desc
}

impl ToString for Sort {
    fn to_string(&self) -> String {
        match self {
            Sort::Asc => "ASC".to_string(),
            Sort::Desc => "DESC".to_string()
        }
    }
}

pub fn domain_config_schema<'a>(start_schema: StartDomainConfigSchema) -> &'a Result<DomainConfigSchema, ValidationError> {
    DOMAIN_CONFIG_SCHEMA.get_or_init(|| {
        start_schema.parse()
    })
}

#[allow(unused)]
pub struct RawTagSchema {
    name: String,
    value: String
}

#[allow(unused)]
pub struct Owner {
    address: String,
    key: String
}

#[allow(unused)]
pub struct SimpleModuleSchema {
    id: String,
    /// address
    owner: String,
    tags: Vec<RawTagSchema>
}

#[allow(unused)]
pub struct ModuleSchema {
    id: String,
    owner: Owner,
    tags: Vec<RawTagSchema>
}

#[derive(FromRow)]
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

#[allow(unused)]
pub struct ProcessSchema {
    id: String,
    signature: Option<String>,
    data: Option<Vec<u8>>,
    anchor: Option<String>,
    /// min 1
    owner: String,
    tags: Vec<RawTagSchema>,
    block: BlockSchema
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

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Output {
    memory: Option<Vec<u8>>,
    messages: Option<Vec<u8>>,
    assignments: Option<Vec<u8>>,
    spawns: Option<Vec<u8>>,
    output: Option<Vec<u8>>,
    gas_used: Option<i64>,
    error: Option<Vec<u8>>
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationSchema {
    process_id: String,
    message_id: String,
    deep_hash: String,
    timestamp: i64,
    epoch: i64,
    nonce: i64,
    ordinate: String,
    block_height: i64,
    cron: Option<String>,
    evaluated_at: DateTime<Utc>,
    output: Output
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationSchemaExtended {
    process_id: String,
    message_id: String,
    deep_hash: Option<String>,
    timestamp: i64,
    epoch: i64,
    nonce: i64,
    ordinate: String,
    block_height: i64,
    cron: Option<String>,
    evaluated_at: DateTime<Utc>,
    output: Output,
    is_assignment: bool
}

#[allow(unused)]
pub struct FromOrToEvaluationSchema {
    timestamp: Option<i64>,
    ordinate: Option<String>,
    cron: Option<String>
}

pub trait StreamSchema {
    fn pipe(&self);
}

#[allow(unused)]
pub struct ProcessUrl {
    url: String
}