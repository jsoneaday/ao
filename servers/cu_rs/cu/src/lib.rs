pub mod app;
pub mod config;
pub mod app_state;
mod tests;
pub mod domain;
pub mod env_vars;
pub mod utils {
    pub mod datetime;
    pub mod string_converters;
    pub mod paths;
}
pub mod routes {
    pub mod index;
    pub mod state;
    pub mod middleware {
        pub mod with_error_handler;
    }
}

use crate::app::server;

pub async fn run() -> std::io::Result<()> {
    server().await
}