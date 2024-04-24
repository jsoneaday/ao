use crate::domain::model::domain_config_schema::DomainConfigSchema;
use log::info;
use crate::domain::client::arweave::InternalArweave;

#[allow(unused)]
pub struct DomainIndex {
    arweave: InternalArweave
}

impl DomainIndex {
    pub async fn create_api(_domain: DomainConfigSchema) {
        info!("Creating business logic apis");


    }
}