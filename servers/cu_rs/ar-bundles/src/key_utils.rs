use std::fs;
use once_cell::sync::OnceCell;
use rsa::{pkcs1::{DecodeRsaPrivateKey, EncodeRsaPublicKey}, RsaPrivateKey};
use jsonwebkey as jwk;
use crate::errors::ArBundleErrors;

pub struct CryptoDriver {
    private_key: RsaPrivateKey
}

impl CryptoDriver {
    pub fn new(keypair_path: &str) -> Self {        
        Self {
            private_key: CryptoDriver::from_keypair_path(keypair_path).unwrap()
        }
    }

    pub fn create_public_key(keypair_path: &str) -> Result<String, ArBundleErrors> {
        match Self::from_keypair_path(keypair_path) {
            Ok(key) => {
                match key.to_public_key().to_pkcs1_pem(rsa::pkcs8::LineEnding::LF) {   
                    Ok(pub_key_str) => Ok(pub_key_str),
                    Err(e) => Err(ArBundleErrors::KeyCreationFailed(Some(Box::new(e))))
                }
            },
            Err(e) => Err(e)
        }        
    }

    pub fn from_jwk(jwk: jwk::JsonWebKey) -> Result<RsaPrivateKey, ArBundleErrors> {
        let pem = jwk.key.as_ref().to_pem();
        match RsaPrivateKey::from_pkcs1_pem(&pem) {
            Ok(priv_key) => Ok(priv_key),
            Err(e) => Err(ArBundleErrors::KeyCreationFailed(Some(Box::new(e))))
        }        
    }

    pub fn from_keypair_path(keypair_path: &str) -> Result<RsaPrivateKey, ArBundleErrors> {
        match fs::read_to_string(keypair_path) {
            Ok(keypair_string) => {
                match keypair_string.parse().map_err(ArBundleErrors::JsonWebKeyError) {
                    Ok(jwk_parsed) => {
                        match Self::from_jwk(jwk_parsed) {
                            Ok(key) => Ok(key),
                            Err(e) => Err(e)
                        }                        
                    },
                    Err(e) => Err(e)
                }
            },
            Err(e) => Err(ArBundleErrors::ReadKeyPairFileFailed(Box::new(e)))
        }
    }
}

static CRYPTO_DRIVER: OnceCell<CryptoDriver> = OnceCell::new();
pub fn get_crypto_driver(keypair_path: &str) -> &CryptoDriver {
    CRYPTO_DRIVER.get_or_init(|| {
        CryptoDriver::new(keypair_path)
    })
}