use sqlx::{prelude::FromRow, sqlite::SqliteQueryResult, Sqlite};
use crate::domain::{maths::increment, model::model::BlockSchema};
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
    parameters: Vec<(i64, i64, i64)>
}

#[allow(unused)]
pub struct AoBlock<'a> {
    client: &'a SqliteClient
}

impl<'a> AoBlock<'a> {
    #[allow(unused)]
    pub fn new(client: &'a SqliteClient) -> Self {
        AoBlock { client }
    }

    #[allow(unused)]
    fn create_query(blocks: &Vec<BlockDocSchema>) -> Query {
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
                let result: (i64, i64, i64) = (block.id, block.height, block.timestamp);
                result
            }).collect()
        }
    }

    #[allow(unused)]
    pub async fn save_block(&self, blocks: &Vec<BlockSchema>) -> Result<Option<SqliteQueryResult>, sqlx::error::Error> {
        if blocks.len() == 0 { return Ok(None) }

        let block_docs = blocks.iter().map(|block| BlockDocSchema { id: block.height, height: block.height, timestamp: block.timestamp }).collect::<Vec<BlockDocSchema>>();
        let query = AoBlock::create_query(&block_docs);
        let mut raw_query = sqlx::query::<Sqlite>(&query.sql);
        for (a, b, c) in query.parameters.iter() {
            let _raw_query = raw_query
                .bind(a)
                .bind(b)
                .bind(c);
            raw_query = _raw_query;
        }
        
        match raw_query
            .execute(self.client.get_conn())
            .await {
                Ok(res) => Ok(Some(res)),
                Err(e) => Err(e)
        }
    }
}