use ao_common::errors::QueryGatewayErrors;
use ao_common::models::gql_models::{Node, TransactionConnectionSchema};
use ao_common::models::shared_models::Tag;
use async_trait::async_trait;
use serde::Serialize;
use ao_common::arweave::InternalArweave;
use crate::err::SchedulerErrors;

const URL_TAG: &str = "Url";
const TTL_TAG: &str = "Time-To-Live";
const SCHEDULER_TAG: &str = "Scheduler";

pub struct Gateway {
    arweave: InternalArweave,
    gateway_url: String
}

#[async_trait]
pub trait GatewayMaker {
    async fn load_process_scheduler<'a>(&self, process_tx_id: &'a str) -> Result<SchedulerResult, SchedulerErrors>;
    async fn load_scheduler<'a>(&self, scheduler_wallet_address: &'a str) -> Result<SchedulerResult, SchedulerErrors>;
}

#[async_trait]
impl GatewayMaker for Gateway {
    async fn load_process_scheduler<'a>(&self, process_tx_id: &'a str) -> Result<SchedulerResult, SchedulerErrors> {
        #[allow(non_snake_case)]
        let GET_TRANSACTIONS_QUERY = r#"
            query GetTransactions ($transactionIds: [ID!]!) {
            transactions(ids: $transactionIds) {
                edges {
                node {
                    tags {
                    name
                    value
                    }
                }
                }
            }
            }
        "#;

        let result = self.gateway_with::<TransactionIds, TransactionConnectionSchema>(
            self.gateway_url.as_str(), 
            GET_TRANSACTIONS_QUERY, 
            TransactionIds { transaction_ids: vec![process_tx_id] }
        ).await;
        match result {
            Ok(tx) => {
                let node: Option<Node> = if tx.data.transactions.edges.is_empty() { None } else { Some(tx.data.transactions.edges[0].node.clone()) };
                let tags = Gateway::find_transaction_tags("Process ${process} was not found on gateway ${GATEWAY_URL}", &node);
                match tags {
                    Ok(tags) => {
                        let tag_val = Gateway::find_tag_value(SCHEDULER_TAG, &tags);
                        if tag_val.is_empty() {
                            let error = SchedulerErrors::new_tag_not_found("No 'Scheduler' tag found on process".to_string());
                            return Err(error);
                        }
                        let load_scheduler = self.load_scheduler(&tag_val).await;
                        match load_scheduler {
                            Ok(res) => Ok(res),
                            Err(e) => Err(e)
                        }
                    },
                    Err(e) => Err(e)
                }
            },
            Err(e) => Err(SchedulerErrors::Network(Some(Box::new(e))))
        }
    }

    async fn load_scheduler<'a>(&self, scheduler_wallet_address: &'a str) -> Result<SchedulerResult, SchedulerErrors> {
        #[allow(non_snake_case)]
        let GET_SCHEDULER_LOCATION = r#"
            query GetSchedulerLocation ($owner: String!) {
            transactions (
                owners: [$owner]
                tags: [
                { name: "Data-Protocol", values: ["ao"] },
                { name: "Type", values: ["Scheduler-Location"] }
                ]
                # Only need the most recent Scheduler-Location
                sort: HEIGHT_DESC
                first: 1
            ) {
                edges {
                node {
                    tags {
                    name
                    value
                    }
                }
                }
            }
            }
        "#;

        let result = self.gateway_with::<WalletAddress, TransactionConnectionSchema>(self.gateway_url.as_str(), GET_SCHEDULER_LOCATION, WalletAddress { owner: scheduler_wallet_address }).await;        
        match result {
            Ok(tx) => {
                let node = if tx.data.transactions.edges.is_empty() { None } else { Some(tx.data.transactions.edges[0].node.clone()) };
                let tags = Gateway::find_transaction_tags(
                    format!("Could not find 'Scheduler-Location' owner by wallet {}", scheduler_wallet_address).as_str(), 
                    &node
                );
                match tags {
                    Ok(tags) => {
                        let url = Gateway::find_tag_value(URL_TAG, &tags);
                        let ttl = Gateway::find_tag_value(TTL_TAG, &tags);

                        if url.is_empty() {
                            let error = SchedulerErrors::new_invalid_scheduler_location("No 'Url' tag found on Scheduler-Location".to_string());
                            return Err(error);
                        }
                        if ttl.is_empty() {
                            let error = SchedulerErrors::new_invalid_scheduler_location("No 'Time-To-Live' tag found on Scheduler-Location".to_string());
                            return Err(error);
                        }
                        return Ok(SchedulerResult {
                            url,
                            ttl: ttl.parse::<u64>().unwrap(),
                            owner: scheduler_wallet_address.to_string()
                        });
                    },
                    Err(e) => {
                        return Err(e);
                    }
                }
            },
            Err(e) => {
                Err(SchedulerErrors::Network(Some(Box::new(e))))
            }
        }
    }
}

impl Gateway {
    /// If you need readonly querying, pass an empty path
    pub fn new(keypair_path: &str, gateway_url: &str, uploader_url: &str) -> Self {
        Gateway { arweave: InternalArweave::new(keypair_path, uploader_url), gateway_url: gateway_url.to_string() }
    }

    fn find_tag_value(name: &str, tags: &Vec<Tag>) -> String {
        match tags.iter().find(|tag| tag.name == name) {
            Some(found_tag) => found_tag.value.to_string(),
            None => "".to_string()
        }
    }
    
    fn find_transaction_tags<'a>(err_msg: &'a str, transaction_node: &'a Option<Node>) -> Result<Vec<Tag>, SchedulerErrors> {
        if let Some(node) = transaction_node {
            return Ok(if node.tags.is_none() { vec![] } else { node.tags.as_ref().unwrap().clone() });
        }
        Err(SchedulerErrors::new_transaction_not_found(err_msg.to_string()))
    }
    
    async fn gateway_with<'a, T: Serialize, U: for<'de> serde::Deserialize<'de>>(&self, gateway_url: &'a str, query: &'a str, variables: T) -> Result<U, QueryGatewayErrors> {
        let result = self.arweave.query_gateway::<T, U>(gateway_url, query, variables).await;
        
        match result {
            Ok(res) => Ok(res),
            Err(e) => Err(e)
        }
    }    
}

#[derive(Serialize)]
struct WalletAddress<'a> {
    owner: &'a str
}

#[derive(Serialize)]
struct TransactionIds<'a> {
    #[serde(rename = "transactionIds")]
    transaction_ids: Vec<&'a str>
}

#[derive(Debug)]
pub struct SchedulerResult {
    pub url: String,
    pub ttl: u64,
    pub owner: String
}

#[cfg(test)]
mod tests {
    use std::sync::Once;
    use ao_common::models::{gql_models::{Amount, MetaData}, shared_models::Owner};
    use dotenv::dotenv;
    use super::*;

    const GATEWAY_URL: &str = "https://arweave.net";
    const UPLOADER_URL: &str = "https://up.arweave.net";
    const SCHEDULER_WALLET_ADDRESS: &str = "_GQ33BkPtZrqxA84vM8Zk-N2aO0toNNu_C-l-rawrBA";
    static INIT: Once = Once::new();

    static JWK_STRING: once_cell::sync::OnceCell<String> = once_cell::sync::OnceCell::new();
    fn get_jwk() -> &'static String {
        JWK_STRING.get_or_init(|| {
            dotenv().ok();

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

    fn init() {
        INIT.call_once(|| env_logger::init_from_env(env_logger::Env::new().default_filter_or("info")));
    }
  
    #[test]
    fn test_find_tag_value() {
        let tags = vec![
            Tag { name: "hello".to_string(), value: "22".to_string() },
            Tag { name: "world".to_string(), value: "tim".to_string() }
        ];

        let value = Gateway::find_tag_value("world", &tags);

        assert!(value != "22".to_string());
        assert!(value == "tim".to_string());
    }

    #[test]
    fn test_find_transaction_tags() {
        let node = Node {
            id: Some("123".to_string()),
            anchor: Some("123".to_string()),
            signature: Some("123".to_string()),
            recipient: Some("123".to_string()),
            owner: Some(Owner {
                address: "123".to_string(),
                key: "123".to_string()
            }),
            fee: Some(Amount {
                winston: "123".to_string(),
                ar: "123".to_string()
            }),
            quantity: Some(Amount {
                winston: "123".to_string(),
                ar: "123".to_string()
            }),
            data: Some(MetaData {
                size: 123,
                content_type: Some("application/json".to_string())
            }),
            tags: Some(vec![
                Tag { name: "hello".to_string(), value: "22".to_string() },
                Tag { name: "world".to_string(), value: "tim".to_string() }
            ]),
            block: None,            
            parent: None,
            bundled_in: None
        };

        let result = Gateway::find_transaction_tags("hello world", &Some(node));

        assert!(result.unwrap()[0].name == "hello".to_string());
    }

    #[tokio::test]
    async fn test_load_scheduler_with() {
        init();

        let gateway = Gateway::new(get_wallet_file(), GATEWAY_URL, UPLOADER_URL);
        
        let result = gateway.load_scheduler(SCHEDULER_WALLET_ADDRESS).await;
        match result {
            Ok(scheduler) => assert!(scheduler.owner == SCHEDULER_WALLET_ADDRESS.to_string()),
            Err(e) => panic!("{:?}", e)
        };
    }

    #[tokio::test]
    async fn test_load_process_scheduler_with() {
        init();

        let gateway = Gateway::new(get_wallet_file(), GATEWAY_URL, UPLOADER_URL);
        
        let result = gateway.load_process_scheduler("KHruEP5dOP_MgNHava2kEPleihEc915GlRRr3rQ5Jz4").await;
        match result {
            Ok(scheduler) => {
                assert!(scheduler.owner == SCHEDULER_WALLET_ADDRESS.to_string());
            },
            Err(e) => panic!("{:?}", e)
        };
    }
}