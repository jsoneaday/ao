use serde::{Deserialize, Serialize};
use super::shared_models::Tag;

#[derive(Serialize)]
pub struct GraphqlInput<T> {
    pub query: String,
    pub variables: T
}

#[derive(Deserialize, Debug)]
pub struct TransactionConnectionSchema {
    pub data: Transactions
}

#[derive(Deserialize, Debug)]
pub struct Transactions {
    #[serde(rename = "pageInfo")]
    pub page_info: Option<PageInfo>,
    pub transactions: Edges
}

#[derive(Deserialize, Debug)]
pub struct Edges {
    pub edges: Vec<NodeParent>
}

#[derive(Deserialize, Debug)]
pub struct NodeParent {
    pub node: Node
}

#[derive(Deserialize, Clone, Debug)]
pub struct Owner {
    pub address: String
}

/// Regardless of the schema def, graphql queries can return any combination of fields 
#[derive(Deserialize, Clone, Debug)]
pub struct Node {
    pub id: Option<String>,
    pub anchor: Option<String>,
    pub signature: Option<String>,
    pub recipient: Option<String>,
    pub owner: Option<Owner>,
    pub fee: Option<Amount>,
    pub quantity: Option<Amount>,
    pub data: Option<MetaData>,
    pub tags: Option<Vec<Tag>>,
    pub block: Option<Block>,
    /// parent is deprecated
    pub parent: Option<Parent>,
    /// bundled_in replaces deprecated parent field
    pub bundled_in: Option<Bundle>
}

#[allow(unused)]
#[derive(Deserialize, Clone, Debug)]
pub struct Amount {
    pub winston: String,
    pub ar: String
}

#[allow(unused)]
#[derive(Deserialize, Clone, Debug)]
pub struct MetaData {
    pub size: i64,
    #[serde(rename = "type")]
    pub content_type: Option<String>
}

#[allow(unused)]
#[derive(Deserialize, Clone, Debug)]
pub struct Block {
    id: String,
    timestamp: i64,
    height: i64,
    /// previous means previous id
    previous: String
}

/// Deprecate
#[allow(unused)]
#[derive(Deserialize, Clone, Debug)]
pub struct Parent {
    id: String
}

#[allow(unused)]
#[derive(Deserialize, Clone, Debug)]
pub struct Bundle {
    id: String
}

#[allow(unused)]
#[derive(Deserialize, Clone, Debug)]
pub struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool
}