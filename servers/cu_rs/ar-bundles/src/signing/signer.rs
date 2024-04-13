use std::any::Any;
use crate:: errors::ArBundleErrors;

pub enum StringOrVecu8 {
    StringType(String),
    BufferType(Vec<u8>)
}

pub struct Signer {
    pub signer: Option<Box<dyn Any>>, // any
    pub public_key: Vec<u8>,
    pub signature_type: i64,
    pub signature_length: usize,
    pub owner_length: usize,
    pub pem: String,
    pub keypair_path: String
}

pub struct Options;

pub trait SignerMaker {
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, ArBundleErrors>;
    // async fn sign_data_item(data_item: StringOrVecu8, tags: Vec<Tag>) -> Result<DataItem, ArBundleErrors>;
    // async fn set_public_key() -> Result<(), ArBundleErrors>;
    // async fn get_address() -> Result<String, ArBundleErrors>;
    fn verify(&self, _pk: &[u8], _message: &[u8], _signature: &[u8]) -> bool {
        unimplemented!("You must implement verify method on child");
    }

    fn get_keypair_path(&self) -> String;
}