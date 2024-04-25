use async_trait::async_trait;
use crate::err::SchedulerErrors;

pub struct ProcessCacheEntry {
    pub url: String,
    pub ttl: Option<u64>,
    pub address: String
}

pub struct Scheduler {
    pub url: String,
    pub address: String,
    pub ttl: u64
}

#[async_trait]
pub trait CheckForRedirectSchema {
    async fn check_for_redirect (&self, url: &str, process: &str) -> Result<String, SchedulerErrors>;
}

/// ttl: milliseconds
#[async_trait]
pub trait CacherSchema {        
    async fn get_by_process(&mut self, process: &str) -> Option<ProcessCacheEntry>;
    async fn set_by_process(&mut self, process: &str, process_cache: ProcessCacheEntry, ttl: u64); // -> Result<Vec<u8>, SchedulerErrors>;

    async fn get_by_owner(&mut self, owner: &str) -> Option<Scheduler>;    
    async fn set_by_owner(&mut self, owner: &str, url: &str, ttl: u64); // -> Result<Vec<u8>, SchedulerErrors>;
}

#[async_trait]
pub trait LoadSchedulerSchema {
    async fn load_scheduler(&self, wallet_address: &str) -> Result<Scheduler, SchedulerErrors>;
}

#[async_trait]
pub trait LoadProcessSchedulerSchema {
    /// process == process tx id
    async fn load_process_scheduler(&self, process: &str) -> Result<Scheduler, SchedulerErrors>;
}