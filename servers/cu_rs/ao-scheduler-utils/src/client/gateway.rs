use ao_common::errors::QueryGatewayErrors;
use ao_common::models::gql_models::{GraphqlInput, TransactionConnectionSchema};
use ao_common::models::shared_models::Tag;
use ao_common::network::utils::get_content_type_headers;
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use crate::dal::{LoadProcessSchedulerSchema, LoadSchedulerSchema, Scheduler};
use crate::err::SchedulerErrors;
use log::error;

const URL_TAG: &str = "Url";
const TTL_TAG: &str = "Time-To-Live";
const SCHEDULER_TAG: &str = "Scheduler";

pub struct Gateway {
    client: Client,
    gateway_url: String
}

impl Gateway {
    /// If you need readonly querying, pass an empty path
    pub fn new(gateway_url: &str) -> Self {
        Gateway { client: Client::new(), gateway_url: gateway_url.to_string() }
    }

    fn find_tag_value(name: &str, tags: &Vec<Tag>) -> String {
        match tags.iter().find(|tag| tag.name == name) {
            Some(found_tag) => found_tag.value.to_string(),
            None => "".to_string()
        }
    }
    
    fn find_transaction_tags<'a>(err_msg: &'a str, transaction_node: &'a Option<ao_common::models::gql_models::Node>) -> Result<Vec<Tag>, SchedulerErrors> {
        if let Some(node) = transaction_node {
            return Ok(if node.tags.is_none() { vec![] } else { node.tags.as_ref().unwrap().clone() });
        }
        Err(SchedulerErrors::new_transaction_not_found(err_msg.to_string()))
    }
    
    async fn gateway_with<'a, T: Serialize, U: for<'de> serde::Deserialize<'de>>(&self, query: &'a str, variables: T) -> Result<U, QueryGatewayErrors> {
        let result = self.client.post(self.gateway_url.clone())
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
}

#[async_trait]
impl LoadProcessSchedulerSchema for Gateway {
    async fn load_process_scheduler(&self, process_tx_id: &str) -> Result<Scheduler, SchedulerErrors> {
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
            GET_TRANSACTIONS_QUERY, 
            TransactionIds { transaction_ids: vec![process_tx_id] }
        ).await;
        match result {
            Ok(tx) => {
                let node: Option<ao_common::models::gql_models::Node> = if tx.data.transactions.edges.is_empty() { None } else { Some(tx.data.transactions.edges[0].node.clone()) };
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
}

#[async_trait]
impl LoadSchedulerSchema for Gateway {
    async fn load_scheduler(&self, scheduler_wallet_address: &str) -> Result<Scheduler, SchedulerErrors> {
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

        let result = self.gateway_with::<WalletAddress, TransactionConnectionSchema>(GET_SCHEDULER_LOCATION, WalletAddress { owner: scheduler_wallet_address }).await;        
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
                        return Ok(Scheduler {
                            url,
                            ttl: ttl.parse::<u64>().unwrap(),
                            address: scheduler_wallet_address.to_string()
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


#[derive(Serialize)]
struct WalletAddress<'a> {
    owner: &'a str
}

#[derive(Serialize)]
struct TransactionIds<'a> {
    #[serde(rename = "transactionIds")]
    transaction_ids: Vec<&'a str>
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    const _GRAPHQL_URL: &str = "https://arweave.net/graphql";
    const PROCESS: &str = "zc24Wpv_i6NNCEdxeKt7dcNrqL5w0hrShtSCcFGGL24";
    const SCHEDULER: &str = "gnVg6A6S8lfB10P38V7vOia52lEhTX3Uol8kbTGUT8w";
    const TWO_DAYS: u64 = 1000 * 60 * 60 * 48;
    
    mod gateway {
        use super::*;

        #[derive(Serialize)]
        struct Tag {
            name: String,
            value: String,
        }
        #[derive(Serialize)]
        struct Tags {
            tags: Vec<Tag>
        }
        #[derive(Serialize)]
        struct Node {
            node: Tags
        }
        #[derive(Serialize)]
        struct Edges {
            edges: Vec<Node>
        }
        #[derive(Serialize)]
        struct Transactions {
            transactions: Edges
        }
        #[derive(Serialize)]
        struct Data {
            data: Transactions
        }

        mod load_process_scheduler {
            use super::*;            

            #[tokio::test]
            async fn test_load_the_scheduler_location_for_the_process() {
                let mut server = Server::new_async().await;
                let url = server.url();

                let _mock = server.mock("POST", "/")
                    .with_status(200)
                    .with_body(
                        serde_json::to_string(&Data {
                            data: Transactions {
                                transactions: Edges {
                                    edges: vec![
                                        Node {
                                            node: Tags {
                                                tags: vec![
                                                    Tag { 
                                                        name: "Scheduler".to_string(), 
                                                        value: SCHEDULER.to_string() 
                                                    }
                                                ]
                                            }
                                        }
                                    ]
                                }
                            }
                        }).unwrap()
                    )
                    .create();

                let gateway = Gateway::new(&url);                
                match gateway.load_process_scheduler(PROCESS).await {
                    Ok(res) => {
                        assert!(res.url == url);
                        assert!(res.ttl == TWO_DAYS);
                        assert!(res.address == SCHEDULER);
                    },
                    Err(e) => panic!("load_the_scheduler_location_for_the_process {:?}", e)
                }
            }

            #[tokio::test]
            async fn test_throws_if_no_scheduler_tag_is_found_on_process() {
                let mut server = Server::new_async().await;
                let url = server.url();

                let _mock = server.mock("POST", "/")
                    .with_status(200)
                    .with_body(
                        serde_json::to_string(&Data {
                            data: Transactions {
                                transactions: Edges {
                                    edges: vec![
                                        Node {
                                            node: Tags {
                                                tags: vec![
                                                    Tag { 
                                                        name: "Not-Scheduler".to_string(), 
                                                        value: SCHEDULER.to_string() 
                                                    }
                                                ]
                                            }
                                        }
                                    ]
                                }
                            }
                        }).unwrap()
                    )
                    .create();

                let gateway = Gateway::new(&url);                
                match gateway.load_process_scheduler(PROCESS).await {
                    Ok(_res) => panic!("throws_if_no_scheduler_tag_is_found_on_process should not succeed"),
                    Err(_e) => ()
                }
            }
        }

        mod load_schedule {
            use super::*;

            #[tokio::test]
            async fn test_load_the_scheduler_location_for_the_wallet_address() {
                let mut server = Server::new_async().await;
                let url = server.url();

                let _mock = server.mock("POST", "/")
                    .with_status(200)
                    .with_body(
                        serde_json::to_string(&Data {
                            data: Transactions {
                                transactions: Edges {
                                    edges: vec![
                                        Node {
                                            node: Tags {
                                                tags: vec![
                                                    Tag { 
                                                        name: "Url".to_string(), 
                                                        value: url.clone()
                                                    },
                                                    Tag {
                                                        name: "Time-To-Live".to_string(),
                                                        value: format!("{}", TWO_DAYS)
                                                    }
                                                ]
                                            }
                                        }
                                    ]
                                }
                            }
                        }).unwrap()
                    )
                    .create();

                let gateway = Gateway::new(&url);
                match gateway.load_scheduler(SCHEDULER).await {
                    Ok(res) => {
                        assert!(res.url == url);
                        assert!(res.ttl == TWO_DAYS);
                        assert!(res.address == SCHEDULER);
                    },
                    Err(e) => panic!("load_the_scheduler_location_for_the_wallet_address {:?}", e)
                }
            }

            #[tokio::test]
            async fn test_throws_if_no_url_tag_is_found_on_scheduler_location_record() {
                let mut server = Server::new_async().await;
                let url = server.url();

                let _mock = server.mock("POST", "/")
                    .with_status(200)
                    .with_body(
                        serde_json::to_string(&Data {
                            data: Transactions {
                                transactions: Edges {
                                    edges: vec![
                                        Node {
                                            node: Tags {
                                                tags: vec![
                                                    Tag { 
                                                        name: "Not-Url".to_string(), 
                                                        value: url.clone()
                                                    },
                                                    Tag {
                                                        name: "Time-To-Live".to_string(),
                                                        value: format!("{}", TWO_DAYS)
                                                    }
                                                ]
                                            }
                                        }
                                    ]
                                }
                            }
                        }).unwrap()
                    )
                    .create();

                let gateway = Gateway::new(&url);
                match gateway.load_scheduler(SCHEDULER).await {
                    Ok(_res) => panic!("throws_if_no_url_tag_is_found_on_scheduler_location_record failed"),
                    Err(_e) => () // todo: update error objects so check for string "No "Url" tag found on Scheduler-Location"
                }
            }

            #[tokio::test]
            async fn test_throws_if_no_time_to_live_tag_is_found_on_scheduler_location_record() {
                let mut server = Server::new_async().await;
                let url = server.url();

                let _mock = server.mock("POST", "/")
                    .with_status(200)
                    .with_body(
                        serde_json::to_string(&Data {
                            data: Transactions {
                                transactions: Edges {
                                    edges: vec![
                                        Node {
                                            node: Tags {
                                                tags: vec![
                                                    Tag { 
                                                        name: "Url".to_string(), 
                                                        value: url.clone()
                                                    },
                                                    Tag {
                                                        name: "Not-Time-To-Live".to_string(),
                                                        value: format!("{}", TWO_DAYS)
                                                    }
                                                ]
                                            }
                                        }
                                    ]
                                }
                            }
                        }).unwrap()
                    )
                    .create();

                let gateway = Gateway::new(&url);
                match gateway.load_scheduler(SCHEDULER).await {
                    Ok(_res) => panic!("throws_if_no_time_to_live_tag_is_found_on_scheduler_location_record failed"),
                    Err(_e) => () // todo: update error objects so check for string "No "Time-To-Live" tag found on Scheduler-Location"
                }
            }
        }
    }
}