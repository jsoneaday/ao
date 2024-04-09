use crate::{client::{gateway::GatewayMaker, in_memory::Cacher}, err::SchedulerErrors};
use async_trait::async_trait;

pub struct Raw<C, G> {
    gateway: G,
    cache: C
}

#[async_trait]
pub trait RawMaker {
    async fn raw(&mut self, address: &str) -> Result<Option<SchedulerLocation>, SchedulerErrors>;
}

#[async_trait]
impl<C, G> RawMaker for Raw<C, G>
where
    C: Cacher + std::marker::Send + std::marker::Sync,
    G: GatewayMaker + std::marker::Send + std::marker::Sync {
    /**
    * Return the `Scheduler-Location` record for the address
    * or None, if it cannot be found
    *
    * @param {string} address - the wallet address used by the Scheduler
    * @returns {{ url: string } | None >} whether the wallet address is Scheduler
    */
    async fn raw(&mut self, address: &str) -> Result<Option<SchedulerLocation>, SchedulerErrors> {
        let result = self.cache.get_by_owner_with(address).await;
        if let Some(result) = result {
            return Ok(Some(SchedulerLocation { url: result.url }))
        }

        match self.gateway.load_scheduler(address).await  {
            Ok(sched) => {
                self.cache.set_by_owner_with(address, &sched.url, sched.ttl).await;
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
    use crate::client::in_memory::UrlOwner;
    use crate::client::gateway::SchedulerResult;
    use super::*;
    use async_trait::async_trait;

    const SCHEDULER: &str = "gnVg6A6S8lfB10P38V7vOia52lEhTX3Uol8kbTGUT8w";
    const DOMAIN: &str = "https://foo.bar";
    const TEN_MS: u64 = 10;

    struct MockCacheRawFound;
    #[async_trait]
    impl Cacher for MockCacheRawFound {
        async fn get_by_owner_with(&mut self, scheduler: &str) -> Option<UrlOwner> {
            assert!(scheduler == SCHEDULER);
            None
        }
        async fn get_by_process_with(&mut self, _process: &str) -> Option<UrlOwner> {
            unimplemented!()
        }
        async fn set_by_process_with(&mut self, _process_tx_id: &str, _value: UrlOwner, _ttl: u64) { unimplemented!() }    
    
        async fn set_by_owner_with(&mut self, owner: &str, url: &str, ttl: u64) {
            assert!(owner == SCHEDULER);
            assert!(url == DOMAIN);
            assert!(ttl == TEN_MS);
        }
    }

    struct MockGatewayRawFound;
    #[async_trait]
    impl GatewayMaker for MockGatewayRawFound {
        async fn load_process_scheduler<'a>(&self, _process_tx_id: &'a str) -> Result<SchedulerResult, SchedulerErrors> {
            unimplemented!()
        }
        async fn load_scheduler<'a>(&self, scheduler_wallet_address: &'a str) -> Result<SchedulerResult, SchedulerErrors>  {
            assert!(scheduler_wallet_address == SCHEDULER);
            Ok(SchedulerResult {
                url: DOMAIN.to_string(), ttl: TEN_MS, owner: SCHEDULER.to_string()
            })
        }
    }

    #[tokio::test]
    async fn test_raw_found() {
        let mut raw = Raw {
            gateway: MockGatewayRawFound,
            cache: MockCacheRawFound
        };
        let result = raw.raw(SCHEDULER).await;
        assert!(result.unwrap().unwrap().url == DOMAIN);
    }

    struct MockCacheRawNotFound;
    #[async_trait]
    impl Cacher for MockCacheRawNotFound {
        async fn get_by_owner_with(&mut self, scheduler: &str) -> Option<UrlOwner> {
            assert!(scheduler == SCHEDULER);
            None
        }
        async fn get_by_process_with(&mut self, _process: &str) -> Option<UrlOwner> {
            unimplemented!()
        }
        async fn set_by_process_with(&mut self, _process_tx_id: &str, _value: UrlOwner, _ttl: u64) { unimplemented!() }    
    
        async fn set_by_owner_with(&mut self, _owner: &str, _url: &str, _ttl: u64) { unimplemented!() }
    }

    struct MockGatewayRawNotFound;
    #[async_trait]
    impl GatewayMaker for MockGatewayRawNotFound {
        async fn load_process_scheduler<'a>(&self, _process_tx_id: &'a str) -> Result<SchedulerResult, SchedulerErrors> {
            unimplemented!()
        }
        async fn load_scheduler<'a>(&self, scheduler_wallet_address: &'a str) -> Result<SchedulerResult, SchedulerErrors>  {
            assert!(scheduler_wallet_address == SCHEDULER);
            Err(SchedulerErrors::new_invalid_scheduler_location("Big womp".to_string()))
        }
    }

    #[tokio::test]
    async fn test_raw_not_found() {
        let mut raw = Raw {
            gateway: MockGatewayRawNotFound,
            cache: MockCacheRawNotFound
        };
        let result = raw.raw(SCHEDULER).await;
        assert!(result.unwrap().is_none());
    }

    struct MockCacheRawUseCachedValue;
    #[async_trait]
    impl Cacher for MockCacheRawUseCachedValue {
        async fn get_by_owner_with(&mut self, wallet_address: &str) -> Option<UrlOwner> {
            assert!(wallet_address == SCHEDULER);
            Some(UrlOwner { url: DOMAIN.to_string(), address: SCHEDULER.to_string() })
        }
        async fn get_by_process_with(&mut self, _process: &str) -> Option<UrlOwner> {
            unimplemented!()
        }
        async fn set_by_process_with(&mut self, _process_tx_id: &str, _value: UrlOwner, _ttl: u64) { unimplemented!() }    
    
        async fn set_by_owner_with(&mut self, _owner: &str, _url: &str, _ttl: u64) { unimplemented!() }
    }

    struct MockGatewayRawUseCachedValue;
    #[async_trait]
    impl GatewayMaker for MockGatewayRawUseCachedValue {
        async fn load_process_scheduler<'a>(&self, _process_tx_id: &'a str) -> Result<SchedulerResult, SchedulerErrors> {
            unimplemented!()
        }
        async fn load_scheduler<'a>(&self, _scheduler_wallet_address: &'a str) -> Result<SchedulerResult, SchedulerErrors>  {
            panic!("should never call on chain if in cache");
        }
    }

    #[tokio::test]
    async fn test_raw_use_cached_value() {
        let mut raw = Raw {
            gateway: MockGatewayRawNotFound,
            cache: MockCacheRawNotFound
        };
        let result = raw.raw(SCHEDULER).await;
        assert!(result.is_ok());
    }
}