use std::collections::HashMap;
use once_cell::sync::OnceCell;

#[derive(Eq, PartialEq, Hash)]
pub enum SignatureConfig {
    ARWEAVE = 1,
    ED25519 = 2,
    ETHEREUM = 3,
    SOLANA = 4,
    INJECTEDAPTOS = 5,
    MULTIAPTOS = 6,
    TYPEDETHEREUM = 7,
}

pub struct SignatureMeta {
    pub sig_length: usize,
    pub pub_length: usize,
    pub sig_name: String
}

static SIG_CONFIG: OnceCell<HashMap<SignatureConfig, SignatureMeta>> = OnceCell::new();
pub fn get_sig_config() -> &'static HashMap<SignatureConfig, SignatureMeta> {
    SIG_CONFIG.get_or_init(|| {
        let mut sig_config: HashMap<SignatureConfig, SignatureMeta> = HashMap::with_capacity(7);
        sig_config.insert(
            SignatureConfig::ARWEAVE, 
            SignatureMeta {
                sig_length: 512,
                pub_length: 512,
                sig_name: "arweave".to_string()
            }
        );
        sig_config.insert(
            SignatureConfig::ED25519, 
            SignatureMeta {
                sig_length: 64,
                pub_length: 32,
                sig_name: "ed25519".to_string()
            }
        );
        sig_config.insert(
            SignatureConfig::ETHEREUM, 
            SignatureMeta {
                sig_length: 65,
                pub_length: 65,
                sig_name: "ethereum".to_string()
            }
        );
        sig_config.insert(
            SignatureConfig::SOLANA, 
            SignatureMeta {
                sig_length: 64,
                pub_length: 32,
                sig_name: "solana".to_string()
            }
        );
        sig_config.insert(
            SignatureConfig::INJECTEDAPTOS, 
            SignatureMeta {
                sig_length: 64,
                pub_length: 32,
                sig_name: "injectedAptos".to_string()
            }
        );
        sig_config.insert(
            SignatureConfig::MULTIAPTOS, 
            SignatureMeta {
                sig_length: 64 * 32 + 4, // max 32 64 byte signatures, +4 for 32-bit bitmap
                pub_length: 32 * 32 + 1, // max 64 32 byte keys, +1 for 8-bit threshold value
                sig_name: "multiAptos".to_string()
            }
        );
        sig_config.insert(
            SignatureConfig::TYPEDETHEREUM, 
            SignatureMeta {
                sig_length: 65,
                pub_length: 42,
                sig_name: "typedEthereum".to_string()
            }
        );

        sig_config
    })
}