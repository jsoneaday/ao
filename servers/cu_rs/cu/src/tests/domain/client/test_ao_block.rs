#[tokio::test]
async fn test_save_block() {
    use crate::domain::{client::{ao_block::AoBlock, sqlite::{ConnGetter, Repository, SqliteClient}}, model::model::BlockSchema};
    use crate::tests::fixtures::log::get_logger;
    use crate::tests::domain::client::test_sqlite::delete_db_files;

    let db_file = "aoblock1.db";
    let db_url = format!("sqlite://{}", db_file);

    let client = SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await;
    let ao_block = AoBlock::new(&client);

    let blocks = vec![
        BlockSchema { height: 22, timestamp: 324453 },
        BlockSchema { height: 56, timestamp: 435435 },
        BlockSchema { height: 8, timestamp: 678768 }
    ];
    let result = ao_block.save_block(&blocks).await;
    match result {
        Ok(res) => match res {
            Some(res) => assert!(res.rows_affected() == blocks.len() as u64),
            None => panic!("blocks parameter is empty")
        },
        Err(e) => panic!("{:?}", e)
    }

    client.get_conn().close().await;
    delete_db_files(db_file);
}

#[tokio::test]
async fn test_find_blocks() {
    use crate::domain::{client::{ao_block::AoBlock, sqlite::{ConnGetter, Repository, SqliteClient}}, model::model::BlockSchema};
    use crate::tests::fixtures::log::get_logger;
    use crate::tests::domain::client::test_sqlite::delete_db_files;

    let db_file = "aoblock2.db";
    let db_url = format!("sqlite://{}", db_file);

    let client = SqliteClient::init(db_url.as_str(), get_logger(), Some(true), None).await;
    let ao_block = AoBlock::new(&client);

    let blocks = vec![
        BlockSchema { height: 22, timestamp: 324453 },
        BlockSchema { height: 23, timestamp: 324452 },
    ];
    let result = ao_block.save_block(&blocks).await;
    match result {
        Ok(res) => match res {
            Some(_) => (),
            None => panic!("blocks parameter is empty")
        },
        Err(e) => panic!("{:?}", e)
    };

    let result = ao_block.find_blocks(22, 324453).await;
    match result {
        Ok(res) => assert!(res.len() == 2),
        Err(e) => panic!("{:?}", e)
    }

    client.get_conn().close().await;
    delete_db_files(db_file);
}
