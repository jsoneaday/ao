use std::sync::Arc;
use ao_common::domain::{builder::Builder, dal::{Config, Gateway, Log, Signer, Uploader, Wallet}, UnitLog};
use ao_common::domain::uploader::UploaderClient;
use ao_common::domain::signer::ArweaveSigner;
use crate::domain::{clients::{gateway::ArweaveGateway, wallet::FileWallet}, config::AoConfig};
use dotenv::dotenv;

pub struct Deps {
    // pub data_store: Arc<dyn DataStore>,
    pub logger: Arc<dyn Log>,
    pub config: Arc<dyn Config>,
    pub gateway: Arc<dyn Gateway>,
    pub signer: Arc<dyn Signer>,
    pub wallet: Arc<dyn Wallet>,
    pub uploader: Arc<dyn Uploader>,

    // pub scheduler: Arc<scheduler::ProcessScheduler>,
}

pub async fn init_deps(mode: Option<String>) -> Arc<Deps> {
    let logger: Arc<dyn Log> = UnitLog::init();
 
    let config = Arc::new(AoConfig::new(mode).expect("Failed to read configuration"));

    let gateway: Arc<dyn Gateway> = Arc::new(
        ArweaveGateway::new()
            .await
            .expect("Failed to initialize gateway"),
    );

    let signer =
        Arc::new(ArweaveSigner::new(&config.wallet_path).expect("Invalid su wallet path"));

    let wallet = Arc::new(FileWallet);

    let uploader = Arc::new(
        UploaderClient::new(&config.upload_node_url, logger.clone()).expect("Invalid uploader url"),
    );
 
    Arc::new(Deps {
        logger,
        config,
        gateway,
        signer,
        wallet,
        uploader,
    })
}

pub fn init_builder(deps: &Arc<Deps>) -> Result<Builder, String> {
    dotenv().ok();
    let builder = Builder::new(deps.gateway.clone(), deps.signer.clone(), &deps.logger)?;
    return Ok(builder);
}

