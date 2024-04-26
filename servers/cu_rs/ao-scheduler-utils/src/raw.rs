use crate::err::SchedulerErrors;
use crate::dal::{CacherSchema, LoadSchedulerSchema};
use async_trait::async_trait;
use std::marker::{Send, Sync};

pub struct Raw<C, LS> {
    load_scheduler: LS,
    cache: C
}

impl<C: CacherSchema, LS: LoadSchedulerSchema> Raw<C, LS> {
    pub fn new(load_scheduler: LS, cache: C) -> Self {
        Self {
            load_scheduler,
            cache
        }
    }
}

#[async_trait]
pub trait RawMaker {
    async fn raw(&mut self, address: &str) -> Result<Option<SchedulerLocation>, SchedulerErrors>;
}

#[async_trait]
impl<C, LS> RawMaker for Raw<C, LS>
where
    C: CacherSchema + Send + Sync,
    LS: LoadSchedulerSchema + Send + Sync {
    /**
    * Return the `Scheduler-Location` record for the address
    * or None, if it cannot be found
    *
    * @param {string} address - the wallet address used by the Scheduler
    * @returns {{ url: string } | None >} whether the wallet address is Scheduler
    */
    async fn raw(&mut self, address: &str) -> Result<Option<SchedulerLocation>, SchedulerErrors> {
        let result = self.cache.get_by_owner(address).await;
        if let Some(result) = result {
            return Ok(Some(SchedulerLocation { url: result.url }));
        }

        match self.load_scheduler.load_scheduler(address).await  {
            Ok(sched) => {
                self.cache.set_by_owner(address, &sched.url, sched.ttl).await;
                Ok(Some(SchedulerLocation { url: sched.url }))
            },
            Err(e) => {
                if let SchedulerErrors::InvalidSchedulerLocationError { name: _, message: _ } = e {
                    return Ok(None);
                }
                Err(e)
            }
        }
    }
}

#[allow(unused)]
pub struct SchedulerLocation {
    url: String
}

#[cfg(test)]
mod tests {
    use crate::dal::{Scheduler, ProcessCacheEntry};    
    use super::*;
    use async_trait::async_trait;

    const SCHEDULER: &str = "gnVg6A6S8lfB10P38V7vOia52lEhTX3Uol8kbTGUT8w";
    const DOMAIN: &str = "https://foo.bar";
    const TEN_MS: u64 = 10;

    mod should_return_the_loan_scheduler_location {
        use super::*;

        struct MockCacheRawFound;
        #[async_trait]
        impl CacherSchema for MockCacheRawFound {
            async fn get_by_owner(&mut self, scheduler: &str) -> Option<Scheduler> {
                assert!(scheduler == SCHEDULER);
                None
            }
            async fn get_by_process(&mut self, _process: &str) -> Option<ProcessCacheEntry> {
                unimplemented!()
            }
            async fn set_by_process(&mut self, _process_tx_id: &str, _value: ProcessCacheEntry, _ttl: u64) { 
                unimplemented!() 
            }        
            async fn set_by_owner(&mut self, owner: &str, url: &str, ttl: u64) {
                assert!(owner == SCHEDULER);
                assert!(url == DOMAIN);
                assert!(ttl == TEN_MS);
            }
        }

        struct MockLoadScheduler;
        #[async_trait]
        impl LoadSchedulerSchema for MockLoadScheduler {
            async fn load_scheduler(&self, scheduler_wallet_address: &str) -> Result<Scheduler, SchedulerErrors>  {
                assert!(scheduler_wallet_address == SCHEDULER);
                Ok(Scheduler {
                    url: DOMAIN.to_string(), ttl: TEN_MS, address: SCHEDULER.to_string()
                })
            }
        }

        #[tokio::test]
        async fn test_raw_found() {
            let mut raw = Raw {
                load_scheduler: MockLoadScheduler,
                cache: MockCacheRawFound
            };
            let result = raw.raw(SCHEDULER).await;
            assert!(result.unwrap().unwrap().url == DOMAIN);
        }
    }

    mod not_found {
        use super::*;

        struct MockCacheRawNotFound;
        #[async_trait]
        impl CacherSchema for MockCacheRawNotFound {
            async fn get_by_owner(&mut self, scheduler: &str) -> Option<Scheduler> {
                assert!(scheduler == SCHEDULER);
                None
            }
            async fn get_by_process(&mut self, _process: &str) -> Option<ProcessCacheEntry> {
                unimplemented!()
            }
            async fn set_by_process(&mut self, _process_tx_id: &str, _value: ProcessCacheEntry, _ttl: u64) { unimplemented!() }    
        
            async fn set_by_owner(&mut self, _owner: &str, _url: &str, _ttl: u64) { unimplemented!("should not call if not scheduler is found") }
        }

        struct MockLoadScheduler;
        #[async_trait]
        impl LoadSchedulerSchema for MockLoadScheduler {
            async fn load_scheduler(&self, scheduler_wallet_address: &str) -> Result<Scheduler, SchedulerErrors>  {
                assert!(scheduler_wallet_address == SCHEDULER);
                Err(SchedulerErrors::new_invalid_scheduler_location("Big womp".to_string()))
            }
        }

        #[tokio::test]
        async fn test_raw_not_found() {
            let mut raw = Raw {
                load_scheduler: MockLoadScheduler,
                cache: MockCacheRawNotFound
            };
            let result = raw.raw(SCHEDULER).await;
            assert!(result.unwrap().is_none());
        }
    }

    mod should_use_the_cache_value {
        use super::*;

        struct MockCacheRawUseCachedValue;
        #[async_trait]
        impl CacherSchema for MockCacheRawUseCachedValue {
            async fn get_by_owner(&mut self, wallet_address: &str) -> Option<Scheduler> {
                assert!(wallet_address == SCHEDULER);
                Some(Scheduler { url: DOMAIN.to_string(), ttl: TEN_MS, address: SCHEDULER.to_string() })
            }
            async fn get_by_process(&mut self, _process: &str) -> Option<ProcessCacheEntry> {
                unimplemented!()
            }
            async fn set_by_process(&mut self, _process_tx_id: &str, _value: ProcessCacheEntry, _ttl: u64) { unimplemented!() }    
        
            async fn set_by_owner(&mut self, _owner: &str, _url: &str, _ttl: u64) { unimplemented!("should not call if not scheduler is in cache") }
        }

        struct MockLoadScheduler;
        #[async_trait]
        impl LoadSchedulerSchema for MockLoadScheduler {
            async fn load_scheduler(&self, _scheduler_wallet_address: &str) -> Result<Scheduler, SchedulerErrors>  {
                panic!("should never call on chain if in cache");
            }
        }

        #[tokio::test]
        async fn test_raw_use_cached_value() {
            let mut raw = Raw {
                load_scheduler: MockLoadScheduler,
                cache: MockCacheRawUseCachedValue
            };
            let result = raw.raw(SCHEDULER).await;
            assert!(result.is_ok());
        }
    }
}