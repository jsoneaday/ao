#[allow(unused)]
const PROCESS: &str = "zc24Wpv_i6NNCEdxeKt7dcNrqL5w0hrShtSCcFGGL24";

#[tokio::test]
async fn test_load_tx_meta() {
    use ao_common::test_utils::{get_uploader_url, get_wallet_file};
    use ao_common::test_utils::get_graphql_url;
    use crate::domain::client::arweave::InternalArweave;

    let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
    match arweave.load_tx_meta(get_graphql_url(), PROCESS).await {
        Ok(_) => (),
        Err(e) => panic!("test_load_tx_meta failed: {:?}", e)
    };
}

#[tokio::test]
async fn test_load_tx_data() {
    use ao_common::test_utils::{get_uploader_url, get_wallet_file};
    use ao_common::test_utils::get_arweave_url;
    use crate::domain::client::arweave::InternalArweave;

    let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
    match arweave.load_tx_data(get_arweave_url(), PROCESS).await {
        Ok(_) => (),
        Err(e) => panic!("test_load_tx_data failed: {:?}", e)
    };
}

#[tokio::test]
async fn test_build_sign_dataitem() {        
    use std::path::PathBuf;
    use ao_common::test_utils::{get_uploader_url, get_wallet_file};
    use bundlr_sdk::tags::Tag;
    use crate::domain::client::arweave::InternalArweave;

    const TEST_IMAGE_FILE: &str = "../test_utils/test.png";

    let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
    let mut path = PathBuf::new();
    path.push(TEST_IMAGE_FILE);
    
    let data = std::fs::read(path).unwrap();
    let data_item = arweave.build_sign_dataitem(data, vec![Tag { 
        name: "hello".to_string(), 
        value: "world".to_string()
    }]).unwrap();

    // try an upload to make sure its a valid data item
    let result = arweave.upload_data_item(get_uploader_url(), data_item).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_upload_dataitem() {
    use std::path::PathBuf;
    use ao_common::{domain::bytes::DataItem, test_utils::{get_uploader_url, get_wallet_file}};
    use bundlr_sdk::tags::Tag;
    use crate::domain::client::arweave::InternalArweave;

    const TEST_IMAGE_FILE: &str = "../test_utils/test.png";
    
    let arweave = InternalArweave::new(get_wallet_file(), get_uploader_url());
    
    let mut path = PathBuf::new();
    path.push(TEST_IMAGE_FILE);
        
    // let signer = Arc::new(ArweaveSigner::new(get_wallet_file()).expect("Invalid su wallet path"));
    let data = std::fs::read(path).unwrap();
    let mut data_item = DataItem::new(vec![], data, vec![
        Tag { 
            name: "Bundle-Format".to_string(), 
            value: "binary".to_string()
        },
        Tag { 
            name: "Bundle-Version".to_string(), 
            value: "2.0.0".to_string() 
        }
    ], base64_url::decode(&arweave.internal_arweave.get_pub_key().unwrap()).unwrap()).unwrap();
    // data_item.signature = signer.sign_tx(data_item.get_message().unwrap().to_vec()).await.unwrap();
    data_item.signature = arweave.internal_arweave.sign(&data_item.get_message().unwrap().to_vec()).unwrap();

    let result = arweave.upload_data_item(get_uploader_url(), data_item).await;
    assert!(result.is_ok());
}