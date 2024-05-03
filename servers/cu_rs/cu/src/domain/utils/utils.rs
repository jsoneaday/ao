use std::io::Read;
use std::path::PathBuf;
use std::fs::canonicalize;
use std::fs::File;
use crate::config::StartConfigEnv;
use super::error::{CuErrors, SchemaValidationError};
use super::strings::IsNoneOrEmpty;

/**
 * If the WALLET_FILE env var is defined, load the contents from the file.
 * Refuse to boot the app if both or none of WALLET and WALLET_FILE are defined.
 */
pub fn preprocess_wallet(mut config: StartConfigEnv) -> Result<StartConfigEnv, CuErrors> {
    // nothing to do here
    if !config.WALLET.is_none_or_empty() && config.WALLET_FILE.is_none_or_empty() {
        return Ok(config);
    }
  
    if config.WALLET.is_none_or_empty() && config.WALLET_FILE.is_none_or_empty() {
        return Err(CuErrors::SchemaValidation(SchemaValidationError { message: "One of WALLET or WALLET_FILE is required".to_string() }));
    }
    if !config.WALLET.is_none_or_empty() && !config.WALLET_FILE.is_none_or_empty() {
        return Err(CuErrors::SchemaValidation(SchemaValidationError { message: "Do not define both WALLET and WALLET_FILE".to_string() }));
    }
  
    let wallet_path = PathBuf::from(config.WALLET_FILE.clone().unwrap());
    let wallet_path = canonicalize(wallet_path).unwrap();
    if !wallet_path.exists() {
        return Err(CuErrors::SchemaValidation(
            SchemaValidationError { message: format!("WALLET_FILE does not exist: {}", wallet_path.to_string_lossy()) }
        ));
    }
  
    match File::open(&wallet_path) {
        Ok(mut file) => {
            let mut wallet = "".to_string();
            _ = file.read_to_string(&mut wallet);
            config.WALLET = Some(wallet);
            Ok(config)
        },
        Err(e) => Err(CuErrors::SchemaValidation(
            SchemaValidationError { message: format!("An error occurred while reading WALLET_FILE from {}\n{:?}", wallet_path.to_string_lossy(), e) }
        ))
    }
  }

/**
 * If either ARWEAVE_URL or GRAPHQL_URL is not defined, then set them to their defaults
 * using GATEWAY_URL, which will always have a value.
 */
pub fn preprocess_urls(mut config: StartConfigEnv) -> Result<StartConfigEnv, CuErrors> {
    if !config.ARWEAVE_URL.is_none_or_empty()
        && !config.GRAPHQL_URL.is_none_or_empty()
        && !config.CHECKPOINT_GRAPHQL_URL.is_none_or_empty() {
        return Ok(config);
    }
  
    if config.GATEWAY_URL.is_none_or_empty() {
      if config.ARWEAVE_URL.is_none_or_empty() && config.GRAPHQL_URL.is_none_or_empty() {
        return Err(CuErrors::SchemaValidation(SchemaValidationError {
            message: "GATEWAY_URL is required, if either ARWEAVE_URL or GRAPHQL_URL is not provided".to_string()
        }));
      }
      if config.ARWEAVE_URL.is_none_or_empty() {
        return Err(CuErrors::SchemaValidation(SchemaValidationError {
            message: "GATEWAY_URL is required if ARWEAVE_URL is not provided".to_string()
        }));
      }
      if config.GRAPHQL_URL.is_none_or_empty() {
        return Err(CuErrors::SchemaValidation(SchemaValidationError {
            message: "GATEWAY_URL is required if GRAPHQL_URL is not provided".to_string()
        }));
      }
    }
  
    if config.ARWEAVE_URL.is_none_or_empty() {
        config.ARWEAVE_URL = config.GATEWAY_URL.clone();
    }
    if config.GRAPHQL_URL.is_none_or_empty() {
        let mut path = PathBuf::new();
        path.push(config.GATEWAY_URL.clone().unwrap()); // by this line it should not be none
        path.push("/graphql");
        config.GRAPHQL_URL = Some(path.to_string_lossy().to_string());
    }
    if config.CHECKPOINT_GRAPHQL_URL.is_none_or_empty() {
        config.CHECKPOINT_GRAPHQL_URL = config.GRAPHQL_URL.clone();
    }
  
    Ok(config)
}