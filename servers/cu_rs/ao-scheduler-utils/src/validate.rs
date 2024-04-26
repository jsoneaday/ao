use crate::err::SchedulerErrors;
use crate::dal::{CacherSchema, LoadSchedulerSchema};
use async_trait::async_trait;
use std::marker::{Send, Sync};

pub struct Validate<C: CacherSchema, LS: LoadSchedulerSchema> {
    loader: LS,
    cache: C
}

impl<C: CacherSchema, LS: LoadSchedulerSchema> Validate<C, LS> {
    pub fn new(loader: LS, cache: C) -> Self {
        Self {
            loader,
            cache
        }
    }
}

#[async_trait]
pub trait ValidateMaker {
    async fn validate(&mut self, address: &str) -> Result<bool, SchedulerErrors>;
}

#[async_trait]
impl<C, LS> ValidateMaker for Validate<C, LS>
where 
    C: CacherSchema + Send + Sync,
    LS: LoadSchedulerSchema + Send + Sync {
    /**
   * Validate whether the given wallet address is an ao Scheduler
   *
   * @param {string} address - the wallet address used by the Scheduler
   * @returns {<boolean>} whether the wallet address is Scheduler
   */
    async fn validate(&mut self, address: &str) -> Result<bool, SchedulerErrors> {
        let cached = self.cache.get_by_owner(address).await;
        if let Some(_) = cached {
            return Ok(true);
        }
    
        match self.loader.load_scheduler(address).await {
            Ok(sched) => {
                self.cache.set_by_owner(address, &sched.url, sched.ttl).await;
                return Ok(true);
            },
            Err(e) => {
                if let SchedulerErrors::InvalidSchedulerLocationError { name: _, message: _ } = e {
                    return Ok(false);
                }
                Err(e)
            }
        }
    }        
}

#[cfg(test)]
mod tests {
    use crate::dal::{Scheduler, ProcessCacheEntry};
    use super::*;
    use async_trait::async_trait;

    const SCHEDULER: &str = "gnVg6A6S8lfB10P38V7vOia52lEhTX3Uol8kbTGUT8w";
    const DOMAIN: &str = "https://foo.bar";
    const TEN_MS: u64 = 10;

    mod should_validate_whether_the_wallet_address_owns_a_valid_scheduler_location {
        use super::*;

        pub struct MockLruCacheForIsValid;
        #[async_trait]
        impl CacherSchema for MockLruCacheForIsValid {
            async fn get_by_owner(&mut self, scheduler: &str) -> Option<Scheduler> {
                assert!(scheduler == SCHEDULER);
                None
            }    
            async fn get_by_process(&mut self, _process: &str) -> Option<ProcessCacheEntry> {
                unimplemented!()
            }   
            async fn set_by_process(&mut self, _process_tx_id: &str, _value: ProcessCacheEntry, _ttl: u64) { unimplemented!() }    
        
            async fn set_by_owner(&mut self, owner: &str, url: &str, ttl: u64) {
                assert!(owner == SCHEDULER);
                assert!(url == DOMAIN);
                assert!(ttl == TEN_MS);
            }
        }

        mod valid {
            use super::*;
            
            pub struct MockLoadScheduler;
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
            async fn test_validate_with_is_valid() {
                let mut validater = Validate {
                    loader: MockLoadScheduler,
                    cache: MockLruCacheForIsValid
                };
                let result = validater.validate(SCHEDULER).await;
                assert!(result.is_ok());
            }
        }

        mod not_valid {
            use super::*;
            pub struct MockLoadScheduler;
            #[async_trait]
            impl LoadSchedulerSchema for MockLoadScheduler {
                async fn load_scheduler(&self, scheduler_wallet_address: &str) -> Result<Scheduler, SchedulerErrors>  {
                    assert!(scheduler_wallet_address == SCHEDULER);
                    Err(SchedulerErrors::new_invalid_scheduler_location("Big womp".to_string()))
                }
            }

            #[tokio::test]
            async fn test_validate_with_is_not_valid() {
                let mut validater = Validate {
                    loader: MockLoadScheduler,
                    cache: MockLruCacheForIsValid
                };
                let result = validater.validate(SCHEDULER).await;
                assert!(result.ok().unwrap() == false);
            }
        }

        mod should_use_the_cached_value {
            use super::*;

            pub struct MockLruCacheForIsFromCache;
            #[async_trait]
            impl CacherSchema for MockLruCacheForIsFromCache {
                async fn get_by_owner(&mut self, key: &str) -> Option<Scheduler> {
                    assert!(key == SCHEDULER);
                    Some(Scheduler { url: DOMAIN.to_string(), ttl: TEN_MS, address: SCHEDULER.to_string() })
                }
                async fn get_by_process(&mut self, _process: &str) -> Option<ProcessCacheEntry> {
                    unimplemented!()
                }   
                async fn set_by_process(&mut self, _process_tx_id: &str, _value: ProcessCacheEntry, _ttl: u64) { unimplemented!() }    
            
                async fn set_by_owner(&mut self, _owner: &str, _url: &str, _ttl: u64) {
                    unimplemented!("should not call if not scheduler is in cache")
                }
            }
            pub struct MockLoadScheduler;
            #[async_trait]
            impl LoadSchedulerSchema for MockLoadScheduler {
                async fn load_scheduler(&self, _scheduler_wallet_address: &str) -> Result<Scheduler, SchedulerErrors>  {
                    panic!("should never call on chain if in cache")
                }
            }

            #[tokio::test]
            async fn test_validate_with_is_from_cache() {
                let mut validater = Validate {
                    loader: MockLoadScheduler,
                    cache: MockLruCacheForIsFromCache
                };
                let result = validater.validate(SCHEDULER).await;
                assert!(result.ok().unwrap() == true);
            }
        }
    }
}