use crate::{
    wallet::ArWallet,
    chunks::{Chunks},
    b64::b64_encode,
    chunk_validator::Validator,
};

use std::error::Error;
use serde::{Deserialize, Serialize};
use primitive_types::U256;

/// Manage all uploading information; handle uplaoding and
/// resuming. However, this struct does not include a copy of raw file
/// or chunks. They shoud be either borrowed from the current
/// transaction or import from a file path (such as resuming).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Uploader {
    current_idx: usize,
    data_root: Vec<u8>,
    data_size: usize,
    chunk_size: usize,
    chunks: Chunks
}

impl Uploader {
    /// Create a new uploader from a raw_data.
    pub fn new(raw_data: &[u8]) -> Self {
        let mut chunks = Chunks::new();
        chunks.finalize(&raw_data);
        Self {
            current_idx: 0,
            data_root: chunks.data_root().clone(),
            data_size: chunks.data_size(),
            chunk_size: chunks.chunks_len(),
            chunks: chunks,
        }
    }

    /// Save the uploader information as a json string. The saved data
    /// can be later used for resuming the uploading.
    pub fn uploader_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    /// Resume an uploading by input a saved uploader json
    /// information.
    pub fn resume(uploader_json: &str) -> Self {
        let uploader: Uploader = serde_json::from_str(uploader_json).unwrap();
        uploader
    }

    pub fn current_idx(&self) -> usize {
        self.current_idx
    }

    pub fn chunk_size(&self) -> usize {
        self.chunk_size
    }

    pub fn verify_uploader(&self, chunks: &Chunks) -> bool {
        if b64_encode(&self.data_root) == b64_encode(&chunks.data_root())
            && self.data_size == chunks.data_size()
            && self.chunk_size == chunks.chunks_len()
        {
            true
        } else {
            false
        }
    }

    /// Upload the current chunk.
    pub async fn upload(&mut self,
                        arwallet: &ArWallet)
                        -> Result<String, Box<dyn Error>> {
        let chunks = &self.chunks;
        if self.verify_uploader(&chunks) == false {
            return Err("Your uploader is invalid. Please create a data transaction first, or resume a previous interrupted uploader.".into());
        }
        let _validate = Validator::validate(
            chunks.data_root(),
            U256::from(chunks.proofs()[self.current_idx].offset()),
            U256::zero(),
            U256::from(chunks.data_size()),
            chunks.proofs()[self.current_idx].proof(),
        )?;
        let chunk_json = chunks.chunk_json(self.current_idx);
        let mut api_url = arwallet.gateway();
        api_url.push_str("chunk");
        let res = arwallet.http_client().post(&api_url)
            .body(chunk_json).send().await;
        match res {
            Ok(r) => {
                let res_status = r.status().as_u16();
                if res_status == 200 {
                    self.current_idx = self.current_idx + 1;
                    let completed_percentage
                        = self.current_idx as f64
                        / self.chunk_size as f64 * 100.0;
                    let mut completed_percentage
                        = (completed_percentage as usize).to_string();
                    completed_percentage.push_str("%");
                    let mut ok_string = "Chunk ".to_string();
                    ok_string.push_str(&self.current_idx.to_string());
                    ok_string.push_str(" completed. ");
                    ok_string.push_str(&completed_percentage);
                    Ok(ok_string)
                } else {
                    let res_text = r.text().await?;
                    let mut err_string = "Error: ".to_string();
                    err_string.push_str(&res_status.to_string());
                    err_string.push_str(" - ");
                    err_string.push_str(&res_text);
                    Err(err_string.into())
                }
            },
            Err(e) => {
                Err(e.into())
            }
        }        
    }
}
