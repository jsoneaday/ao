use index_common::{connect, ConnectReturn};
use once_cell::sync::OnceCell;

mod client {
    pub mod gateway;
    pub mod in_memory;
    pub mod scheduler;
}
mod validate;
mod err;
mod raw;
mod locate;
pub mod index_common;

/// lib.rs is effectively index.js

static CONNECT: OnceCell<ConnectReturn> = OnceCell::new();
pub fn get_connect() -> &'static ConnectReturn {
    CONNECT.get_or_init(|| {
        dotenv::dotenv().ok();
        
        let cache_size = std::env::var("SCHEDULER_UTILS_CACHE_SIZE").unwrap_or("10".to_owned()).parse::<u64>().unwrap();
        let follow_redirects = std::env::var("SCHEDULER_UTILS_FOLLOW_REDIRECTS").unwrap_or("true".to_owned()).parse::<bool>().ok();
        let wallet_path = std::env::var("WALLET_FILE").unwrap_or("".to_owned());
        let gateway = std::env::var("GATEWAY_URL").unwrap();
        let uploader = std::env::var("UPLOADER_URL").unwrap();
        
        connect(cache_size, &wallet_path, Some(&gateway), Some(&uploader), follow_redirects)
    })
}