use ao_common::{models::gql_models::GraphqlInput, network::utils::get_content_type_headers};
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
    async fn fetch_page(&self, min_height: i64, max_timestamp: i64, page_size: i64, graphql_url: &str) {
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

        let result = match result {
            Ok(res) => {
                let body_str = res.text().await.unwrap();
                println!("body_str {}", body_str);
                // match serde_json::from_str::<U>(&body_str) {
                //     Ok(res) => {
                //         Ok(res)
                //     },
                //     Err(e) => {                        
                //         self.sql_client.logger.error(e.to_string());
                //         Err(CuErrors::BlockMeta(Some(Box::new(e))))
                //     }
                // }
            },
            Err(e) => {
                self.sql_client.logger.error(
                    format!("Error Encountered when fetching page of block metadata from gateway with minBlock '{}' and maxTimestamp '{}'", min_height, max_timestamp).to_string()
                );
                // Err(CuErrors::BlockMeta(Some(Box::new(e))))
            }
        };
    }

    #[allow(unused)]
    async fn fetch_all_pages(min_height: i64, max_timestamp: i64) {
        let _max_timestamp = f64::floor(max_timestamp as f64 / 1000.0);


    }

    pub async fn load_blocks_meta(&self, graphql_url: &str, page_size: i64) {

    }
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_fetch_page() {
        use crate::domain::{client::{ao_block::AoBlock, sqlite::{ConnGetter, Repository, SqliteClient}}, model::model::BlockSchema};
        use crate::tests::fixtures::log::get_logger;
        use crate::tests::domain::client::test_sqlite::delete_db_files;

        let db_file = "aoblock2.db";
        let db_url = format!("sqlite://{}", db_file);

        let client = SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await;
        let ao_block = AoBlock::new(&client);

        // let result = ao_block.fetch_page(1, 12321, 10, "").await;
        // match result {
        //     Ok(res) => match res {
        //         Some(_) => (),
        //         None => panic!("blocks parameter is empty")
        //     },
        //     Err(e) => panic!("{:?}", e)
        // };

        // let result = ao_block.find_blocks(22, 324453).await;
        // match result {
        //     Ok(res) => assert!(res.len() == 2),
        //     Err(e) => panic!("{:?}", e)
        // }

        client.get_conn().close().await;
        delete_db_files(db_file);
    }
}