use async_trait::async_trait;
use serde::Deserialize;
pub use super::json::{JsonErrorType, Message, PaginatedMessages, Process};

#[derive(Deserialize)]
pub struct NetworkInfo {
    pub height: String,
    pub current: String,
}

#[derive(Deserialize)]
pub struct TxStatus {
    pub block_height: i32,
    pub number_of_confirmations: i32,
}

#[async_trait]
pub trait Gateway: Send + Sync {
    async fn check_head(&self, tx_id: String) -> Result<bool, String>;
    async fn network_info(&self) -> Result<NetworkInfo, String>;
    async fn status(&self, tx_id: &String) -> Result<TxStatus, String>;
}

pub trait Wallet: Send + Sync {
    fn wallet_json(&self) -> Result<String, String>;
    fn wallet_address(&self) -> Result<String, String>;
}

#[async_trait]
pub trait Signer: Send + Sync {
    async fn sign_tx(&self, buffer: Vec<u8>) -> Result<Vec<u8>, String>;
    fn get_public_key(&self) -> Vec<u8>;
}

pub trait Log: Send + Sync {
    fn log(&self, message: String);
    fn error(&self, message: String);
}

pub trait ScheduleProvider {
    fn epoch(&self) -> String;
    fn nonce(&self) -> String;
    fn timestamp(&self) -> String;
    fn hash_chain(&self) -> String;
}

pub trait Config: Send + Sync {
    fn wallet_path(&self) -> String;
    fn upload_node_url(&self) -> String;
    fn gateway_url(&self) -> String;
    fn mode(&self) -> String;
    // fn scheduler_list_path(&self) -> String;
}

#[derive(Debug)]
pub enum UploaderErrorType {
    UploadError(String),
}

impl From<UploaderErrorType> for String {
    fn from(error: UploaderErrorType) -> Self {
        format!("{:?}", error)
    }
}

#[async_trait]
pub trait Uploader: Send + Sync {
    async fn upload(&self, tx: Vec<u8>) -> Result<(), UploaderErrorType>;
}

#[derive(Debug)]
pub enum StoreErrorType {
    DatabaseError(String),
    NotFound(String),
    JsonError(String),
    EnvVarError(String),
    IntError(String),
    MessageExists(String),
}
