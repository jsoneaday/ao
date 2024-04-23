use std::sync::Arc;
use reqwest::{Client, Url};
extern crate serde;
use serde::{Deserialize, Serialize};
use tokio::{spawn, time::{sleep, Duration}};
use crate::domain::core::dal::{Uploader, Log, UploaderErrorType};

pub struct UploaderClient {
    node_url: Url,
    logger: Arc<dyn Log>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IrysResponse {
    id: String,
    timestamp: u64,
    winc: String,
    version: String,
    #[serde(rename = "deadlineHeight")]
    deadline_height: u64,
    public: String,
    signature: String,
    owner: String
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

impl Uploader for UploaderClient {
    fn try_upload(&self, tx: Vec<u8>) -> Result<(), UploaderErrorType> {
        let node_url_clone = self.node_url.clone();
        let tx_clone = tx.clone();
        let logger_clone = Arc::clone(&self.logger);

        spawn(async move {
            let client = Client::new();

            for _attempt in 0..100 {
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
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use bundlr_sdk::tags::Tag;
    use std::path::PathBuf;
    use std::sync::Arc;
    use crate::domain::dal::{Gateway, NetworkInfo, TxStatus, Signer, ScheduleProvider};
    use crate::domain::bytes::DataItem;
    use crate::domain::signer::ArweaveSigner;
    use crate::test_utils::{get_uploader_url, get_wallet_file};

    const _ITEM_STR: &str = "AQB9q2yhsQlBHv2LOTIrtmKjw063S1DG0prKcq86DykIegmPnXOReXkWXwpqXt4YxTRw6Rw1jG7f1QFF5ReoJO2MrJmia9ymkTmnhamv3lsYYIotBC6U4Bmzo6IZiKmn2llJt0MDvCe8rxzG15vvff9bpnDIVflY_Dm9Y0dCH-w2Xg8rb2xLq-cM8SBoNRiYruwcwpahiHTjXcxboJKksZRXaI_E7_7vL1gWlMLqeYeF_uXqkth8_PGtZcqMA7pbTYcRzGki_rifGXKUIZKgSIRXTk54iboiqNzOklIFpDKDJpC9Xk_6ppSw_Xzs8S0KpR-veBL8TeURtGhrsDecu_36Pk2MMvdZedxiAg7bvQ9H_NZecoZcju-sQKZiE7haq9Nos3g6njh9IpXivGJ1k8tRLeox7hXOeynffzcXz1Vnz5c4Zxw8LKUbLygni49sflKyFTMnQ8sgDw00fPsuhrznq37-2OLhmYe-tIg-TEV3T4VNdqchzeRSFIv_l7ZJcxeFxcEgdq9aXMx2yzVhSInFuk_W8fJSbhPKX9cewbr4BA_XUNMReowLVcnjB_19iCWnivkVk9sz-QRbjuVL2IMqZePWcRdN5ncXRJoYv4F-Z4FfXDCFuyCD4UAtiQfdch-S4KvRf99DwKrZrMIF28MDdRFdE3ZGDs3FXcPuN8eMLoKBrkyfkM3J89W1GNvrcCNHSNzhF8oPItU4Qno7-x52ZIOAjfdFcXTYLQYU7Xfr6GKaRByemPrkbkrJpdB8RQREt3rQRDNGRQ0jnbPn62PQugvss98JZn9D4ScNusbbgKMihj4MqfXE2mt7Ab9ewx5d01d-Mwf3D6mGz_ERBJgJo8b119bRXdNvgUDJC58NFd4chEOUF4mbyj2pZB9P7fx22yEvV7y6DNzuKvk02YQt7TwL7sdxH1PT63CYJx0tlVGGDvJhGKUQwOfDaXHFMjuuUlXa_klTJT5wEb78aAyh33rw0n9wpOakTIk2KgekbJAzVWCT0BfLrrOhKs3556_d--2mLmcLOONosBjSLokuvtyrTOX7btKRf6Zl5l3wtxsFaPgO6M3Qy9UR46AtK76XSFQd9kcDf_Qj1FyronJS_enQFWYn5Um97mDnYT9SJwMpDFS_FYBTKlsNhsVy11EW5kKuo6mTRlfebJa9CQv-NzbUajd7ulAcM4VNWYt-KbbhVZtUUUxgDvXJdlwRSYR5U8JwSze3sfatb5mbds-EAS-tT7grwrvTb4wRz20e9ARtBg6kC_x8QujHmFORJ97zrFlnnunPbsWgwWz8bfT9RMFy5xUE1KDCtnJqp-M3FoWwQc4sREIyCl7Q6JTq_slPe-Xwt9C5oquj4e_SoOuTAfqDPAmIG6rEXKSN7RP3KRjN5IA5Wpp2I0hgOJ6bT2qNAAUAAAAAAAAASAAAAAAAAAAKGkRhdGEtUHJvdG9jb2wEYW8QZnVuY3Rpb24GcmF3GkRhdGEtUHJvdG9jb2wEYW8OYW8tdHlwZQ5tZXNzYWdlBlNESwRhbwA2NTgz";

    struct MockGateway;
    #[async_trait]
    impl Gateway for MockGateway {
        async fn check_head(&self, _tx_id: String) -> Result<bool, String> {
            Ok(true)
        }

        async fn network_info(&self) -> Result<NetworkInfo, String> {
            Ok(NetworkInfo {
                height: "1000".to_string(),
                current: "test-network".to_string(),
            })
        }

        async fn status(&self, _tx_id: &String) -> Result<TxStatus, String> {
            Ok(TxStatus {
                block_height: 0,
                number_of_confirmations: 0,
            })
        }
    }

    struct MockSigner;
    #[async_trait]
    impl Signer for MockSigner {
        async fn sign_tx(&self, _buffer: Vec<u8>) -> Result<Vec<u8>, String> {
            Ok(vec![1, 2, 3, 4])
        }

        fn get_public_key(&self) -> Vec<u8> {
            vec![5, 6, 7, 8]
        }
    }

    struct MockLogger;
    #[async_trait]
    impl Log for MockLogger {
        fn log(&self, message: String) {
            println!("{}", message)
        }
        fn error(&self, message: String) {
            println!("{}", message);
        }
    }

    struct MockScheduler;
    impl ScheduleProvider for MockScheduler {
        fn epoch(&self) -> String {
            "epoch".to_string()
        }
        fn nonce(&self) -> String {
            "nonce".to_string()
        }
        fn timestamp(&self) -> String {
            "timestamp".to_string()
        }
        fn hash_chain(&self) -> String {
            "hash_chain".to_string()
        }
    }

    #[tokio::test]
    async fn test_uploaderclient_uploading() {
        let logger: Arc<dyn Log> = Arc::new(MockLogger);

        let mut path = PathBuf::new();
        path.push("test.png");

        let signer = Arc::new(ArweaveSigner::new(get_wallet_file()).expect("Invalid su wallet path"));
        let data = std::fs::read(path).unwrap();
        let mut data_item = DataItem::new(
            vec![], 
            data, 
            vec![
                Tag { 
                    name: "Bundle-Format".to_string(), 
                    value: "binary".to_string()
                },
                Tag { 
                    name: "Bundle-Version".to_string(), 
                    value: "2.0.0".to_string() 
                }
            ], 
            signer.get_public_key()
        ).unwrap();
        data_item.signature = signer.sign_tx(data_item.get_message().unwrap().to_vec()).await.unwrap();
                
        let uploader = UploaderClient::new(&get_uploader_url(), logger).unwrap();
        let result = uploader.try_upload(data_item.as_bytes().unwrap());
        assert!(result.is_ok());
    }
}