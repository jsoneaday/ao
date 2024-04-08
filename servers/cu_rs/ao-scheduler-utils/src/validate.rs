use crate::{client::{gateway::GatewayMaker, in_memory::Cacher}, err::SchedulerErrors};

/**
   * Validate whether the given wallet address is an ao Scheduler
   *
   * @param {string} address - the wallet address used by the Scheduler
   * @returns {<boolean>} whether the wallet address is Scheduler
   */
pub async fn validate_with<C: Cacher, G: GatewayMaker>(mut cache: C, gateway: &G, gateway_url: &str, address: &str) -> Result<bool, SchedulerErrors> {
    let cached = cache.get_by_owner_with(address).await;
    if let Some(_) = cached {
        return Ok(true);
    }
  
    match gateway.load_scheduler_with(gateway_url, address).await {
        Ok(sched) => {
            cache.set_by_owner_with(address, &sched.url, sched.ttl).await;
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

#[cfg(test)]
mod tests {
    use crate::client::{gateway::SchedulerResult, in_memory::UrlOwner};
    use super::*;
    use async_trait::async_trait;

    const SCHEDULER: &str = "gnVg6A6S8lfB10P38V7vOia52lEhTX3Uol8kbTGUT8w";
    const DOMAIN: &str = "https://foo.bar";
    const TEN_MS: u64 = 10;

    pub struct MockLruCacheForIsValid;
    #[async_trait]
    impl Cacher for MockLruCacheForIsValid {
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
    pub struct MockGatewayForIsValid;
    #[async_trait]
    impl GatewayMaker for MockGatewayForIsValid {
        async fn load_process_scheduler_with<'a>(&self, _gateway_url: &'a str, _process_tx_id: &'a str) -> Result<SchedulerResult, SchedulerErrors> {
            unimplemented!()
        }
        async fn load_scheduler_with<'a>(&self, _gateway_url: &'a str, scheduler_wallet_address: &'a str) -> Result<SchedulerResult, SchedulerErrors>  {
            assert!(scheduler_wallet_address == SCHEDULER);
            Ok(SchedulerResult {
                url: DOMAIN.to_string(), ttl: TEN_MS, owner: SCHEDULER.to_string()
            })
        }
    }

    #[tokio::test]
    async fn test_validate_with_is_valid() {
        let result = validate_with(MockLruCacheForIsValid, &MockGatewayForIsValid, "", SCHEDULER).await;
        assert!(result.is_ok());
    }

    pub struct MockGatewayForIsNotValid;
    #[async_trait]
    impl GatewayMaker for MockGatewayForIsNotValid {
        async fn load_process_scheduler_with<'a>(&self, _gateway_url: &'a str, _process_tx_id: &'a str) -> Result<SchedulerResult, SchedulerErrors> {
            unimplemented!()
        }
        async fn load_scheduler_with<'a>(&self, _gateway_url: &'a str, scheduler_wallet_address: &'a str) -> Result<SchedulerResult, SchedulerErrors>  {
            assert!(scheduler_wallet_address == SCHEDULER);
            Err(SchedulerErrors::new_invalid_scheduler_location("Big womp".to_string()))
        }
    }

    #[tokio::test]
    async fn test_validate_with_is_not_valid() {
        let result = validate_with(MockLruCacheForIsValid, &MockGatewayForIsNotValid, "", SCHEDULER).await;
        assert!(result.ok().unwrap() == false);
    }

    pub struct MockLruCacheForIsFromCache;
    #[async_trait]
    impl Cacher for MockLruCacheForIsFromCache {
        async fn get_by_owner_with(&mut self, key: &str) -> Option<UrlOwner> {
            assert!(key == SCHEDULER);
            Some(UrlOwner { url: DOMAIN.to_string(), address: SCHEDULER.to_string() })
        }
        async fn get_by_process_with(&mut self, _process: &str) -> Option<UrlOwner> {
            unimplemented!()
        }   
        async fn set_by_process_with(&mut self, _process_tx_id: &str, _value: UrlOwner, _ttl: u64) { unimplemented!() }    
    
        async fn set_by_owner_with(&mut self, _owner: &str, _url: &str, _ttl: u64) {
            unimplemented!()
        }
    }
    pub struct MockGatewayForIsFromCache;
    #[async_trait]
    impl GatewayMaker for MockGatewayForIsFromCache {
        async fn load_process_scheduler_with<'a>(&self, _gateway_url: &'a str, _process_tx_id: &'a str) -> Result<SchedulerResult, SchedulerErrors> {
            unimplemented!()
        }
        async fn load_scheduler_with<'a>(&self, _gateway_url: &'a str, _scheduler_wallet_address: &'a str) -> Result<SchedulerResult, SchedulerErrors>  {
            panic!("should never call on chain if in cache")
        }
    }

    #[tokio::test]
    async fn test_validate_with_is_from_cache() {
        let result = validate_with(MockLruCacheForIsFromCache, &MockGatewayForIsFromCache, "", SCHEDULER).await;
        assert!(result.ok().unwrap() == true);
    }
}