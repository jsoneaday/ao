use ar_bundles::bundle_item::BundleItemFn;
use ar_bundles::interface_jwk::JWKInterface;
use ar_bundles::signing::{signer::SignerMaker, chains::arweave_signer::ArweaveSigner};
use ar_bundles::tags::Tag;
use ar_bundles::data_item::DataItem;
use arweave_rs::Arweave;
use arweave_rs::ArweaveBuilder;
use ar_bundles::ar_data_create::{create_data, Data, DataItemCreateOptions};
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
    pub fn build_sign_dataitem(&self, data: Data, anchor: Option<[u8;32]>, tags: Vec<Tag>) -> Result<DataItem, QueryGatewayErrors> {
        let jwk_str = std::fs::read_to_string(self.wallet_path.as_str()).unwrap();
        let jwk = serde_json::from_str::<JWKInterface>(&jwk_str).unwrap();
        let signer = ArweaveSigner::new(jwk, &self.wallet_path);
        
        match create_data(data, &signer, Some(&DataItemCreateOptions {
            target: None,
            anchor,
            tags: Some(tags)
        })) {
            Ok(mut di) => {
                di.sign(&signer);
                Ok(di)
            },
            Err(e) => Err(QueryGatewayErrors::BundlerFailure(e))
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

    pub async fn upload_data_item(&self, uploader_url: &str, data_item: &DataItem) -> Result<String, Error> {
        let mut headers = HeaderMap::new();
        headers.append("Content-Type", HeaderValue::from_str("application/octet-stream").unwrap());
        headers.append("Accept", HeaderValue::from_str("application/json").unwrap());
        let result = self.client
            .post(format!("{}/tx/arweave", uploader_url))
            .headers(headers)
            .body(serde_json::to_string(data_item).unwrap())
            .send()
            .await;

        match result {
            Ok(res) => {
                let result_text = res.text().await.unwrap();
                Ok(result_text)
            },
            Err(e) => {
                error!("Error while communicating with uploader:");
                Err(e)
            }
        }
    }
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
    use crate::test_utils::{get_gateway_url, get_uploader_url, get_wallet_file};

    #[tokio::test]
    async fn test_new() {
        let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
        assert!(!arweave.address().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_create_wallet_client() {
        let wallet = InternalArweave::create_wallet_client(get_wallet_file(), get_uploader_url());
        assert!(!wallet.get_wallet_address().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_address_with() {
        let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
        assert!(!arweave.address().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_build_sign_dataitem_with() {        
        let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
        let mut path = PathBuf::new();
        path.push("test.png");
        
        let data = std::fs::read(path).unwrap();
        let data_item = arweave.build_sign_dataitem(
            Data::BinaryData(data.clone()), 
            None,
            vec![Tag { 
                name: Some("test".to_string()), 
                value: Some("test value".to_string()) 
            }]
        );
        // println!("error {:?}", data_item.ok());
        assert!(data_item.is_ok());
    }

    #[tokio::test]
    async fn test_upload_dataitem() {
        let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());

        let mut path = PathBuf::new();
        path.push("test.png");
        
        let data = std::fs::read(path).unwrap();
        let data_item = arweave.build_sign_dataitem(
            Data::BinaryData(data.clone()), 
            None,
            vec![
                Tag { 
                    name: Some("Bundle-Format".to_string()), 
                    value: Some("binary".to_string()) 
                },
                Tag { 
                    name: Some("Bundle-Version".to_string()), 
                    value: Some("2.0.0".to_string()) 
                }
            ]
        );

        let result = arweave.upload_data_item(get_uploader_url(), &data_item.unwrap()).await;
        println!("result {:?}", result);
    }
}