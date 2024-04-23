use crate::{client::{gateway::Gateway, in_memory::LocalLruCache, scheduler::CheckForRedirect}, locate::Locate, raw::Raw, validate::Validate};

const DEFAULT_GATEWAY_URL: &str = "https://arweave.net";
const DEFAULT_UPLOADER_URL: &str = "https://up.arweave.net";

/**
 * @typedef ConnectParams
 * @property {number} [cacheSize] - the size of the internal LRU cache
 * @property {boolean} [followRedirects] - whether to follow redirects and cache that url instead
 * @property {string} [GATEWAY_URL] - the url of the gateway to be used
 *
 * Build the apis using the provided configuration. You can currently specify
 *
 * - a GATEWAY_URL. Defaults to https://arweave.net
 * - a cache size for the internal LRU cache. Defaults to 100
 * - whether or not to follow redirects when locating a scheduler. Defaults to false
 *
 * If either value is not provided, a default will be used.
 * Invoking connect() with no parameters or an empty object is functionally equivalent
 * to using the top-lvl exports
 *
 * @param {ConnectParams} [params]
 */
pub fn connect(cache_size: u64, gateway_url: Option<&str>, uploader_url: Option<&str>, follow_redirects: Option<bool>) -> ConnectReturn {
    let _gateway_url = if let Some(gateway_url) = gateway_url { 
        gateway_url
    } else { 
        DEFAULT_GATEWAY_URL
    };
    let _uploader_url = if let Some(uploader_url) = uploader_url { 
        uploader_url
    } else { 
        DEFAULT_UPLOADER_URL
    };
    let _follow_redirects = if let Some(follow_redirects) = follow_redirects { follow_redirects } else { false };

    let check_for_redirect = CheckForRedirect;
    let cache = LocalLruCache::new(cache_size);

    let locate = Locate::new(Gateway::new(_gateway_url), cache.clone(), _follow_redirects, check_for_redirect);

    let validate = Validate::new(Gateway::new(_gateway_url), cache.clone());
    
    let raw = Raw::new(Gateway::new(_gateway_url), cache.clone());

    ConnectReturn {
        locate,
        validate,
        raw
    }
}

pub struct ConnectReturn {
    pub locate: Locate<LocalLruCache, Gateway, CheckForRedirect>,
    pub validate: Validate<LocalLruCache, Gateway>,
    pub raw: Raw<LocalLruCache, Gateway>
}