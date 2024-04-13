use crate::errors::ArBundleErrors;
use crate::interface_jwk::JWKInterface;
use crate::key_utils::get_crypto_driver;
use crate::constants::{get_sig_config, SignatureConfig};
use crate::signing::signer::SignerMaker;

#[allow(unused)]
pub struct ArweaveSigner {
    signature_type: i64,
    owner_length: usize,
    signature_length: usize,
    jwk: JWKInterface,
    pub pk: String,
    keypair_path: String
}

impl ArweaveSigner {
    pub fn new(jwk: JWKInterface, keypair_path: &str) -> Self {
        let sig_config = get_sig_config();
        Self {
            signature_type: 1,
            owner_length: sig_config.get(&SignatureConfig::ARWEAVE).unwrap().pub_length,
            signature_length: sig_config.get(&SignatureConfig::ARWEAVE).unwrap().sig_length,
            pk: jwk.base.n.clone(),
            jwk,            
            keypair_path: keypair_path.to_string()
        }
    }

    pub fn get_public_key(&self) -> Vec<u8> {
        let mut output: Vec<u8> = vec![];
        base64_url::encode_to_vec(&self.pk, &mut output);
        output
    }

    pub fn sign(&self, message: &Vec<u8>) -> Result<Vec<u8>, ArBundleErrors> {
        return get_crypto_driver(self.keypair_path.as_str()).sign(message);
    }
}

impl SignerMaker for ArweaveSigner {     
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, ArBundleErrors> {
        match get_crypto_driver(self.keypair_path.as_str()).sign(message) {
            Ok(res) => Ok(res),
            Err(e) => Err(e)
        }
    }
    /// todo: figure out what to do about keypair_path
    fn verify(&self, pk: &[u8], message: &[u8], signature: &[u8]) -> bool {
        match get_crypto_driver(self.keypair_path.as_str()).verify(pk, message, signature) {
            Ok(()) => true,
            Err(_) => false
        }
    }

    fn get_keypair_path(&self) -> String {
        self.keypair_path.clone()
    }
}