use arrs::wallet::ArWallet;
use arweave_rs::crypto::base64::Base64;
use arweave_rs::transaction::tags::Tag as DataItemTag;
use arweave_rs::ArweaveBuilder;
use futures::{Future, FutureExt};
use reqwest::{Client, Error, Response, Url};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::pin::Pin;
#[allow(unused)]
use log::{error, info};
use crate::errors::QueryGatewayErrors;
use crate::network::utils::get_content_type_headers;
use crate::models::gql_models::{Node, TransactionConnectionSchema};
use crate::models::shared_models::Tag;
use std::fs::read_to_string;

pub struct InternalArweave {
    internal_arweave: ArWallet,
    uploader_url: String,
    client: Client
}

impl InternalArweave {
    /// jwk_string: is the contents of the wallet private key json file
    pub fn new(keypair_path: &str, uploader_url: &str) -> Self {
        InternalArweave {
            internal_arweave: InternalArweave::create_wallet_client(keypair_path),
            uploader_url: uploader_url.to_string(),
            client: Client::new()
        }
    }

    pub fn create_wallet_client(keypair_path: &str) -> ArWallet {
        let mut path = PathBuf::new();
        path.push(keypair_path);
        let jwk_string = read_to_string(path).unwrap();

        ArWallet::from_jwk(jwk_string.as_str())        
        
        // arweave_builder
        //     .keypair_path(path)
        //     .base_url(Url::parse(gateway_url).unwrap())
        //     .build().unwrap()
        // arweave.upload_file_from_path(file_path, additional_tags, fee);
    }

    pub fn address_with<'a>(&'a self) -> Box<dyn Fn() -> String + 'a> {
        Box::new(|| self.internal_arweave.address())
    }

    /// todo: need to go through code path to understand if this call is actually necessary and can be replaced, 
    /// since arweave-rs doesn't have a distinct DataItem object
    pub fn build_sign_dataitem_with(&self) -> impl FnOnce(Vec<u8>, Vec<Tag>, String) -> Pin<Box<dyn Future<Output = DataItem>>> {
        // let mut path = PathBuf::new();
        // path.push("");
        // let arweave_builder = ArweaveBuilder::new();
        // let arweave_uploader = arweave_builder
        //      .keypair_path(path)
        //      .base_url(Url::parse(self.uploader_url.as_str()).unwrap())
        //      .build().unwrap();

        // let tx = arweave_uploader.create_transaction(target, other_tags, data, quantity, fee, auto_content_tag).await.unwrap();
        
        move |data: Vec<u8>, tags: Vec<Tag>, anchor: String| {
            async move {
                DataItem {
                    id: "".to_string(),
                    data,
                    tags,
                    anchor   
                }
            }.boxed()
        }
    }

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
    pub async fn load_tx_meta_with(self, gateway_url: &str, id: &str) -> Result<Node, QueryGatewayErrors> {
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

        let result = self.query_gateway_with::<ProcessIds, TransactionConnectionSchema>(gateway_url, GET_PROCESSES_QUERY, ProcessIds {
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
    pub async fn load_tx_data_with(&self, gateway_url: &str, id: &str) -> Result<Response, Error> {
        let result = self.client.get(format!("{}/raw/{}", gateway_url, id)).send().await;
        match result {
            Ok(res) => Ok(res),
            Err(e) => {
                error!("Error Encountered when fetching raw data for transaction {} from gateway {}", id, gateway_url);
                Err(e)
            }
        }
    }

    pub async fn query_gateway_with<T: Serialize, U: for<'de> Deserialize<'de>>(&self, gateway_url: &str, query: &str, variables: T) -> 
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

    pub async fn upload_data_item_with<U: for<'de> Deserialize<'de>>(&self, gateway_url: String, data_item: &DataItem) -> Result<U, Error> {
        let result = self.client
            .post(format!("{}/tx/arweave", gateway_url))
            .headers(get_content_type_headers())
            .body(serde_json::to_string(data_item).unwrap())
            .send()
            .await;

        match result {
            Ok(res) => Ok(res.json::<U>().await.unwrap()),
            Err(e) => {
                error!("Error while communicating with uploader:");
                Err(e)
            }
        }
    }
}

#[derive(Serialize)]
pub struct DataItem {
    pub id: String,
    pub data: Vec<u8>,
    pub tags: Vec<Tag>,
    pub anchor: String  
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
    use super::*;

    static JWK_STRING: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();
    fn get_jwk() -> &'static String {
        JWK_STRING.get_or_init(|| {
            dotenv::dotenv().ok();

            std::env::var("WALLET").unwrap()
        })
    }

    static WALLET_FILE: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();
    fn get_wallet_file() -> &'static String {
        WALLET_FILE.get_or_init(|| {
            dotenv::dotenv().ok();

            std::env::var("WALLET_FILE").unwrap()
        })
    }

    static UPLOADER_URL: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();
    fn get_uploader_url() -> &'static String {
        UPLOADER_URL.get_or_init(|| {
            dotenv::dotenv().ok();

            std::env::var("UPLOADER_URL").unwrap()
        })
    }

    #[tokio::test]
    async fn test_new() {
        let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
        let balance = arweave.internal_arweave.balance().await.unwrap().parse::<i64>().unwrap();
        assert!(balance >= 0);
    }

    #[tokio::test]
    async fn test_create_wallet_client() {
        let wallet = InternalArweave::create_wallet_client(get_jwk());
        assert!(!wallet.address().is_empty());
    }

    #[tokio::test]
    async fn test_address_with() {
        let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
        assert!(!arweave.address_with()().is_empty());
    }
}