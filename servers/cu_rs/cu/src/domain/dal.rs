use crate::domain::model::model::{
    BlockSchema, EvaluationSchema, EvaluationSchemaExtended, FromOrToEvaluationSchema, MessageMetaSchema, ModuleSchema, ProcessSchema, 
    ProcessSchemaWithoutId, ProcessUrl, RawTagSchema, Sort, StreamSchema, TimestampSchema
};
use async_trait::async_trait;
use reqwest::Url;

use super::{model::model::{gql_return_types, EntityId, EvaluateArgs, ModuleOptions, Output}, utils::error::CuErrors};

// todo: the Vec<u8> types might be better as serde Value types?

#[async_trait]
pub trait LoadTransactionMetaSchema {
    async fn load_transaction_meta_schema(module_id: Option<String>) -> ModuleSchema;
}

#[async_trait]
pub trait LoadTransactionDataSchema {
    async fn load_transaction_data(tx_id: String) -> Vec<u8>;
}

#[async_trait]
pub trait LoadBlocksMetaSchema {
    async fn load_blocks_meta(&self, min_height: i64, max_time_stamp: i64, graphql_url: &str, page_size: i64) -> Result<Vec<gql_return_types::Node>, CuErrors>;
}

#[async_trait]
pub trait FindProcessSchema {
    async fn find_process(&self, process_id: &str) -> Result<ProcessSchema, CuErrors>;
}

#[async_trait]
pub trait SaveProcessSchema {
    async fn save_process(&self, process_schema: ProcessSchema) -> Result<(), CuErrors>;
}

#[async_trait]
pub trait FindModuleSchema {
    async fn find_module(&self, module_id: &str) -> Result<Option<ModuleSchema>, CuErrors>;
}

#[async_trait]
pub trait SaveModuleSchema {
    async fn save_module(&self, module_schema: ModuleSchema) -> Result<String, CuErrors>;
}

#[async_trait]
pub trait EvaluatorSchema {
    async fn evaluator<F>(&self, evaluate: F, module_id: &str, module_options: ModuleOptions, args: EvaluateArgs) 
        where F: Fn(EvaluateArgs) -> Result<Output, CuErrors>;
}

#[async_trait]
pub trait FindEvaluationSchema {
    /// to: is timestamp
    async fn find_evaluation(&self, process_id: &str, to: i64, ordinate: &str, cron: Option<String>) -> Result<Option<EvaluationSchema>, CuErrors>;
}

#[async_trait]
pub trait SaveEvaluationSchema {
    async fn save_evaluation(&self, evaluation_schema: EvaluationSchemaExtended) -> Result<(), CuErrors> ;
}

#[async_trait]
pub trait FindEvaluationsSchema {
    /// sort defauls to Asc
    /// only_cron defaults to false
    async fn find_evaluations(
        &self,
        process_id: String, 
        from: Option<FromOrToEvaluationSchema>, 
        to: Option<FromOrToEvaluationSchema>, 
        sort: Option<Sort>, 
        limit: i64, 
        only_cron: Option<bool>
    ) -> Result<Vec<EvaluationSchema>, CuErrors>;
}

#[async_trait]
pub trait FindMessageBeforeSchema {
    async fn find_message_before(
        &self,
        message_id: String,
        deep_hash: Option<String>,
        is_assignment: bool,
        process_id: String,
        epoch: i64,
        nonce: i64
    ) -> Result<EntityId, CuErrors>;
}

#[async_trait]
pub trait SaveBlocksSchema {
    async fn save_blocks(&self, blocks: &Vec<BlockSchema>) -> Result<(), sqlx::error::Error>;
}

#[async_trait]
pub trait FindBlocksSchema {
    async fn find_blocks(&self, min_height: i64, max_timestamp: i64) -> Result<Vec<BlockSchema>, sqlx::error::Error>;
}

#[async_trait]
pub trait LoadMessageSchema {
    async fn load_message_schema(
        su_url: Url,
        process_id: String,
        owner: String,
        tags: Vec<RawTagSchema>,
        module_id: String,
        module_tags: Vec<RawTagSchema>,
        module_owner: String,
        from: Option<u64>,
        to: Option<u64>
    ) -> impl StreamSchema;
}

#[async_trait]
pub trait LoadProcessSchema {
    async fn load_process_schema(su_url: String, process_id: String) -> ProcessSchemaWithoutId;
}

#[async_trait]
pub trait LoadTimestampSchema {
    async fn load_timestamp_schema(su_url: String, process_id: String) -> TimestampSchema;
}

#[async_trait]
pub trait LoadMessageMetaSchema {
    async fn load_message_meta_schema(su_url: String, process_id: String, message_tx_id: String) -> MessageMetaSchema;
}

#[async_trait]
pub trait LocateProcessSchema {
    async fn locate_process_schema(process_id: String, scheduler_hint: Option<String>) -> ProcessUrl;
}