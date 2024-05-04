mod config;
mod dal;
mod utils {
    pub mod utils;
    pub mod error;
    pub mod maths;
    pub mod strings;
}
pub mod client {
    pub mod ao_block;
    pub mod arweave;
    pub mod sqlite;
}
mod clients {
    pub mod gateway;
    pub mod wallet;
}
mod core {
    mod flows;
}
pub mod model {        
    pub mod model;
    pub mod domain_config_schema;
    mod shared_validation;
    mod stream_validation;
    pub mod parse_schema;
    pub mod server_config_schema;
    mod positive_int_schema;
    mod url_parse_schema;
    mod db_mode_schema;
    mod db_max_listeners_schema;
    mod boolean_schema;
    mod uuid_array_schema;
}    

use std::sync::Arc;
use ao_common::domain::dal::Log;

pub use crate::domain::model::domain_config_schema::DomainConfigSchema;
pub use crate::domain::utils::maths;
pub use crate::domain::utils::utils as schema_utils;
pub use crate::domain::utils::strings;
use crate::domain::client::arweave::InternalArweave;

#[allow(unused)]
pub struct DomainContext {
    arweave: InternalArweave,
    domain_config_schema: DomainConfigSchema,
    logger: Arc<dyn Log>
}

impl DomainContext {
    pub async fn create_api(ctx: DomainContext) {
        ctx.logger.log("Creating business logic apis".to_string());

    }
}
