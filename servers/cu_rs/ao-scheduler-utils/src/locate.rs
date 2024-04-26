use crate::err::SchedulerErrors;
use crate::dal::{CacherSchema, CheckForRedirectSchema, LoadProcessSchedulerSchema, LoadSchedulerSchema, ProcessCacheEntry, Scheduler};
use async_trait::async_trait;
use std::marker::{Send, Sync};

pub struct Locate<LP: LoadProcessSchedulerSchema, LS: LoadSchedulerSchema, C: CacherSchema, R: CheckForRedirectSchema> {
  load_process_scheduler_loader: LP,
  load_scheduler_loader: LS,
  cache: C,
  follow_redirects: bool,
  check_for_redirect: R
}

impl<LP: LoadProcessSchedulerSchema, LS: LoadSchedulerSchema, C: CacherSchema, R: CheckForRedirectSchema> Locate<LP, LS, C, R> {
  pub fn new(load_process_scheduler_loader: LP, load_scheduler_loader: LS, cache: C, follow_redirects: bool, check_for_redirect: R) -> Self {
    Locate {
      load_process_scheduler_loader,
      load_scheduler_loader,
      cache,
      follow_redirects,
      check_for_redirect
    }
  }
}

#[async_trait]
pub trait LocateMaker {
  async fn locate(&mut self, process: &str, scheduler_hint: Option<&str>) -> Result<ProcessCacheEntry, SchedulerErrors>;
}

#[async_trait]
impl<LP, LS, C, R>  LocateMaker for Locate<LP, LS, C, R>
where
    LP: LoadProcessSchedulerSchema + Send + Sync,
    LS: LoadSchedulerSchema + Send + Sync,
    C: CacherSchema + Send + Sync,
    R: CheckForRedirectSchema + Send + Sync {
  /**
   * Locate the scheduler for the given process.
   *
   * Later on, this implementation could encompass the automatic swapping
   * of decentralized sequencers
   *
   * @param {string} process - the id of the process
   * @param {string} [schedulerHint] - the id of owner of the scheduler, which prevents having to query the process
   * from a gateway, and instead skips to querying Scheduler-Location
   * @returns { url: string, address: string } - an object whose url field is the Scheduler Location
   */  
  async fn locate(&mut self, process: &str, scheduler_hint: Option<&str>) -> Result<ProcessCacheEntry, SchedulerErrors> {
      if let Some(cached) = self.cache.get_by_process(process).await { 
        return Ok(cached);
      }
      
      // If the scheduler hint was provided,
      // so skip querying the process and instead
      // query the Scheduler-Location record directly
      #[allow(unused)]
      let mut scheduler: Option<Scheduler> = None;   
      if scheduler_hint.is_some() {
        if let Some(by_owner) = self.cache.get_by_owner(scheduler_hint.unwrap()).await {
          scheduler = Some(Scheduler { url: by_owner.url, ttl: by_owner.ttl, address: by_owner.address });
        } else {
          match self.load_scheduler_loader.load_scheduler(scheduler_hint.unwrap()).await {
            Ok(sched) => {
              self.cache.set_by_owner(&sched.address, &sched.url, sched.ttl).await;
              scheduler = Some(sched);
            },
            Err(e) => return Err(e)
          }
        }
      } else {
        match self.load_process_scheduler_loader.load_process_scheduler(process).await {
          Ok(sched) => scheduler = Some(sched),
          Err(e) => return Err(e)
        }
      }

      let scheduler = scheduler.unwrap();
      let mut final_url = scheduler.url.clone();
      if self.follow_redirects {        
        match self.check_for_redirect.check_for_redirect(&scheduler.url, process).await {
          Ok(url) => {
            final_url = url;
            println!("final_url: {:?}", final_url);
          },
          Err(e) => return Err(e)
        };
      }

      let by_process = ProcessCacheEntry { url: final_url, ttl: None, address: scheduler.address };
      self.cache.set_by_process(process, by_process.clone(), scheduler.ttl).await;
      return Ok(by_process);
  }
}


  #[cfg(test)]
  mod tests {
    use async_trait::async_trait;
    use crate::dal::ProcessCacheEntry;
    use super::*;

    const PROCESS: &str = "zc24Wpv_i6NNCEdxeKt7dcNrqL5w0hrShtSCcFGGL24";
    const SCHEDULER: &str = "gnVg6A6S8lfB10P38V7vOia52lEhTX3Uol8kbTGUT8w";
    const DOMAIN: &str = "https://foo.bar";
    const DOMAIN_REDIRECT: &str = "https://foo-redirect.bar";
    const TEN_MS: u64 = 10;
    
    mod should_load_the_value_and_cache_it {
      use super::*;

      struct MockLoadProcessScheduler;
      #[async_trait]
      impl LoadProcessSchedulerSchema for MockLoadProcessScheduler {
        async fn load_process_scheduler(&self, process: &str) -> Result<Scheduler, SchedulerErrors>  {
            assert!(process == PROCESS);
            Ok(Scheduler { url: DOMAIN.to_string(), ttl: TEN_MS, address: SCHEDULER.to_string() })
        }
      }

      struct MockLoadScheduler;
      #[async_trait]
      impl LoadSchedulerSchema for MockLoadScheduler {
        async fn load_scheduler(&self, _wallet_address: &str) -> Result<Scheduler, SchedulerErrors>  {
            Err(SchedulerErrors::new_invalid_scheduler_location("should not load the scheduler if no hint".to_string()))
        }
      }

      struct MockCache;
      #[async_trait]
      impl CacherSchema for MockCache {
        async fn get_by_process(&mut self, process: &str) -> Option<ProcessCacheEntry> {
          assert!(process == PROCESS);
          None
        }   
        async fn get_by_owner(&mut self, _owner: &str) -> Option<Scheduler> {
          unimplemented!("should not get by owner, if no scheduler hint")
        }    
        async fn set_by_process(&mut self, process_tx_id: &str, value: ProcessCacheEntry, ttl: u64) { 
          assert!(process_tx_id == PROCESS);
          assert!(value.url == DOMAIN);
          assert!(value.address == SCHEDULER);
          assert!(ttl == TEN_MS);
        }    
        async fn set_by_owner(&mut self, owner: &str, url: &str, ttl: u64) {
            assert!(owner == SCHEDULER);
            assert!(url == DOMAIN);
            assert!(ttl == TEN_MS);
        }
      }

      struct MockCheckForRedirect;
      #[async_trait]
      impl CheckForRedirectSchema for MockCheckForRedirect {
        async fn check_for_redirect(&self, _url: &str, _process: &str) -> Result<String, SchedulerErrors> {
          unimplemented!("should not check for redirect if followRedirects is false")
        }
      }

      #[tokio::test]
      async fn test_location_load_and_cache() {
        let mut locate = Locate {
          load_process_scheduler_loader: MockLoadProcessScheduler,
          load_scheduler_loader: MockLoadScheduler,
          cache: MockCache,
          follow_redirects: false,
          check_for_redirect: MockCheckForRedirect
        };
        let result = locate.locate(PROCESS, None).await;
        let result = result.unwrap();
        assert!(result.url == DOMAIN && result.address == SCHEDULER);
      }
    }

    mod should_serve_the_cached_value {
      use super::*;

      struct MockLoadProcessScheduler;
      #[async_trait]
      impl LoadProcessSchedulerSchema for MockLoadProcessScheduler {
        async fn load_process_scheduler(&self, _process: &str) -> Result<Scheduler, SchedulerErrors>  {
          unimplemented!("should never call on chain if in cache")
        }
      }

      struct MockLoadScheduler;
      #[async_trait]
      impl LoadSchedulerSchema for MockLoadScheduler {
        async fn load_scheduler(&self, _process: &str) -> Result<Scheduler, SchedulerErrors>  {
          unimplemented!("should not load the scheduler if no hint")
        }
      }

      struct MockCache;
      #[async_trait]
      impl CacherSchema for MockCache {
        async fn get_by_process(&mut self, process: &str) -> Option<ProcessCacheEntry> {
          assert!(process == PROCESS);
          Some(ProcessCacheEntry { url: DOMAIN.to_string(), ttl: None, address: SCHEDULER.to_string() })
        }    
        async fn get_by_owner(&mut self, _owner: &str) -> Option<Scheduler> {
          panic!("should not check cache by owner if cached by process");
        }    
        async fn set_by_process(&mut self, _process_tx_id: &str, _value: ProcessCacheEntry, _ttl: u64) { 
          panic!("should not set cache by process if cached by process");
        }    
        async fn set_by_owner(&mut self, _owner: &str, _url: &str, _ttl: u64) {
          panic!("should not set cache by owner if cached by process");
        }
      }

      struct MockCheckForRedirect;
      #[async_trait]
      impl CheckForRedirectSchema for MockCheckForRedirect {
        async fn check_for_redirect(&self, _url: &str, _process: &str) -> Result<String, SchedulerErrors> {
          unimplemented!("should not check for redirect if followRedirects is false")
        }
      }

      #[tokio::test]
      async fn test_location_serve_cached_value() {
        let mut locate = Locate {
          load_process_scheduler_loader: MockLoadProcessScheduler,
          load_scheduler_loader: MockLoadScheduler,
          cache: MockCache,
          follow_redirects: false,
          check_for_redirect: MockCheckForRedirect
        };
        let result = locate.locate(PROCESS, None).await;
        let result = result.unwrap();
        assert!(result.url == DOMAIN && result.address == SCHEDULER);
      }
    }

    mod should_load_the_redirected_value_and_cache_it {
      use super::*;

      struct MockLoadProcessScheduler;
      #[async_trait]
      impl LoadProcessSchedulerSchema for MockLoadProcessScheduler {
        async fn load_process_scheduler(&self, process: &str) -> Result<Scheduler, SchedulerErrors>  {
            assert!(process == PROCESS);
            Ok(Scheduler { url: DOMAIN.to_string(), ttl: TEN_MS, address: SCHEDULER.to_string() })
        }
      }

      struct MockLoadScheduler;
      #[async_trait]
      impl LoadSchedulerSchema for MockLoadScheduler {
        async fn load_scheduler(&self, _process: &str) -> Result<Scheduler, SchedulerErrors>  {
            unimplemented!("should not load the scheduler if no hint")
        }
      }

      struct MockCache;
      #[async_trait]
      impl CacherSchema for MockCache {
        async fn get_by_process(&mut self, process: &str) -> Option<ProcessCacheEntry> {
          assert!(process == PROCESS);
          None
        }    
        async fn get_by_owner(&mut self, _owner: &str) -> Option<Scheduler> {
          unimplemented!("should not get by owner, if no scheduler hint")
        }    
        async fn set_by_process(&mut self, process_tx_id: &str, value: ProcessCacheEntry, ttl: u64) { 
          assert!(process_tx_id == PROCESS);
          assert!(value.url == DOMAIN_REDIRECT);
          assert!(value.address == SCHEDULER);
          assert!(ttl == TEN_MS);
        }    
        async fn set_by_owner(&mut self, owner: &str, url: &str, ttl: u64) {
          assert!(owner == SCHEDULER);          
          // Original DOMAIN not the redirect          
          assert!(url == DOMAIN);
          assert!(ttl == TEN_MS);
        }
      }

      struct MockCheckForRedirect;
      #[async_trait]
      impl CheckForRedirectSchema for MockCheckForRedirect {
        async fn check_for_redirect(&self, url: &str, process: &str) -> Result<String, SchedulerErrors> {
          assert!(process == PROCESS);
          assert!(url == DOMAIN);
          Ok(DOMAIN_REDIRECT.to_string())
        }
      }

      #[tokio::test]
      async fn test_location_load_redirected_and_cache_it() {
        let mut locate = Locate {
          load_process_scheduler_loader: MockLoadProcessScheduler,
          load_scheduler_loader: MockLoadScheduler,
          cache: MockCache,
          follow_redirects: true,
          check_for_redirect: MockCheckForRedirect
        };
        let result = locate.locate(PROCESS, None).await;
        let result = result.unwrap();
        assert!(result.url == DOMAIN_REDIRECT && result.address == SCHEDULER);
      }
    }

    mod should_use_the_scheduler_hint_and_skip_querying_for_the_process {
      use super::*;

      struct MockLoadProcessScheduler;
      #[async_trait]
      impl LoadProcessSchedulerSchema for MockLoadProcessScheduler {
        async fn load_process_scheduler(&self, _process: &str) -> Result<Scheduler, SchedulerErrors>  {
            unimplemented!("should not load process if given a scheduler hint")
        }
      }

      struct MockLoadScheduler;
      #[async_trait]
      impl LoadSchedulerSchema for MockLoadScheduler {
        async fn load_scheduler(&self, owner: &str) -> Result<Scheduler, SchedulerErrors>  {
          assert!(owner == SCHEDULER);
          Ok(Scheduler { url: DOMAIN.to_string(), ttl: TEN_MS, address: SCHEDULER.to_string() })
        }
      }

      struct MockCache;
      #[async_trait]
      impl CacherSchema for MockCache {
        async fn get_by_process(&mut self, process: &str) -> Option<ProcessCacheEntry> {
          assert!(process == PROCESS);
          None
        }    
        async fn get_by_owner(&mut self, owner: &str) -> Option<Scheduler> {
          assert!(owner == SCHEDULER);
          None
        }    
        async fn set_by_process(&mut self, process_tx_id: &str, value: ProcessCacheEntry, ttl: u64) { 
          assert!(process_tx_id == PROCESS);
          assert!(value.url == DOMAIN_REDIRECT);
          assert!(value.address == SCHEDULER);
          assert!(ttl == TEN_MS);
        }    
        async fn set_by_owner(&mut self, owner: &str, url: &str, ttl: u64) {
          assert!(owner == SCHEDULER);          
          // Original DOMAIN not the redirect
          assert!(url == DOMAIN);
          assert!(ttl == TEN_MS);
        }
      }

      struct MockCheckForRedirect;
      #[async_trait]
      impl CheckForRedirectSchema for MockCheckForRedirect {
        async fn check_for_redirect(&self, url: &str, process: &str) -> Result<String, SchedulerErrors> {
          assert!(process == PROCESS);
          assert!(url == DOMAIN);
          Ok(DOMAIN_REDIRECT.to_string())
        }
      }

      #[tokio::test]
      async fn test_location_use_scheduler_hint_and_skip_querying_process() {
        let mut locate = Locate {
          load_process_scheduler_loader: MockLoadProcessScheduler,
          load_scheduler_loader: MockLoadScheduler,
          cache: MockCache,
          follow_redirects: true,
          check_for_redirect: MockCheckForRedirect
        };
        let result = locate.locate(
          PROCESS, 
          Some(SCHEDULER)
        ).await;
        let result = result.unwrap();
        assert!(result.url == DOMAIN_REDIRECT && result.address == SCHEDULER);
      }
    }

    mod should_use_the_scheduler_hint_and_use_the_cached_owner {
      use super::*;

      struct MockLoadProcessScheduler;
      #[async_trait]
      impl LoadProcessSchedulerSchema for MockLoadProcessScheduler {
        async fn load_process_scheduler(&self, _scheduler_wallet_address: &str) -> Result<Scheduler, SchedulerErrors>  {
            unimplemented!("should not load process if given a scheduler hint");
        }
      }

      struct MockLoadScheduler;
      #[async_trait]
      impl LoadSchedulerSchema for MockLoadScheduler {
        async fn load_scheduler(&self, _scheduler_wallet_address: &str) -> Result<Scheduler, SchedulerErrors>  {
            unimplemented!("should not load the scheduler if cached");
        }
      }

      struct MockCache;
      #[async_trait]
      impl CacherSchema for MockCache {
        async fn get_by_process(&mut self, process: &str) -> Option<ProcessCacheEntry> {
          assert!(process == PROCESS);
          None
        }    
        async fn get_by_owner(&mut self, owner: &str) -> Option<Scheduler> {
          assert!(owner == SCHEDULER);
          Some(Scheduler { url: DOMAIN.to_string(), ttl: TEN_MS, address: SCHEDULER.to_string() })
        }    
        async fn set_by_process(&mut self, process: &str, value: ProcessCacheEntry, ttl: u64) { 
          assert!(process == PROCESS);
          assert!(value.url == DOMAIN_REDIRECT);
          assert!(value.address == SCHEDULER);
          assert!(ttl == TEN_MS);
        }    
        async fn set_by_owner(&mut self, _owner: &str, _url: &str, _ttl: u64) {
          panic!("should not cache by owner if cached");
        }
      }

      struct MockCheckForRedirect;
      #[async_trait]
      impl CheckForRedirectSchema for MockCheckForRedirect {
        async fn check_for_redirect(&self, url: &str, process: &str) -> Result<String, SchedulerErrors> {
          assert!(process == PROCESS);
          assert!(url == DOMAIN);
          
          Ok(DOMAIN_REDIRECT.to_string())
        }
      }

      #[tokio::test]
      async fn test_should_use_the_scheduler_hint_and_use_the_cached_owner() {
        let mut locater = Locate {
          load_process_scheduler_loader: MockLoadProcessScheduler,
          load_scheduler_loader: MockLoadScheduler,
          cache: MockCache,
          follow_redirects: true,
          check_for_redirect: MockCheckForRedirect
        };
        let result = locater.locate(
          PROCESS, 
          Some(SCHEDULER)
        ).await;
        let result = result.unwrap();
        assert!(result.url == DOMAIN_REDIRECT && result.address == SCHEDULER);
      }
    }
  }