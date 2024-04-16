use bytes::{BufMut, Bytes};
use bundlr_sdk::{error::BundlrError, index::SignerMap, tags::Tag};
use sha2::{Digest, Sha256, Sha384};
use base64_url;
use ring::rand::SecureRandom;

#[derive(Debug)]
pub enum ByteErrorType {
    ByteError(String)
}

impl From<BundlrError> for ByteErrorType {
    fn from(error: BundlrError) -> Self {
        ByteErrorType::ByteError(format!("Byte error: {}", error))
    }
}

impl From<&str> for ByteErrorType {
    fn from(error: &str) -> Self {
        ByteErrorType::ByteError(format!("Byte error: {}", error))
    }
}

#[derive(Clone)]
pub struct DataBundle {
    pub items: Vec<DataItem>,
    pub tags: Vec<Tag>
}

impl DataBundle {
    
}

enum Data {
    None,
    Bytes(Vec<u8>)
}

#[derive(Clone)]
pub struct DataItem {
    signature_type: SignerMap,
    pub signature: Vec<u8>,
    owner: Vec<u8>,
    target: Vec<u8>,
    anchor: Vec<u8>,
    tags: Vec<u8>,
    data: Data
}