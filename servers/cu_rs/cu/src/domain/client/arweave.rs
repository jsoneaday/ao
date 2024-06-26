use arweave_rs::Arweave;
use arweave_rs::ArweaveBuilder;
use bundlr_sdk::tags::Tag;
use rand::Rng;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{Client, Error, Response, Url};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
#[allow(unused)]
use log::{error, info};
use ao_common::domain::bytes::DataItem;
use ao_common::errors::QueryGatewayErrors;
use ao_common::models::gql_models::{Node, TransactionConnectionSchema, GraphqlInput};
use ao_common::network::utils::get_content_type_headers;
use ao_common::domain::uploader::IrysResponse;

pub struct InternalArweave {
    pub internal_arweave: Arweave,
    client: Client
}

impl InternalArweave {
    pub fn new(keypair_path: &str, uploader_url: &str) -> Self {
        InternalArweave {
            internal_arweave: InternalArweave::create_wallet_client(keypair_path, uploader_url),
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
    /// This function signs the data item
    pub fn build_sign_dataitem(&self, data: Vec<u8>, tags: Vec<Tag>) -> Result<DataItem, QueryGatewayErrors> {
        let mut data_item = DataItem::new(
            vec![], 
            data, 
            tags, 
            base64_url::decode(&self.internal_arweave.get_pub_key().unwrap()).unwrap()
        ).unwrap();

        data_item.signature = self.internal_arweave.sign(&data_item.get_message().unwrap().to_vec()).unwrap();

        Ok(data_item)
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
    pub async fn load_tx_meta(self, graphql_url: &str, id: &str) -> Result<Node, QueryGatewayErrors> {
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

        let result = self.query_gateway::<ProcessIds, TransactionConnectionSchema>(graphql_url, GET_PROCESSES_QUERY, ProcessIds {
            process_ids: vec![id.to_string()]
        }).await;

        match result {
            Ok(tx) => {
                Ok(tx.data.transactions.edges[0].node.clone())
            },
            Err(e) => {
                error!("Error Encountered when fetching transaction {} from gateway {}", id, graphql_url);
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
    pub async fn load_tx_data(&self, arweave_url: &str, id: &str) -> Result<Response, Error> {
        let result = self.client.get(format!("{}/raw/{}", arweave_url, id)).send().await;
        match result {
            Ok(res) => Ok(res),
            Err(e) => {
                error!("Error Encountered when fetching raw data for transaction {} from gateway {}", id, arweave_url);
                Err(e)
            }
        }
    }

    pub async fn query_gateway<T: Serialize, U: for<'de> Deserialize<'de>>(&self, graphql_url: &str, query: &str, variables: T) -> 
        Result<U, QueryGatewayErrors> {        
        let result = self.client.post(graphql_url)
            .headers(get_content_type_headers())
            .body(
                serde_json::to_string(&GraphqlInput {
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

    pub async fn upload_data_item(&self, uploader_url: &str, data_item: DataItem) -> Result<IrysResponse, Error> {
        let mut headers = HeaderMap::new();
        headers.append("Content-Type", HeaderValue::from_str("application/octet-stream").unwrap());
        
        let result = self.client
            .post(format!("{}/tx/arweave", uploader_url))
            .headers(headers)
            .body(data_item.as_bytes().unwrap())
            .send()
            .await;

        match result {
            Ok(res) => {
                let result = res.json::<IrysResponse>().await.unwrap();
                Ok(result)
            },
            Err(e) => {
                error!("Error while communicating with uploader:");
                Err(e)
            }
        }
    }
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
    use ao_common::test_utils::{get_uploader_url, get_wallet_file};
    
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
    async fn test_random_anchor_created_without_panic() {
        InternalArweave::create_random_anchor();
    }
}