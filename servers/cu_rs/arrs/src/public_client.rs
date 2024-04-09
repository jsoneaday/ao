//! Arweave Public Client is a set of tools for using some Arweaves
//! features which do not need a keypair. It includes searching
//! database, retrieving Arweave network and transaction information,
//! and downloading files.

use reqwest;
use std::error::Error;
use crate::{
    downloader::Downloader,
};

#[derive(Debug, Clone)]
pub struct ArPublic {
    gateway: String,
    http_client: reqwest::Client,
    downloader: Downloader,
}

impl ArPublic {
    
    /// Create a new instance of a Arweave public client.
    pub fn new() -> Self {
        Self {
            gateway: "https://arweave.net/".to_string(),
            http_client: reqwest::Client::new(),
            downloader: Downloader::default(),
        }
    }

    /// Get the current gateway.
    pub fn gateway(&self) -> String {
        self.gateway.clone()
    }

    /// Change Arweave gateway. The format should look like
    /// `https://arweave.net/`.
    pub fn set_gateway(&mut self, address: &str) {
        self.gateway = address.to_string();
    }

    /// Get the current http client.
    pub fn http_client(&self) -> &reqwest::Client {
        &self.http_client
    }

    /// Get transaction by id. This will return an already submitted
    /// Arweave transaction in a format of JSON. You can deserialize
    /// it to further use the information.
    pub async fn transaction(&self, id: &str)
                             -> Result<String, Box<dyn Error>> {
        let mut api_url: String = self.gateway.clone();
        api_url.push_str("tx/");
        api_url.push_str(id);
        let res =self.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    /// Get a single chunk of a file by inputting one of its
    /// offset. This method is useful for creating a resumable chunks
    /// downloader.
    pub async fn chunk(&self, offset: &str)
                       -> Result<String, Box<dyn Error>> {
        let mut api_url: String = self.gateway.clone();
        api_url.push_str("chunk/");
        api_url.push_str(offset);
        let res =self.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    /// Get a transaction offset and size. This method is useful for
    /// creating a resumable chunks downloader.
    pub async fn transaction_offset_size(&self, id: &str)
                                         -> Result<String, Box<dyn Error>> {
        let mut api_url: String = self.gateway.clone();
        api_url.push_str("tx/");
        api_url.push_str(id);
        api_url.push_str("/offset");
        let res =self.http_client().get(&api_url).send().await?;
        Ok(res.text().await?)
    }

    /// Get the reference of the downloader. You have to setup a new
    /// download or resume a previous download before it can do
    /// anything.
    pub fn downloader(&self) -> &Downloader {
        &self.downloader
    }

    /// Setup a new download by inputting a data transaction id. This
    /// can be anyone's data transaction. It is not neccessary to
    /// belong the current wallet.
    pub async fn new_download(&mut self, id: &str)
                              -> Result<(), Box<dyn Error>> {
        self.downloader.new_download(&self.clone(), id).await?;
        Ok(())
    }

    /// Download the current chunk based on the information of the
    /// Downloader. You have to setup a new download or resume a
    /// previous download before it can actually download anything.
    pub async fn download(&mut self)
                          -> Result<String, Box<dyn Error>> {
        Ok(self.downloader.download(&self.clone()).await?)
    }
}
