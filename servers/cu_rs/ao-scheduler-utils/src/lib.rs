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
mod dal;
pub mod index_common;

/// lib.rs is effectively index.js

static CONNECT: OnceCell<ConnectReturn> = OnceCell::new();
pub fn get_connect() -> &'static ConnectReturn {
    CONNECT.get_or_init(|| {
        dotenv::dotenv().ok();
        
        let graphql_url = std::env::var("GRAPHQL_URL").unwrap();
        let cache_size = std::env::var("SCHEDULER_UTILS_CACHE_SIZE").unwrap_or("10".to_owned()).parse::<u64>().unwrap();
        let follow_redirects = std::env::var("SCHEDULER_UTILS_FOLLOW_REDIRECTS").unwrap_or("true".to_owned()).parse::<bool>().ok();        
        
        connect(Some(cache_size), Some(&graphql_url), follow_redirects)
    })
}