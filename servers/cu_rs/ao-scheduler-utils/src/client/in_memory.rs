use std::time::Duration;
use async_trait::async_trait;
use moka::{future::Cache, Expiry};
use crate::dal::{CacherSchema, ProcessCacheEntry, Scheduler};

/// moka is internally thread safe, but requires cache to be cloned
#[derive(Clone)]
#[allow(unused)]
pub struct LocalLruCache {
    process_cache: Cache<String, (Expiration, ProcessCacheEntry)>,
    owner_cache: Cache<String, (Expiration, Scheduler)>
}

impl LocalLruCache {
    pub fn new(size: u64) -> Self {
        Self {
            process_cache: Cache::builder()
                .max_capacity(size)
                .expire_after(ProcessCacheExpiry)
                .build(),
            owner_cache: Cache::builder()
                .max_capacity(size)
                .expire_after(OwnerCacheExpiry)
                .build()
        }
    }
}

#[async_trait]
impl CacherSchema for LocalLruCache {    
    async fn get_by_process(&mut self, process: &str) -> Option<ProcessCacheEntry> {
        let result = self.process_cache.get(process).await;
        if let Some(result) = result {
            return Some(result.1);
        }
        None
    }

    async fn set_by_process(&mut self, process: &str, process_cache: ProcessCacheEntry, ttl: u64) {
        self.process_cache.get_with(process.to_string(), async {(
            get_expiration_from_ms(ttl), 
            process_cache
        )}).await;
    }   

    async fn get_by_owner(&mut self, owner: &str) -> Option<Scheduler> {
        let result = self.owner_cache.get(owner).await;
        if let Some(result) = result {
            return Some(result.1);
        }
        None
    }     

    async fn set_by_owner(&mut self, owner: &str, url: &str, ttl: u64) {
        self.owner_cache.get_with(owner.to_string(), async {(
            get_expiration_from_ms(ttl), 
            Scheduler { url: url.to_string(), ttl: ttl, address: owner.to_string() }
        )}).await;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Expiration {
    /// The value never expires.
    Never,
    OneSecond,
    FiveSeconds,
    TenSeconds,
}

impl Expiration {
    pub fn as_duration(&self) -> Option<Duration> {
        match self {
            Expiration::Never => None,
            Expiration::OneSecond => Some(Duration::from_secs(1)),
            Expiration::FiveSeconds => Some(Duration::from_secs(5)),
            Expiration::TenSeconds => Some(Duration::from_secs(10))
        }
    }
}

pub struct ProcessCacheExpiry;
impl Expiry<String, (Expiration, ProcessCacheEntry)> for ProcessCacheExpiry {
    fn expire_after_create(&self, _key: &String, value: &(Expiration, ProcessCacheEntry), _created_at: std::time::Instant) -> Option<Duration> {
        let duration = value.0.as_duration();
        duration
    }
}

pub struct OwnerCacheExpiry;
impl Expiry<String, (Expiration, Scheduler)> for OwnerCacheExpiry {
    fn expire_after_create(&self, _key: &String, value: &(Expiration, Scheduler), _created_at: std::time::Instant) -> Option<Duration> {
        let duration = value.0.as_duration();
        duration
    }
}

pub fn get_expiration_from_ms(ttl: u64) -> Expiration {
    if ttl == 0 {
        Expiration::Never
    } else if ttl / 1000 <= 1 { 
        Expiration::OneSecond 
    } else if ttl / 1000 <= 5 { 
        Expiration::FiveSeconds 
    } else { 
        Expiration::TenSeconds 
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROCESS: &str = "zc24Wpv_i6NNCEdxeKt7dcNrqL5w0hrShtSCcFGGL24";
    const SCHEDULER: &str = "gnVg6A6S8lfB10P38V7vOia52lEhTX3Uol8kbTGUT8w";
    const DOMAIN: &str = "https://foo.bar";
    const SIZE: u64 = 10;
    const TEN_MS: u64 = 10;

    #[tokio::test]
    async fn test_get_by_process() {
        let mut cache = LocalLruCache::new(SIZE);
        let process_cache = cache.clone().process_cache;
        process_cache.insert(PROCESS.to_string(), (get_expiration_from_ms(TEN_MS), ProcessCacheEntry { url: DOMAIN.to_string(), ttl: None, address: SCHEDULER.to_string() })).await;

        let result = cache.get_by_process(PROCESS).await;
        assert!(result.clone().unwrap().url == DOMAIN.to_string() && result.unwrap().address == SCHEDULER.to_string());
    }

    #[tokio::test]
    async fn test_get_by_owner() {
        let mut cache = LocalLruCache::new(SIZE);
        let owner_cache = cache.clone().owner_cache;
        owner_cache.insert(SCHEDULER.to_string(), (get_expiration_from_ms(TEN_MS), Scheduler { url: DOMAIN.to_string(), ttl: TEN_MS, address: SCHEDULER.to_string() })).await;

        let result = cache.get_by_owner(SCHEDULER).await;
        assert!(result.clone().unwrap().url == DOMAIN.to_string() && result.unwrap().address == SCHEDULER.to_string());
    }

    #[tokio::test]
    async fn test_set_by_process() {
        let mut cache = LocalLruCache::new(SIZE);    

        cache.clone().set_by_process(PROCESS, ProcessCacheEntry { url: DOMAIN.to_string(), ttl: None, address: SCHEDULER.to_string() }, TEN_MS).await;

        assert!(cache.get_by_process(PROCESS).await.is_some());
    }

    #[tokio::test]
    async fn test_set_by_owner() {
        let mut cache = LocalLruCache::new(SIZE);

        cache.clone().set_by_owner(SCHEDULER, DOMAIN, TEN_MS).await;

        assert!(cache.get_by_owner(SCHEDULER).await.is_some());
    }
}