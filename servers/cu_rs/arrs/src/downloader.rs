use crate::{
    public_client::ArPublic,
    b64::b64_decode,
    chunk_validator::Validator,
};

use std::error::Error;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use primitive_types::U256;
use f256::f256;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OffsetSize {
    size: String,
    offset: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownloadedChunk {
    tx_path: String,
    data_path: String,
    chunk: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Downloader {
    data_root: Vec<u8>,
    data_size: U256,
    current_offset: U256,
    end_point: U256,
    downloaded_data: Vec<u8>,
    current_proof_offset: U256,
}

impl Downloader {
    /// Create a new downloader for downloading a new file from very
    /// beginning.
    pub async fn new_download(&mut self, ap: &ArPublic, id: &str)
                              -> Result<(), Box<dyn Error>> {
        let tx_string = ap.transaction(&id).await?;
        let tx_value: Value = serde_json::from_str(&tx_string)?;
        let data_root_string = tx_value["data_root"]
            .as_str().unwrap().to_string();
        let data_root = b64_decode(&data_root_string)?;
        let offset_size_json = ap.transaction_offset_size(&id).await?;
        let offset_size: OffsetSize = serde_json::from_str(&offset_size_json)?;
        let offset = U256::from_dec_str(&offset_size.offset)?;
        let size =  U256::from_dec_str(&offset_size.size)?;
        let start_point = offset - size + 1;
        self.data_root = data_root;
        self.data_size = size;
        self.current_offset = start_point;
        self.end_point = offset;
        Ok(())
    }

    /// Get current offset.
    pub fn current_offset(&self) -> U256 {
        self.current_offset
    }

    /// Get current end point.
    pub fn end_point(&self) -> U256 {
        self.end_point
    }

    /// Get downloaded data.
    pub fn downloaded_data(&self) -> Vec<u8>{
        self.downloaded_data.clone()
    }
    
    /// Download the current chunk based on the current offset.
    pub async fn download(&mut self, ap: &ArPublic)
                          -> Result<String, Box<dyn Error>> {
        let chunk_json = ap.chunk(&self.current_offset.to_string()).await?;
        let chunk: DownloadedChunk = serde_json::from_str(&chunk_json)?;
        let b64_data = chunk.chunk;
        let result_data = b64_decode(&b64_data)?;
        let chunk_size = U256::from(result_data.len());
        self.current_proof_offset
            = self.current_proof_offset + chunk_size - U256::from(1);
        
        let _validate = Validator::validate(
            self.data_root.clone(),
            self.current_proof_offset,
            U256::zero(),
            self.data_size,
            b64_decode(&chunk.data_path)?,
        )?;
        
        self.downloaded_data.extend(&result_data);
        
        let remain = self.end_point - self.current_offset;
        let size = f256::from_str(&self.data_size.to_string())?;
        let remain = f256::from_str(&remain.to_string())?;
        let unfinished_ratio = remain / size;
        let finished_ratio = f256::from(1.0) - unfinished_ratio;
        let finished_percent = finished_ratio * f256::from(100.0);
        let finished_percent = finished_percent.round();
        let finished_percent = U256::from_dec_str(
            &finished_percent.to_string())?;
        self.current_offset = self.current_offset + chunk_size;
        let result = finished_percent.to_string();
        Ok(result)
    }
}
