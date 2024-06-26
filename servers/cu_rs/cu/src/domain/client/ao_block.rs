use std::sync::Arc;
use async_trait::async_trait;
use ao_common::{models::gql_models::{GraphqlInput, PageInfo}, network::utils::get_content_type_headers};
use async_recursion::async_recursion;
use serde::Serialize;
use sqlx::{FromRow, Sqlite};
use crate::domain::{dal::{FindBlocksSchema, LoadBlocksMetaSchema, SaveBlocksSchema}, maths::increment, model::model::{gql_return_types, BlockSchema}, utils::error::CuErrors};
use super::sqlite::{ConnGetter, SqliteClient, BLOCKS_TABLE};

#[allow(unused)]
pub struct Query {
    pub sql: String,
    pub parameters: Vec<Vec<i64>>
}

#[allow(unused)]
#[derive(FromRow)]
pub struct BlockDocSchema {
    /// id is actually the height value of BlockSchema
    pub id: i64,
    pub height: i64,
    pub timestamp: i64
}

#[derive(Serialize)]
struct GqlQueryVariables {
    /// height
    min: i64,
    /// page_size
    limit: i64
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
pub struct AoBlock {
    sql_client: Arc<SqliteClient>,
    client: reqwest::Client
}

impl AoBlock {
    #[allow(unused)]
    // todo: if SqliteClient isn't always needed make optional
    pub fn new(sql_client: Arc<SqliteClient>) -> Self {
        AoBlock { 
            sql_client,
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
    async fn may_fetch_next(&self, graphql_url: &str, page_size: i64, page_info: PageInfo, edges: &Vec<gql_return_types::Edge>, max_timestamp: i64) -> 
        Result<gql_return_types::BlocksTransactions, CuErrors> {
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
      let mut surpassed_max_timestamp_idx: i64 = -1;
      for i in 0..edges.len() {
        if edges[i].node.timestamp > max_timestamp {
            surpassed_max_timestamp_idx = i as i64;
            break;
        }        
      };

      if surpassed_max_timestamp_idx != -1 {
        println!("surpassed_max_timestamp_idx != -1");
        return Ok(gql_return_types::BlocksTransactions {
            page_info: Some(page_info),
            edges: edges[0..surpassed_max_timestamp_idx as usize].to_vec()
        });
      }
      if !page_info.has_next_page {
        println!("does not have has_next_page");
        return Ok(gql_return_types::BlocksTransactions {
            page_info: Some(page_info.clone()),
            edges: edges.to_vec()
        });
      }

      match self.fetch_page(edges.iter().last().unwrap().node.height, max_timestamp, page_size, graphql_url).await {
        Ok(res) => {
            //println!("start may_fetch_next");
            match self.may_fetch_next(graphql_url, page_size, res.data.blocks.page_info.clone().unwrap(), &res.data.blocks.edges, max_timestamp).await {
                Ok(mut res) => {
                    println!("next may_fetch_next");
                    let mut cloned_edges = edges.to_vec();
                    cloned_edges.append(&mut res.edges);
                    return Ok(gql_return_types::BlocksTransactions {
                        page_info: Some(page_info),
                        edges: cloned_edges
                    });
                },
                Err(e) => {
                    println!("error may_fetch_next");
                    Err(e)
                }
            }            
        },
        Err(e) => {
            println!("error may_fetch_next");
            Err(e)
        }
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
}

#[async_trait]
impl FindBlocksSchema for AoBlock {
    async fn find_blocks(&self, min_height: i64, max_timestamp: i64) -> Result<Vec<BlockSchema>, sqlx::error::Error> {
        let query = AoBlock::create_select_blocks_query(min_height, max_timestamp);
        let mut raw_query = sqlx::query_as::<_, BlockSchema>(&query.sql);
        for params in query.parameters.iter() {
            for param in params {
                let _raw_query = raw_query
                    .bind(param);
                raw_query = _raw_query;
            }            
        }
        
        match raw_query.fetch_all(self.sql_client.get_conn()).await {
                Ok(res) => Ok(res),
                Err(e) => Err(e)
        }
    }
}

#[async_trait]
impl SaveBlocksSchema for AoBlock {
    #[allow(unused)]
    async fn save_blocks(&self, blocks: &Vec<BlockSchema>) -> Result<(), sqlx::error::Error> {
        if blocks.len() == 0 { return Ok(()) }
        
        let block_docs = blocks.iter().map(|block| BlockDocSchema { id: block.height, height: block.height, timestamp: block.timestamp }).collect::<Vec<BlockDocSchema>>();
        let query = AoBlock::create_insert_blocks_query(&block_docs);
        let mut raw_query = sqlx::query::<Sqlite>(&query.sql);
        for params in query.parameters.iter() {
            for param in params {
                raw_query = raw_query.bind(param);
            }            
        }
        
        match raw_query.execute(self.sql_client.get_conn()).await {
                Ok(res) => Ok(()),
                Err(e) => Err(e)
        }
    }
}

#[async_trait]
impl LoadBlocksMetaSchema for AoBlock {
    async fn load_blocks_meta(&self, min_height: i64, max_timestamp: i64, graphql_url: &str, page_size: i64) -> Result<Vec<gql_return_types::Node>, CuErrors> {
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

// some tests in integration folder
#[cfg(test)]
mod tests {
    use crate::domain::client::{ao_block::AoBlock, sqlite::{ConnGetter, Repository, SqliteClient}};
    use crate::tests::fixtures::log::get_logger;
    use crate::tests::domain::client::test_sqlite::delete_db_files;    
    use super::*;
    use crate::domain::{dal::FindBlocksSchema, model::model::BlockSchema};
    
    mod find_blocks {
        use super::*;

        mod find_the_blocks {
            use super::*;

            struct MockFindBlocks;
            #[async_trait]
            impl FindBlocksSchema for MockFindBlocks {
                async fn find_blocks(&self, min_height: i64, max_timestamp: i64) -> Result<Vec<BlockSchema>, sqlx::error::Error> {
                    assert!(min_height == 123);
                    assert!(max_timestamp == 456);
                    Ok(
                        vec![
                            BlockSchema {
                                height: 123,
                                timestamp: 123
                            },
                            BlockSchema {
                                height: 124,
                                timestamp: 345
                            },
                            BlockSchema {
                                height: 125,
                                timestamp: 456
                            }
                        ]
                    )
                }
            }

            #[tokio::test]
            async fn test_find_the_blocks() {
                let mock = MockFindBlocks;
                match mock.find_blocks(123, 456).await {
                    Ok(res) => {
                        assert!(res[0].height == 123);
                        assert!(res[0].timestamp == 123);
                        assert!(res[1].height == 124);
                        assert!(res[1].timestamp == 345);
                        assert!(res[2].height == 125);
                        assert!(res[2].timestamp == 456);
                    },
                    Err(e) => panic!("test_find_blocks failed {:?}", e)
                }
            }
        }

        mod return_an_empty_array_if_no_blocks_are_found {
            use super::*;

            struct MockFindBlocks;
            #[async_trait]
            impl FindBlocksSchema for MockFindBlocks {
                async fn find_blocks(&self, _min_height: i64, _max_timestamp: i64) -> Result<Vec<BlockSchema>, sqlx::error::Error> {
                    Ok(vec![])
                }
            }

            #[tokio::test]
            async fn test_return_an_empty_array_if_no_blocks_are_found() {
                let mock = MockFindBlocks;
                match mock.find_blocks(123, 456).await {
                    Ok(res) => assert!(res.len() == 0),
                    Err(e) => panic!("{:?}", e)
                }
            }
        }
    }

    mod save_blocks {
        use super::*;

        mod save_the_blocks {
            use super::*;

            struct MockSaveBlocks;
            #[async_trait]
            impl SaveBlocksSchema for MockSaveBlocks {
                async fn save_blocks(&self, blocks: &Vec<BlockSchema>) -> Result<(), sqlx::error::Error> {
                    let block_docs = blocks.iter().map(|block| BlockDocSchema { id: block.height, height: block.height, timestamp: block.timestamp }).collect::<Vec<BlockDocSchema>>();
                    let query = AoBlock::create_insert_blocks_query(&block_docs);

                    for i in 0..query.parameters.len() {
                        assert!(query.parameters[i][0] == blocks[i].height);
                        assert!(query.parameters[i][1] == blocks[i].height);
                        assert!(query.parameters[i][2] == blocks[i].timestamp);
                    }
                    Ok(())
                }
            }

            #[tokio::test]
            async fn test_save_the_blocks() {
                let mock = MockSaveBlocks;
                let blocks = vec![
                    BlockSchema {
                        height: 123,
                        timestamp: 123
                    },
                    BlockSchema {
                        height: 124,
                        timestamp: 345
                    },
                    BlockSchema {
                        height: 125,
                        timestamp: 456
                    }
                ];                
                
                match mock.save_blocks(&blocks).await {
                    Ok(_) => (),
                    Err(e) => panic!("{:?}", e)
                }
            }
        }

        mod should_noop_a_block_if_it_already_exists_the_blocks {
            use super::*;

            struct MockSaveBlocks;
            #[async_trait]
            impl SaveBlocksSchema for MockSaveBlocks {
                async fn save_blocks(&self, blocks: &Vec<BlockSchema>) -> Result<(), sqlx::error::Error> {
                    let block_docs = blocks.iter().map(|block| BlockDocSchema { id: block.height, height: block.height, timestamp: block.timestamp }).collect::<Vec<BlockDocSchema>>();
                    let query = AoBlock::create_insert_blocks_query(&block_docs);

                    assert!(query.sql.contains("INSERT OR IGNORE"));
                    Ok(())
                }
            }

            #[tokio::test]
            async fn test_save_the_blocks() {
                let mock = MockSaveBlocks;
                let blocks = vec![
                    BlockSchema {
                        height: 123,
                        timestamp: 123
                    },
                    BlockSchema {
                        height: 124,
                        timestamp: 345
                    },
                    BlockSchema {
                        height: 125,
                        timestamp: 456
                    }
                ];                
                
                match mock.save_blocks(&blocks).await {
                    Ok(_) => (),
                    Err(e) => panic!("{:?}", e)
                }
            }
        }

        mod should_do_nothing_if_no_blocks_to_save {
            use super::*;

            #[tokio::test]
            async fn test_should_do_nothing_if_no_blocks_to_save() {
                let db_file = "aoblockunit1.db";
                let db_url = format!("sqlite://{}", db_file);

                let sql_client_arc = Arc::new(SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await);
                let ao_block = AoBlock::new(sql_client_arc.clone());
                let blocks = vec![];                
                
                match ao_block.save_blocks(&blocks).await {
                    Ok(_) => (),
                    Err(e) => panic!("{:?}", e)
                }

                sql_client_arc.clone().get_conn().close().await;
                delete_db_files(db_file);
            }
        }
    }
}