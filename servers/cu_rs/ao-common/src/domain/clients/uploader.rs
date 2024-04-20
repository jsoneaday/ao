use std::sync::Arc;
use async_trait::async_trait;
use reqwest::{Client, Url};
extern crate serde;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use crate::domain::core::dal::{Uploader, Log, UploaderErrorType};

pub struct UploaderClient {
    node_url: Url,
    logger: Arc<dyn Log>,
}

#[derive(Debug, Deserialize, Serialize)]
struct IrysResponse {
    id: String,
    timestamp: u64,
    version: String,
    public: String,
    signature: String,
}

impl From<reqwest::Error> for UploaderErrorType {
    fn from(error: reqwest::Error) -> Self {
        UploaderErrorType::UploadError(format!("Request error: {}", error))
    }
}

impl From<serde_json::Error> for UploaderErrorType {
    fn from(error: serde_json::Error) -> Self {
        UploaderErrorType::UploadError(format!("Request error: {}", error))
    }
}

impl UploaderClient {
    pub fn new(node_url: &str, logger: Arc<dyn Log>) -> Result<Self, UploaderErrorType> {
        let url = match Url::parse(node_url) {
            Ok(u) => u,
            Err(e) => return Err(UploaderErrorType::UploadError(format!("{}", e))),
        };

        Ok(UploaderClient {
            node_url: url,
            logger,
        })
    }
}

#[async_trait]
impl Uploader for UploaderClient {
    async fn upload(&self, tx: Vec<u8>) -> Result<(), UploaderErrorType> {
        let node_url_clone = self.node_url.clone();
        let tx_clone = tx.clone();
        let logger_clone = Arc::clone(&self.logger);

        println!("start upload attempt");
        let client = Client::new();

        for _attempt in 0..100 {
            println!("_attempt {:?}", _attempt);
            let response = client
                .post(
                    node_url_clone
                        .join(&format!("tx/{}", "arweave".to_string()))
                        .expect("Failed to join URL"), // Handle URL joining error
                )
                .header("Content-Type", "application/octet-stream")
                .body(tx_clone.clone())
                .send()
                .await;
            println!("post response {:?}", response);
            match response {
                Ok(resp) if resp.status().is_success() => {
                    // Handle success
                    logger_clone.log("Upload successful".to_string());
                    break; // Exit the loop on success
                }
                Ok(resp) => {
                    // Handle non-success HTTP status
                    logger_clone.error(format!("Non-success status: {}", resp.status()));
                    sleep(Duration::from_secs(1)).await;
                }
                Err(e) => {
                    // Handle request error
                    logger_clone.error(format!("Request error: {}", e));
                    sleep(Duration::from_secs(1)).await;
                }
            }
        }

        Ok(())
    }
}
