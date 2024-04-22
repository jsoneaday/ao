use std::sync::Arc;

mod clients;
mod core;
mod logger;

pub use logger::UnitLog;
pub use core::dal;
pub use core::builder;
pub use core::bytes;
pub use clients::signer;
pub use clients::uploader;

// pub use core::router;
// pub use flows::Deps;

use async_trait::async_trait;

#[async_trait]
pub trait Dependencies<T> {
    async fn init_deps(mode: Option<String>) -> Arc<T>;
}
