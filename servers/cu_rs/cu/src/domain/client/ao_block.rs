use ao_common::{models::gql_models::{GraphqlInput, PageInfo}, network::utils::get_content_type_headers};
use async_recursion::async_recursion;
use serde::Serialize;
use sqlx::{prelude::FromRow, sqlite::SqliteQueryResult, Sqlite};
use crate::domain::{maths::increment, model::model::BlockSchema, utils::error::CuErrors};
use super::sqlite::{ConnGetter, SqliteClient, BLOCKS_TABLE};

#[allow(unused)]
#[derive(FromRow)]
pub struct BlockDocSchema {
    /// id is actually the height value of BlockSchema
    pub id: i64,
    pub height: i64,
    pub timestamp: i64
}

#[allow(unused)]
pub struct Query {
    sql: String,
    parameters: Vec<Vec<i64>>
}

#[derive(Serialize)]
struct GqlQueryVariables {
    /// height
    min: i64,
    /// page_size
    limit: i64
}

mod gql_return_types {
    use ao_common::models::gql_models::PageInfo;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    pub struct DataBlocks {
        pub data: Blocks
    }

    #[derive(Deserialize, Debug)]
    pub struct Blocks {
        pub blocks: BlocksTransactions
    }

    #[derive(Deserialize, Debug)]
    pub struct BlocksTransactions {
        #[serde(rename = "pageInfo")]
        pub page_info: Option<PageInfo>,
        pub edges: Vec<Edge>
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct Edge {
        pub node: Node
    }

    #[derive(Deserialize, Debug, Clone)]
    pub struct Node {
        pub timestamp: i64,
        pub height: i64
    }
}

#[allow(unused)]
const GET_BLOCKS_QUERY: &str = r"
    query GetBlocks($min: Int!, $limit: Int!) {
        blocks (
            height: { min: $min },
            first: $limit,
            sort: HEIGHT_ASC
        ) {
            pageInfo {
                hasNextPage
            }
            edges {
                node {
                    timestamp
                    height
                }
            }
        }
    }";

#[allow(unused)]
pub struct AoBlock<'a> {
    sql_client: &'a SqliteClient,
    client: reqwest::Client
}

impl<'a> AoBlock<'a> {
    #[allow(unused)]
    // todo: if SqliteClient isn't always needed make optional
    pub fn new(client: &'a SqliteClient) -> Self {
        AoBlock { 
            sql_client: client,
            client: reqwest::Client::new()
        }
    }

    #[allow(unused)]
    fn create_insert_blocks_query(blocks: &Vec<BlockDocSchema>) -> Query {
        let mut blocks_placeholder = "".to_string();
        let mut last_param = 0;
        for i in 0..blocks.len() {
            let first = increment(last_param);
            let second = first + 1;
            let third = second + 1;

            if i == 0 {                
                blocks_placeholder = format!("(${}, ${}, ${}),\n", first, second, third);
            } else if i != blocks.len() - 1 {
                blocks_placeholder = format!("{}(${}, ${}, ${}),\n", blocks_placeholder.clone(), first, second, third);
            } else {
                blocks_placeholder = format!("{}(${}, ${}, ${})", blocks_placeholder.clone(), first, second, third);
            }

            last_param = third;
        }
        
        Query {
            sql: format!(r"
                INSERT OR IGNORE INTO {}
                (id, height, timestamp)
                VALUES
                {}
            ", BLOCKS_TABLE, blocks_placeholder),
            parameters: blocks.iter().map(|block| {
                let result: Vec<i64> = vec![block.id, block.height, block.timestamp];
                result
            }).collect()
        }
    }

    #[allow(unused)]
    pub async fn save_block(&self, blocks: &Vec<BlockSchema>) -> Result<Option<SqliteQueryResult>, sqlx::error::Error> {
        if blocks.len() == 0 { return Ok(None) }

        let block_docs = blocks.iter().map(|block| BlockDocSchema { id: block.height, height: block.height, timestamp: block.timestamp }).collect::<Vec<BlockDocSchema>>();
        let query = AoBlock::create_insert_blocks_query(&block_docs);
        let mut raw_query = sqlx::query::<Sqlite>(&query.sql);
        for params in query.parameters.iter() {
            for param in params {
                let _raw_query = raw_query
                    .bind(param);
                raw_query = _raw_query;
            }            
        }
        
        match raw_query
            .execute(self.sql_client.get_conn())
            .await {
                Ok(res) => Ok(Some(res)),
                Err(e) => Err(e)
        }
    }

    fn create_select_blocks_query(min_height: i64, max_timestamp: i64) -> Query {
        Query {
            sql: format!(r"
                SELECT height, timestamp
                FROM {}
                WHERE
                    height >= ?
                AND timestamp <= ?
                ORDER BY height ASC
            ", BLOCKS_TABLE),
            parameters: vec![
                vec![
                    min_height,
                    max_timestamp
                ]
            ]
        }
    }

    pub async fn find_blocks(&self, min_height: i64, max_timestamp: i64) -> Result<Vec<BlockSchema>, sqlx::error::Error> {
        let query = AoBlock::create_select_blocks_query(min_height, max_timestamp);
        let mut raw_query = sqlx::query_as::<_, BlockSchema>(&query.sql);
        for params in query.parameters.iter() {
            for param in params {
                let _raw_query = raw_query
                    .bind(param);
                raw_query = _raw_query;
            }            
        }
        
        match raw_query
            .fetch_all(self.sql_client.get_conn())
            .await {
                Ok(res) => Ok(res),
                Err(e) => Err(e)
        }
    }

    #[allow(unused)]
    async fn fetch_page(&self, min_height: i64, max_timestamp: i64, page_size: i64, graphql_url: &str) -> Result<gql_return_types::DataBlocks, CuErrors> {
        self.sql_client.logger.log(format!("Loading page of up to {} blocks after height {} up to timestamp {}", page_size, min_height, max_timestamp));

        let result = self.client.post(graphql_url)
            .headers(get_content_type_headers())
            .body(
                serde_json::to_string(&GraphqlInput {
                    query: GET_BLOCKS_QUERY.to_string(),
                    variables: GqlQueryVariables {
                        min: min_height,
                        limit: page_size
                    }
                }).unwrap()
            )
            .send()
            .await;

        match result {
            Ok(res) => {
                let body_str = res.text().await.unwrap();
                match serde_json::from_str::<gql_return_types::DataBlocks>(&body_str) {
                    Ok(res) => {
                        Ok(res)
                    },
                    Err(e) => {                        
                        self.sql_client.logger.error(e.to_string());
                        Err(CuErrors::BlockMeta(Some(Box::new(e))))
                    }
                }
            },
            Err(e) => {
                self.sql_client.logger.error(
                    format!("Error Encountered when fetching page of block metadata from gateway with minBlock '{}' and maxTimestamp '{}'", min_height, max_timestamp).to_string()
                );
                Err(CuErrors::BlockMeta(Some(Box::new(e))))
            }
        }
    }   

    #[async_recursion]
    async fn may_fetch_next(&self, graphql_url: &str, page_size: i64, page_info: PageInfo, edges: &Vec<gql_return_types::Edge>, max_timestamp: i64) -> Result<gql_return_types::BlocksTransactions, CuErrors> {
        // /**
        //  * HACK to incrementally fetch the correct range of blocks with only
        //  * a timestamp as the right most limit.
        //  *
        //  * (we no longer have a sortKey to extract a block height from)
        //  *
        //  * If the last block has a timestamp greater than the maxTimestamp
        //  * then we're done.
        //  *
        //  * We then slice off the results in the page, not within our range.
        //  * So we overfetch a little on the final page, but at MOST pageSize - 1
        //  */
      let mut surpassed_max_timestamp_idx: i64 = 0;
      for i in 0..edges.len() {
        if edges[i].node.timestamp > max_timestamp {
            surpassed_max_timestamp_idx = 1;
            break;
        }        
      };

      if surpassed_max_timestamp_idx != -1 {
        return Ok(gql_return_types::BlocksTransactions {
            page_info: Some(page_info),
            edges: edges[0..surpassed_max_timestamp_idx as usize].to_vec()
        });
      }
      if page_info.has_next_page {
        return Ok(gql_return_types::BlocksTransactions {
            page_info: Some(page_info.clone()),
            edges: edges.to_vec()
        });
      }

      match self.fetch_page(edges.iter().last().unwrap().node.height, max_timestamp, page_size, graphql_url).await {
        Ok(res) => {
            match self.may_fetch_next(graphql_url, page_size, page_info.clone(), &res.data.blocks.edges, max_timestamp).await {
                Ok(mut res) => {
                    let mut cloned_edges = edges.to_vec();
                    cloned_edges.append(&mut res.edges);
                    return Ok(gql_return_types::BlocksTransactions {
                        page_info: Some(page_info),
                        edges: cloned_edges
                    });
                },
                Err(e) => Err(e)
            }            
        },
        Err(e) => Err(e)
      }
    }

    #[allow(unused)]
    async fn fetch_all_pages(&self, min_height: i64, max_timestamp: i64, page_size: i64, graphql_url: &str) -> Result<gql_return_types::BlocksTransactions, CuErrors> {
        let _max_timestamp = f64::floor(max_timestamp as f64 / 1000.0);

        match self.fetch_page(min_height, _max_timestamp as i64, page_size, graphql_url).await {
            Ok(res) => {
                match self.may_fetch_next(graphql_url, page_size, res.data.blocks.page_info.unwrap(), &res.data.blocks.edges, max_timestamp).await {
                    Ok(res) => Ok(res),
                    Err(e) => Err(e)
                }
            },
            Err(e) => Err(e)
        }
    }

    pub async fn load_blocks_meta(&self, min_height: i64, max_timestamp: i64, graphql_url: &str, page_size: i64) -> Result<Vec<gql_return_types::Node>, CuErrors> {
        match self.fetch_all_pages(min_height, max_timestamp, page_size, graphql_url).await {
            Ok(res) => {
                Ok(res.edges.iter().map(|edge| gql_return_types::Node {
                    height: edge.node.height,
                    timestamp: edge.node.timestamp * 1000
                }).collect())
            },
            Err(e) => Err(e)
        }
    }
}
