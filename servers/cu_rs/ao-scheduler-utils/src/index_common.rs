use crate::{client::{gateway::Gateway, in_memory::LocalLruCache, scheduler::CheckForRedirect}, locate::Locate, raw::Raw, validate::Validate};

const DEFAULT_GRAPHQL_URL: &str = "https://arweave.net/graphql";

/**
 * @typedef ConnectParams
 * @property {number} [cacheSize] - the size of the internal LRU cache
 * @property {boolean} [followRedirects] - whether to follow redirects and cache that url instead
 * @property {string} [GRAPHQL_URL] - the url of the gateway to be used
 *
 * Build the apis using the provided configuration. You can currently specify
 *
 * - a GRAPHQL_URL. Defaults to https://arweave.net/graphql
 * - a cache size for the internal LRU cache. Defaults to 100
 * - whether or not to follow redirects when locating a scheduler. Defaults to false
 *
 * If either value is not provided, a default will be used.
 * Invoking connect() with no parameters or an empty object is functionally equivalent
 * to using the top-lvl exports
 *
 * @param {ConnectParams} [params]
 */
pub fn connect(cache_size: Option<u64>, graphql_url: Option<&str>, follow_redirects: Option<bool>) -> ConnectReturn {
    let _cache_size = if let Some(cache_size) = cache_size {
        cache_size
    } else {
        100
    };
    let _graphql_url = if let Some(gateway_url) = graphql_url { 
        gateway_url
    } else { 
        DEFAULT_GRAPHQL_URL
    };
    let _follow_redirects = if let Some(follow_redirects) = follow_redirects { follow_redirects } else { false };

    let check_for_redirect = CheckForRedirect;
    let cache = LocalLruCache::new(_cache_size);

    // Locate the scheduler for the given process.
    let locate = Locate::new(Gateway::new(_graphql_url), Gateway::new(_graphql_url), cache.clone(), _follow_redirects, check_for_redirect);

    // Validate whether the given wallet address is an ao Scheduler
    let validate = Validate::new(Gateway::new(_graphql_url), cache.clone());
    
    // Return the `Scheduler-Location` record for the address
    // or undefined, if it cannot be found
    let raw = Raw::new(Gateway::new(_graphql_url), cache.clone());

    ConnectReturn {
        locate,
        validate,
        raw
    }
}

pub struct ConnectReturn {
    pub locate: Locate<Gateway, Gateway, LocalLruCache, CheckForRedirect>,
    pub validate: Validate<LocalLruCache, Gateway>,
    pub raw: Raw<LocalLruCache, Gateway>
}