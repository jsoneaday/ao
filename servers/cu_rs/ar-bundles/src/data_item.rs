use std::any::Any;
use base64_url;

use crate::{constants::SignatureConfig, errors::ArBundleErrors, utils::byte_array_to_long};

pub const MAX_TAG_BYTES: usize = 4096;
pub const MIN_BINARY_SIZE: usize = 80;

pub struct DataItem {
    binary: Vec<u8>,
    _id: Option<Vec<u8>>
}

impl DataItem {
    fn new(binary: Vec<u8>) -> Self {
        Self {
            binary,
            _id: None
        }
    }

    fn is_data_item(obj: Box<dyn Any>) -> bool {
        let test = obj.downcast_ref::<DataItem>();
        if test.is_some() {
            return true;
        }
        false
    }

    fn get_signature_type(&self) -> Result<SignatureConfig, ArBundleErrors> {
        let signature_type_val = byte_array_to_long(self.binary.drain(0..2).collect::<Vec<u8>>());
        if SignatureConfig::ARWEAVE as u8 == signature_type_val {
            return Ok(SignatureConfig::ARWEAVE);
        } else if SignatureConfig::ED25519 as u8 == signature_type_val {
            return Ok(SignatureConfig::ED25519);
        } else if  SignatureConfig::ETHEREUM as u8 == signature_type_val {
            return Ok(SignatureConfig::ETHEREUM);
        } else if SignatureConfig::SOLANA as u8 == signature_type_val {
            return Ok(SignatureConfig::SOLANA);
        } else if SignatureConfig::INJECTEDAPTOS as u8 == signature_type_val {
            return Ok(SignatureConfig::INJECTEDAPTOS);
        } else if SignatureConfig::MULTIAPTOS as u8 == signature_type_val {
            return Ok(SignatureConfig::MULTIAPTOS);
        } else if SignatureConfig::TYPEDETHEREUM as u8 == signature_type_val {
            return Ok(SignatureConfig::TYPEDETHEREUM);
        }
            
        Err(ArBundleErrors::SignatureConfigTypeNotFound)
    }


}