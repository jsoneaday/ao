use arweave_rs::Arweave;
use arweave_rs::ArweaveBuilder;
use rand::Rng;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Error, Response, Url};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[allow(unused)]
use log::{error, info};
use crate::errors::QueryGatewayErrors;
use crate::network::utils::get_content_type_headers;
use crate::models::gql_models::{Node, TransactionConnectionSchema};

pub struct InternalArweave {
    internal_arweave: Arweave,
    uploader_url: String,
    wallet_path: String,
    client: Client
}

impl InternalArweave {
    pub fn new(keypair_path: &str, uploader_url: &str) -> Self {
        InternalArweave {
            internal_arweave: InternalArweave::create_wallet_client(keypair_path, uploader_url),
            uploader_url: uploader_url.to_string(),
            wallet_path: keypair_path.to_string(),
            client: Client::new()
        }
    }

    fn create_wallet_client(keypair_path: &str, uploader_url: &str) -> Arweave {
        let mut path = PathBuf::new();
        path.push(keypair_path);

        let arweave_builder = ArweaveBuilder::new();
        arweave_builder
             .keypair_path(path)
             .base_url(Url::parse(uploader_url).unwrap())
             .build().unwrap() 
        
        // arweave.upload_file_from_path(file_path, additional_tags, fee);
    }

    pub fn address(&self) -> Result<String, QueryGatewayErrors> {
        match self.internal_arweave.get_wallet_address() {
            Ok(res) => Ok(res),
            Err(e) => Err(QueryGatewayErrors::WalletError(e))
        }
    }

    /// todo: may need to switch to non-random
    pub fn create_random_anchor() -> [u8; 32] {
        let mut rng = rand::thread_rng();
        let mut anchor = [0; 32];
        rng.fill(&mut anchor[..]);
        anchor
    }

    /// A DataItem allows for uploading of a bundled item without sender themselves having to pay for it
    // pub fn build_sign_dataitem(&self, data: Vec<u8>, anchor: Option<[u8;32]>, tags: Vec<Tag>) -> Result<DataItem, QueryGatewayErrors> {
    //     let signer = ArweaveSigner::new(jwk, &self.wallet_path);
        
    //     match create_data(ar_bundles::ar_data_create::Data::BinaryData(data), &signer, Some(&DataItemCreateOptions {
    //         target: None,
    //         anchor,
    //         tags: Some(tags)
    //     })) {
    //         Ok(mut di) => {
    //             di.sign(&signer);
    //             Ok(di)
    //         },
    //         Err(e) => Err(QueryGatewayErrors::BundlerFailure(e))
    //     }
    // }

    /**
     * @typedef Env1
     * @property {fetch} fetch
     * @property {string} GATEWAY_URL
     *
     * @callback LoadTransactionMeta
     * @param {string} id - the id of the process whose src is being loaded
     * @returns {Async<z.infer<typeof transactionConnectionSchema>['data']['transactions']['edges'][number]['node']>}
     *
     * @param {Env1} env
     * @returns {LoadTransactionMeta}
    */
    pub async fn load_tx_meta(self, gateway_url: &str, id: &str) -> Result<Node, QueryGatewayErrors> {
        #[allow(non_snake_case)]
        let GET_PROCESSES_QUERY = r#"
            query GetProcesses ($processIds: [ID!]!) {
            transactions(ids: $processIds) {
                edges {
                node {
                    id
                    signature
                    anchor
                    owner {
                    address
                    }
                    tags {
                    name
                    value
                    }
                }
                }
            }
        }"#;

        let result = self.query_gateway::<ProcessIds, TransactionConnectionSchema>(gateway_url, GET_PROCESSES_QUERY, ProcessIds {
            process_ids: vec![id.to_string()]
        }).await;

        match result {
            Ok(tx) => {
                Ok(tx.data.transactions.edges[0].node.clone())
            },
            Err(e) => {
                error!("Error Encountered when fetching transaction {} from gateway {}", id, gateway_url);
                Err(e)
            }
        }
    }

    /**
    * @typedef Env2
    * @property {fetch} fetch
    * @property {string} GATEWAY_URL
    *
    * @callback LoadTransactionData
    * @param {string} id - the id of the process whose src is being loaded
    * @returns {Async<Response>}
    *
    * @param {Env2} env
    * @returns {LoadTransactionData}
    */
    pub async fn load_tx_data(&self, gateway_url: &str, id: &str) -> Result<Response, Error> {
        let result = self.client.get(format!("{}/raw/{}", gateway_url, id)).send().await;
        match result {
            Ok(res) => Ok(res),
            Err(e) => {
                error!("Error Encountered when fetching raw data for transaction {} from gateway {}", id, gateway_url);
                Err(e)
            }
        }
    }

    pub async fn query_gateway<T: Serialize, U: for<'de> Deserialize<'de>>(&self, gateway_url: &str, query: &str, variables: T) -> 
        Result<U, QueryGatewayErrors> {        
        let result = self.client.post(format!("{}{}", gateway_url, "/graphql"))
            .headers(get_content_type_headers())
            .body(
                serde_json::to_string(&GraphqlQuery {
                    query: query.to_string(),
                    variables
                }).unwrap()
            )
            .send()
            .await;

        match result {
            Ok(res) => {
                let body_str = res.text().await.unwrap();
                match serde_json::from_str::<U>(&body_str) {
                    Ok(res) => Ok(res),
                    Err(e) => {                        
                        error!("Serialization error {:?}", e);
                        Err(QueryGatewayErrors::Serialization(Some(Box::new(e))))
                    }
                }
            },
            Err(e) => {
                error!("Error Encountered when querying gateway");
                Err(QueryGatewayErrors::Network(Some(Box::new(e))))
            }
        }
    }

    // pub async fn upload_data_item(&self, uploader_url: &str, data_item: &DataItem) -> Result<String, Error> {
    //     let mut headers = HeaderMap::new();
    //     headers.append("Content-Type", HeaderValue::from_str("application/octet-stream").unwrap());
    //     // headers.append("Accept", HeaderValue::from_str("application/json").unwrap());
    //     let result = self.client
    //         .post(format!("{}/tx/arweave", uploader_url))
    //         .headers(headers)
    //         .body(serde_json::to_string(data_item).unwrap())
    //         .send()
    //         .await;

    //     match result {
    //         Ok(res) => {
    //             let result_text = res.text().await.unwrap();
    //             Ok(result_text)
    //         },
    //         Err(e) => {
    //             error!("Error while communicating with uploader:");
    //             Err(e)
    //         }
    //     }
    // }
}

#[derive(Serialize)]
struct GraphqlQuery<T> {
    query: String,
    variables: T
}

/// variables type
#[derive(Serialize)]
struct ProcessIds {
    #[serde(rename = "processIds")]
    process_ids: Vec<String>
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use async_trait::async_trait;
    use serde_json::to_vec;
    use super::*;
    use crate::{domain::{clients::uploader::UploaderClient, core::{builder::Builder, bytes::DataItem, dal::{Uploader, Gateway, Log, NetworkInfo, ScheduleProvider, Signer, TxStatus}}}, test_utils::{get_uploader_url, get_wallet_file}};

    // #[tokio::test]
    // async fn test_new() {
    //     let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
    //     assert!(!arweave.address().unwrap().is_empty());
    // }

    // #[tokio::test]
    // async fn test_create_wallet_client() {
    //     let wallet = InternalArweave::create_wallet_client(get_wallet_file(), get_uploader_url());
    //     assert!(!wallet.get_wallet_address().unwrap().is_empty());
    // }

    // #[tokio::test]
    // async fn test_address_with() {
    //     let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
    //     assert!(!arweave.address().unwrap().is_empty());
    // }

    // #[tokio::test]
    // async fn test_build_sign_dataitem_with() {        
    //     let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
    //     let mut path = PathBuf::new();
    //     path.push("test.png");
        
    //     let data = std::fs::read(path).unwrap();
    //     let data_item = arweave.build_sign_dataitem(
    //         data.clone(), 
    //         None,
    //         vec![Tag { 
    //             name: Some("test".to_string()), 
    //             value: Some("test value".to_string())
    //         }]
    //     );
    //     // println!("error {:?}", data_item.ok());
    //     assert!(data_item.is_ok());
    // }

    // #[tokio::test]
    // async fn test_upload_dataitem() {
    //     let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());

    //     let mut path = PathBuf::new();
    //     path.push("test.png");
        
    //     let data = std::fs::read(path).unwrap();
    //     let data_item = arweave.build_sign_dataitem(
    //         data.clone(), 
    //         None,
    //         vec![
    //             Tag { 
    //                 name: Some("Bundle-Format".to_string()), 
    //                 value: Some("binary".to_string())
    //             },
    //             Tag { 
    //                 name: Some("Bundle-Version".to_string()), 
    //                 value: Some("2.0.0".to_string()) 
    //             }
    //         ]
    //     );

    //     let result = arweave.upload_data_item(get_uploader_url(), &data_item.unwrap()).await;
    //     println!("result {:?}", result);
    // }

    const ITEM_STR: &str = "AQB9q2yhsQlBHv2LOTIrtmKjw063S1DG0prKcq86DykIegmPnXOReXkWXwpqXt4YxTRw6Rw1jG7f1QFF5ReoJO2MrJmia9ymkTmnhamv3lsYYIotBC6U4Bmzo6IZiKmn2llJt0MDvCe8rxzG15vvff9bpnDIVflY_Dm9Y0dCH-w2Xg8rb2xLq-cM8SBoNRiYruwcwpahiHTjXcxboJKksZRXaI_E7_7vL1gWlMLqeYeF_uXqkth8_PGtZcqMA7pbTYcRzGki_rifGXKUIZKgSIRXTk54iboiqNzOklIFpDKDJpC9Xk_6ppSw_Xzs8S0KpR-veBL8TeURtGhrsDecu_36Pk2MMvdZedxiAg7bvQ9H_NZecoZcju-sQKZiE7haq9Nos3g6njh9IpXivGJ1k8tRLeox7hXOeynffzcXz1Vnz5c4Zxw8LKUbLygni49sflKyFTMnQ8sgDw00fPsuhrznq37-2OLhmYe-tIg-TEV3T4VNdqchzeRSFIv_l7ZJcxeFxcEgdq9aXMx2yzVhSInFuk_W8fJSbhPKX9cewbr4BA_XUNMReowLVcnjB_19iCWnivkVk9sz-QRbjuVL2IMqZePWcRdN5ncXRJoYv4F-Z4FfXDCFuyCD4UAtiQfdch-S4KvRf99DwKrZrMIF28MDdRFdE3ZGDs3FXcPuN8eMLoKBrkyfkM3J89W1GNvrcCNHSNzhF8oPItU4Qno7-x52ZIOAjfdFcXTYLQYU7Xfr6GKaRByemPrkbkrJpdB8RQREt3rQRDNGRQ0jnbPn62PQugvss98JZn9D4ScNusbbgKMihj4MqfXE2mt7Ab9ewx5d01d-Mwf3D6mGz_ERBJgJo8b119bRXdNvgUDJC58NFd4chEOUF4mbyj2pZB9P7fx22yEvV7y6DNzuKvk02YQt7TwL7sdxH1PT63CYJx0tlVGGDvJhGKUQwOfDaXHFMjuuUlXa_klTJT5wEb78aAyh33rw0n9wpOakTIk2KgekbJAzVWCT0BfLrrOhKs3556_d--2mLmcLOONosBjSLokuvtyrTOX7btKRf6Zl5l3wtxsFaPgO6M3Qy9UR46AtK76XSFQd9kcDf_Qj1FyronJS_enQFWYn5Um97mDnYT9SJwMpDFS_FYBTKlsNhsVy11EW5kKuo6mTRlfebJa9CQv-NzbUajd7ulAcM4VNWYt-KbbhVZtUUUxgDvXJdlwRSYR5U8JwSze3sfatb5mbds-EAS-tT7grwrvTb4wRz20e9ARtBg6kC_x8QujHmFORJ97zrFlnnunPbsWgwWz8bfT9RMFy5xUE1KDCtnJqp-M3FoWwQc4sREIyCl7Q6JTq_slPe-Xwt9C5oquj4e_SoOuTAfqDPAmIG6rEXKSN7RP3KRjN5IA5Wpp2I0hgOJ6bT2qNAAUAAAAAAAAASAAAAAAAAAAKGkRhdGEtUHJvdG9jb2wEYW8QZnVuY3Rpb24GcmF3GkRhdGEtUHJvdG9jb2wEYW8OYW8tdHlwZQ5tZXNzYWdlBlNESwRhbwA2NTgz";

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
    async fn test_uploading() {
        let gateway = Arc::new(MockGateway);
        let signer = Arc::new(MockSigner);
        let logger: Arc<dyn Log> = Arc::new(MockLogger);

        let builder = Builder::new(gateway, signer, &logger).unwrap();
        let tx = base64_url::decode(ITEM_STR).expect("failed to encode data item");
        let scheduler = MockScheduler;

        let message = builder.build_message(tx, &scheduler).await.unwrap();
        // println!("message {:?}", message.binary);
        let uploader = UploaderClient::new(&get_uploader_url(), logger).unwrap();
        println!("items {:?}", message.bundle.items);
        let result = uploader.upload(message.binary.to_vec()).await;
    }

    // #[tokio::test]
    // async fn test_uploading_from_dataitem() {
    //     let gateway = Arc::new(MockGateway);
    //     let signer = Arc::new(MockSigner);
    //     let logger: Arc<dyn Log> = Arc::new(MockLogger);

    //     let mut path = PathBuf::new();
    //     path.push("test.png");        
    //     let data = std::fs::read(path).unwrap();

    //     let tags: Vec<Tag> = vec![
    //         Tag { name: "hello".to_string(), value: "world".to_string() }
    //     ];

    //     println!("create builder");
    //     let builder = Builder::new(gateway, signer.clone(), &logger).unwrap();
    //     let mut dataitem = DataItem::new(vec![], data, tags, signer.get_public_key()).unwrap();
    //     let dataitem_vec = dataitem.get_message().unwrap().to_vec();
    //     let signature = signer.sign_tx(dataitem_vec).await.unwrap();
    //     let cloned_dataitem_vec = dataitem_vec.clone();
    //     let dataitem_encoded = base64_url::encode::<[u8]>(&cloned_dataitem_vec);
    //     let tx = base64_url::decode(&dataitem_encoded).unwrap();
    //     let scheduler = MockScheduler;

    //     println!("start build message");
    //     let result = builder.build_message(tx, &scheduler).await.unwrap();
    //     println!("result {:?}", result.binary);
    // }
}