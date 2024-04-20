use std::path::PathBuf;

use arweave_rs::{Arweave, ArweaveBuilder};
use arweave_rs::crypto::hash::{deep_hash, hash_all_sha256, DeepHashItem};
use once_cell::sync::OnceCell;
use crate::errors::ArBundleErrors;

pub struct CryptoDriver {
    arweave: Arweave
}

impl CryptoDriver {
    pub fn new(keypair_path: &str) -> Self {
        let mut path = PathBuf::new();
        path.push(keypair_path);
        Self {
            arweave: ArweaveBuilder::new().keypair_path(path).build().unwrap()
        }
    }

    pub fn get_public_key(&self) -> Vec<u8> {
        self.arweave.signer.as_ref().unwrap().keypair_modulus().0
    }

    // pub fn from_jwk(jwk: jwk::JsonWebKey) -> Result<RsaPrivateKey, ArBundleErrors> {
    //     let pem = jwk.key.as_ref().to_pem();
    //     match RsaPrivateKey::from_pkcs1_pem(&pem) {
    //         Ok(priv_key) => Ok(priv_key),
    //         Err(e) => Err(ArBundleErrors::KeyCreationFailed(Some(Box::new(e))))
    //     }        
    // }

    // pub fn from_keypair_path(keypair_path: &str) -> Result<RsaPrivateKey, ArBundleErrors> {
    //     match fs::read_to_string(keypair_path) {
    //         Ok(keypair_string) => {
    //             match keypair_string.parse().map_err(ArBundleErrors::JsonWebKeyError) {
    //                 Ok(jwk_parsed) => {
    //                     match Self::from_jwk(jwk_parsed) {
    //                         Ok(key) => Ok(key),
    //                         Err(e) => Err(e)
    //                     }                        
    //                 },
    //                 Err(e) => Err(e)
    //             }
    //         },
    //         Err(e) => Err(ArBundleErrors::ReadKeyPairFileFailed(Box::new(e)))
    //     }
    // }

    pub fn sign(&self, message: &[u8]) -> Result<Vec<u8>, ArBundleErrors>{
        match self.arweave.sign(message) {
            Ok(res) => Ok(res),
            Err(e) => Err(ArBundleErrors::ArweaveError(e))
        }
    }

    pub fn verify(&self, pk: &[u8], message: &[u8], signature: &[u8]) -> Result<(), ArBundleErrors> {
        match Arweave::verify(pk, message, signature) {
            Ok(_res) => Ok(()),
            Err(e) => Err(ArBundleErrors::ArweaveError(e))
        }
    }

    pub fn hash(&self, message: &[u8]) -> [u8; 32] {
        hash_all_sha256(vec![message])
    }

    pub fn deep_hash(items: &Vec<u8>) -> [u8; 48] {
        let item = DeepHashItem::from_item(items);
        deep_hash(item)
    }

    pub fn string_to_buffer(str: &str) -> &[u8] {
        str.as_bytes()
    }

    // pub fn concat_buffers(
    //     buffers: [u8],
    //   ) -> [u8] {
    //     let mut total_length = 0;
      
    //     for i in 0..buffers.len() {
    //       total_length += buffers[i].len();
    //     }
      
    //     let mut temp = [0u8; total_length];
    //     let mut offset = 0;
      
    //     temp[offset..1].copy_from_slice(buffers[0]);
    //     offset += buffers[0].len();
      
    //     for i in 1..buffers.len() {
    //       temp[offset..buffers.len()].copy_from_slice(buffers[i]);
    //       offset += buffers[i].len();
    //     }
      
    //     return temp;
    // }
}

static CRYPTO_DRIVER: OnceCell<CryptoDriver> = OnceCell::new();
pub fn get_crypto_driver(keypair_path: &str) -> &CryptoDriver {
    CRYPTO_DRIVER.get_or_init(|| {
        CryptoDriver::new(keypair_path)
    })
}