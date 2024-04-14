use crate::{interface_jwk::JWKInterface, signing::chains::arweave_signer::ArweaveSigner};
use std::collections::HashMap;
use once_cell::sync::OnceCell;
use async_trait::async_trait;

#[async_trait]
pub trait IndexToTypeValueFn {
    async fn verify(keypair_path: &str, pk: &Vec<u8>, message: &Vec<u8>, signature: &Vec<u8>) -> bool;
}

pub enum IndexToTypeValue {
    ArweaveSigner(ArweaveSigner)
}

pub type IndexToType = HashMap<i64, IndexToTypeValue>;

static INDEX_TO_TYPE: OnceCell<IndexToType> = OnceCell::new();
pub fn get_index_to_type<T: IndexToTypeValueFn>(jwk: JWKInterface, keypair_path: &str) -> &IndexToType {
    INDEX_TO_TYPE.get_or_init(|| {
        let index_to_type = HashMap::from([
            (1, IndexToTypeValue::ArweaveSigner(ArweaveSigner::new(jwk, keypair_path)))
        ]);
        index_to_type
    })
}
