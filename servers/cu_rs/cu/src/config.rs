use std::env;
use dotenv::dotenv;
use once_cell::sync::OnceCell;
use crate::utils::{datetime::{get_ms_from_hour, get_ms_from_sec}, paths::get_path_as_string, string_converters::get_array};
use std::env::temp_dir;

/**
 * @type {z.infer<typeof serverConfigSchema>}
 *
 * We get some nice Intellisense by defining the type in JSDoc
 * before parsing with the serverConfig schema
 */
pub static CONFIG_ENVS: OnceCell<ConfigEnvSet> = OnceCell::new();

fn get_config_env(development: bool) -> ConfigEnv {
    dotenv().ok();

    let envs = CONFIG_ENVS.get_or_init(|| ConfigEnvSet {
        development: ConfigEnv {
            MODE: env::var("NODE_CONFIG_ENV").unwrap(),
            port: env::var("PORT").ok().and_then(|val| val.parse::<i64>().ok()).unwrap_or(6363),
            GATEWAY_URL: env::var("GATEWAY_URL").unwrap_or("https://arweave.net".to_string()),
            UPLOADER_URL: env::var("UPLOADER_URL").unwrap_or("https://up.arweave.net".to_string()),
            DB_MODE: env::var("DB_MODE").unwrap_or("embedded".to_string()),
            DB_URL: env::var("DB_URL").unwrap_or("ao-cache".to_string()),
            DB_MAX_LISTENERS: env::var("DB_MAX_LISTENERS")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(100),
            DUMP_PATH: env::var("DUMP_PATH").unwrap_or("./static".to_string()),
            WALLET: env::var("WALLET").unwrap(),
            WALLET_FILE: env::var("WALLET_FILE").unwrap(),
            MEM_MONITOR_INTERVAL: env::var("MEM_MONITOR_INTERVAL")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(get_ms_from_sec(10)),
            PROCESS_CHECKPOINT_CREATION_THROTTLE: env::var("PROCESS_CHECKPOINT_CREATION_THROTTLE")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(get_ms_from_hour(24)),
            DISABLE_PROCESS_CHECKPOINT_CREATION: env::var("DISABLE_PROCESS_CHECKPOINT_CREATION")
                .ok()
                .and_then(|val| val.parse::<bool>().ok())
                .unwrap_or(false) != false,
            EAGER_CHECKPOINT_THRESHOLD: env::var("EAGER_CHECKPOINT_THRESHOLD")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(100),
            PROCESS_WASM_MEMORY_MAX_LIMIT: env::var("PROCESS_WASM_MEMORY_MAX_LIMIT")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(1_000_000_000), // 1GB
            PROCESS_WASM_COMPUTE_MAX_LIMIT: env::var("PROCESS_WASM_COMPUTE_MAX_LIMIT")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(9_000_000_000), // 9b
            WASM_EVALUATION_MAX_WORKERS: env::var("WASM_EVALUATION_MAX_WORKERS")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(3),
            WASM_INSTANCE_CACHE_MAX_SIZE: env::var("WASM_INSTANCE_CACHE_MAX_SIZE")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(5), // 5 loaded wasm modules
            WASM_MODULE_CACHE_MAX_SIZE: env::var("WASM_MODULE_CACHE_MAX_SIZE")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(5), // 5 wasm binaries
            WASM_BINARY_FILE_DIRECTORY: env::var("WASM_BINARY_FILE_DIRECTORY").unwrap_or(get_path_as_string(temp_dir())),
            PROCESS_IGNORE_ARWEAVE_CHECKPOINTS: get_array(env::var("PROCESS_IGNORE_ARWEAVE_CHECKPOINTS").unwrap_or("".to_string())),
            PROCESS_CHECKPOINT_FILE_DIRECTORY: env::var("PROCESS_CHECKPOINT_FILE_DIRECTORY").unwrap_or(get_path_as_string(temp_dir())),
            PROCESS_MEMORY_CACHE_MAX_SIZE: env::var("PROCESS_MEMORY_CACHE_MAX_SIZE")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(500_000_000), // 500MB
            PROCESS_MEMORY_CACHE_TTL: env::var("PROCESS_MEMORY_CACHE_TTL")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(get_ms_from_hour(24)),
            BUSY_THRESHOLD: env::var("BUSY_THRESHOLD")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(0) // disabled
        },
        production: ConfigEnv {
            MODE: env::var("NODE_CONFIG_ENV").unwrap(),
            port: env::var("PORT").ok().and_then(|val| val.parse::<i64>().ok()).unwrap_or(6363),
            GATEWAY_URL: env::var("GATEWAY_URL").unwrap_or("https://arweave.net".to_string()),
            UPLOADER_URL: env::var("UPLOADER_URL").unwrap_or("https://up.arweave.net".to_string()),
            DB_MODE: env::var("DB_MODE").unwrap_or("embedded".to_string()),
            DB_URL: env::var("DB_URL").unwrap_or("ao-cache".to_string()), // todo: need to see how this is used
            DB_MAX_LISTENERS: env::var("DB_MAX_LISTENERS").ok().and_then(|val| val.parse::<i64>().ok()).unwrap_or(100),
            DUMP_PATH: env::var("DUMP_PATH").unwrap_or(get_path_as_string(temp_dir())),
            WALLET: env::var("WALLET").unwrap(),
            WALLET_FILE: env::var("WALLET_FILE").unwrap(),
            MEM_MONITOR_INTERVAL: env::var("MEM_MONITOR_INTERVAL")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(get_ms_from_sec(30)),
            PROCESS_CHECKPOINT_CREATION_THROTTLE: env::var("PROCESS_CHECKPOINT_CREATION_THROTTLE")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(get_ms_from_hour(24)),
            DISABLE_PROCESS_CHECKPOINT_CREATION: env::var("DISABLE_PROCESS_CHECKPOINT_CREATION")
                .ok()
                .and_then(|val| val.parse::<bool>().ok())
                .unwrap_or(false) != false, // TODO: disabled by default for now. Enable by default later
            EAGER_CHECKPOINT_THRESHOLD: env::var("EAGER_CHECKPOINT_THRESHOLD")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(100),
            PROCESS_WASM_MEMORY_MAX_LIMIT: env::var("PROCESS_WASM_MEMORY_MAX_LIMIT")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(1_000_000_000), // 1GB
            PROCESS_WASM_COMPUTE_MAX_LIMIT: env::var("PROCESS_WASM_COMPUTE_MAX_LIMIT")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(9_000_000_000), // 9b
            WASM_EVALUATION_MAX_WORKERS: env::var("WASM_EVALUATION_MAX_WORKERS")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(3),
            WASM_INSTANCE_CACHE_MAX_SIZE: env::var("WASM_INSTANCE_CACHE_MAX_SIZE")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(5), // 5 loaded wasm modules
            WASM_MODULE_CACHE_MAX_SIZE: env::var("WASM_MODULE_CACHE_MAX_SIZE")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(5), // 5 wasm binaries
            WASM_BINARY_FILE_DIRECTORY: env::var("WASM_BINARY_FILE_DIRECTORY").unwrap_or(get_path_as_string(temp_dir())),
            PROCESS_IGNORE_ARWEAVE_CHECKPOINTS: get_array(env::var("PROCESS_IGNORE_ARWEAVE_CHECKPOINTS").unwrap_or("".to_string())),
            PROCESS_CHECKPOINT_FILE_DIRECTORY: env::var("PROCESS_CHECKPOINT_FILE_DIRECTORY").unwrap_or(get_path_as_string(temp_dir())),
            PROCESS_MEMORY_CACHE_MAX_SIZE: env::var("PROCESS_MEMORY_CACHE_MAX_SIZE")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(500_000_000), // 500MB
            PROCESS_MEMORY_CACHE_TTL: env::var("PROCESS_MEMORY_CACHE_TTL")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(get_ms_from_hour(24)),
            BUSY_THRESHOLD: env::var("BUSY_THRESHOLD")
                .ok()
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(0) // disabled
        }        
    });

    match development {
        true => envs.development.clone(),
        false => envs.production.clone()
    }
}

pub struct ConfigEnvSet {
    pub development: ConfigEnv,
    pub production: ConfigEnv
}

#[allow(non_snake_case)]
#[derive(Clone)]
struct ConfigEnv {
    MODE: String,
    port: i64, // process.env.PORT || 6363,
    GATEWAY_URL: String, // process.env.GATEWAY_URL || 'https://arweave.net',
    UPLOADER_URL: String, // process.env.UPLOADER_URL || 'https://up.arweave.net',
    DB_MODE: String, // process.env.DB_MODE || 'embedded',
    DB_URL: String, // process.env.DB_URL || 'ao-cache',
    DB_MAX_LISTENERS: i64, // parseInt(process.env.DB_MAX_LISTENERS || '100'),
    DUMP_PATH: String, // process.env.DUMP_PATH || './static',
    WALLET: String, // process.env.WALLET,
    WALLET_FILE: String, // process.env.WALLET_FILE,
    MEM_MONITOR_INTERVAL: i64, // process.env.MEM_MONITOR_INTERVAL || ms('10s'),
    PROCESS_CHECKPOINT_CREATION_THROTTLE: i64, // process.env.PROCESS_CHECKPOINT_CREATION_THROTTLE || ms('24h'),
    DISABLE_PROCESS_CHECKPOINT_CREATION: bool, // process.env.DISABLE_PROCESS_CHECKPOINT_CREATION !== 'false',
    EAGER_CHECKPOINT_THRESHOLD: i64, // process.env.EAGER_CHECKPOINT_THRESHOLD || 100,
    PROCESS_WASM_MEMORY_MAX_LIMIT: i64, // process.env.PROCESS_WASM_MEMORY_MAX_LIMIT || 1_000_000_000, // 1GB
    PROCESS_WASM_COMPUTE_MAX_LIMIT: i64, // process.env.PROCESS_WASM_COMPUTE_MAX_LIMIT || 9_000_000_000, // 9b
    WASM_EVALUATION_MAX_WORKERS: i64, // process.env.WASM_EVALUATION_MAX_WORKERS || 3,
    WASM_INSTANCE_CACHE_MAX_SIZE: i64, // process.env.WASM_INSTANCE_CACHE_MAX_SIZE || 5, // 5 loaded wasm modules
    WASM_MODULE_CACHE_MAX_SIZE: i64, // process.env.WASM_MODULE_CACHE_MAX_SIZE || 5, // 5 wasm binaries
    WASM_BINARY_FILE_DIRECTORY: String, // process.env.WASM_BINARY_FILE_DIRECTORY || tmpdir(),
    PROCESS_IGNORE_ARWEAVE_CHECKPOINTS: Vec<String>, // process.env.PROCESS_IGNORE_ARWEAVE_CHECKPOINTS || [],
    PROCESS_CHECKPOINT_FILE_DIRECTORY: String, // process.env.PROCESS_CHECKPOINT_FILE_DIRECTORY || tmpdir(),
    PROCESS_MEMORY_CACHE_MAX_SIZE: i64, // process.env.PROCESS_MEMORY_CACHE_MAX_SIZE || 500_000_000, // 500MB
    PROCESS_MEMORY_CACHE_TTL: i64, // process.env.PROCESS_MEMORY_CACHE_TTL || ms('24h'),
    BUSY_THRESHOLD: i64, // process.env.BUSY_THRESHOLD || 0 // disabled
}

