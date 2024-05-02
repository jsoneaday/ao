use crate::domain::model::model::{BlockSchema, EvaluationSchema, EvaluationSchemaExtended, FromOrToEvaluationSchema, MessageMetaSchema, ModuleSchema, ProcessSchema, ProcessSchemaWithoutId, ProcessUrl, RawTagSchema, Sort, StreamSchema, TimestampSchema};
use async_trait::async_trait;
use reqwest::Url;

// todo: the Vec<u8> types might be better as serde Value types?

#[async_trait]
pub trait LoadTransactionMetaSchema {
    async fn load_transaction_meta_schema(module_id: Option<String>) -> ModuleSchema;
}

#[async_trait]
pub trait LoadTransactionDataSchema {
    async fn load_transaction_data_schema(tx_id: String) -> Vec<u8>;
}

#[async_trait]
pub trait LoadBlockMetaSchema {
    async fn load_block_meta_schema(min: u64, max_time_stamp: u64) -> Vec<BlockSchema>;
}

#[async_trait]
pub trait FindProcessSchema {
    async fn find_process_schema(process_id: String) -> ProcessSchema;
}

#[async_trait]
pub trait SaveProcessSchema {
    async fn save_process_schema(process_schema: ProcessSchema) -> Vec<u8>;
}

#[async_trait]
pub trait FindModuleSchema {
    async fn find_module_schema(module_id: String) -> ModuleSchema;
}

#[async_trait]
pub trait SaveModuleSchema {
    async fn save_module_schema(module_schema: ModuleSchema) -> Vec<u8>;
}

#[async_trait]
pub trait EvaluatorSchema {
    async fn evaluator_schema(input: Vec<u8>) -> Vec<u8>;
}

#[async_trait]
pub trait FindEvaluationSchema {
    async fn find_evaluation_schema(process_id: String, to: Option<u64>, ordinate: Option<String>, cron: Option<String>) -> EvaluationSchema;
}

#[async_trait]
pub trait SaveEvaluationSchema {
    async fn save_evaluation_schema(evaluation_schema: EvaluationSchemaExtended) -> Vec<u8>;
}

#[async_trait]
pub trait FindEvaluationsSchema {
    /// sort defauls to Asc
    /// only_cron default to false
    async fn find_evaluations_schema(
        process_id: String, 
        from: FromOrToEvaluationSchema, 
        to: FromOrToEvaluationSchema, 
        sort: Option<Sort>, 
        limit: u64, 
        only_cron: Option<bool>
    ) -> Vec<EvaluationSchema>;
}

#[async_trait]
pub trait FindMessageBeforeSchema {
    async fn find_message_before_schema(
        message_id: Option<String>,
        deep_hash: Option<String>,
        is_assignment: bool,
        process_id: String,
        epoch: u64,
        nonce: u64
    ) -> Vec<u8>;
}

#[async_trait]
pub trait SaveBlockSchema {
    async fn save_block_schema(block_schema: BlockSchema) -> Vec<u8>;
}

#[async_trait]
pub trait FindBlockSchema {
    async fn find_block_schema(min_height: u64, max_timestamp: u64) -> Vec<BlockSchema>;
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