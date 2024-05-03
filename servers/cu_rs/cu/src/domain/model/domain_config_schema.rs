use valid::{constraint::{CharCount, NotEmpty, INVALID_CHAR_COUNT_MIN, INVALID_DIGITS_INTEGER}, invalid_value, ConstraintViolation, State, Validate, Validation, ValidationError};
use crate::config::StartConfigEnv;
use super::{
    parse_schema::StartSchemaParser, 
    positive_int_schema::PositiveIntSchemaConstraint, 
    shared_validation::{parse_db_url_schema, parse_min_char_one_schema, parse_wallet_schema, INVALID_URL, INVALID_WALLET}, 
    truthy_schema::{TruthyConstraint, INVALID_NOT_BOOLEAN}, 
    url_parse_schema::UrlConstraint, uuid_array_schema::{UuidArrayConstraint, INVALID_ARRAY}
};
use super::positive_int_schema::parse_positive_int_schema;
use super::url_parse_schema::parse_url_parse_schema;
use super::truthy_schema::parse_boolean_schema;
use super::uuid_array_schema::parse_array_schema;

#[derive(Clone)]
#[allow(non_snake_case)]
pub struct StartDomainConfigSchema {
    /**
    * The maximum Memory-Limit, in bytes, supported for ao processes
    *
    * ie. '1000' or '1_000'
    */
    pub PROCESS_WASM_MEMORY_MAX_LIMIT: Option<String>,
    /**
    * The maximum Compute-Limit, in bytes, supported for ao processes
    *
    * ie. '1000' or '1_000'
    */
    pub PROCESS_WASM_COMPUTE_MAX_LIMIT: Option<String>,
    /**
     * The url for the graphql server to be used by the CU
     * to query for metadata from an Arweave Gateway
     *
     * ie. https://arweave.net/graphql
     */
    pub GRAPHQL_URL: Option<String>,
    /**
     * The url for the graphql server to be used by the CU
     * to query for process Checkpoints.
     *
     * ie. https://arweave.net/graphql
     */
    pub CHECKPOINT_GRAPHQL_URL: Option<String>,
    /**
     * The url for the server that hosts the Arweave http API
     *
     * ie. https://arweave.net
     */
    pub ARWEAVE_URL: Option<String>,
    /**
    * The url of the uploader to use to upload Process Checkpoints to Arweave
    */
    pub UPLOADER_URL: Option<String>,
    /**
    * The connection string to the database
    */
    pub DB_URL: Option<String>,
    /**
    * The wallet for the CU
    */
    pub WALLET: Option<String>,
    /**
    * The interval, in milliseconds, at which to log memory usage on this CU.
    */
    pub MEM_MONITOR_INTERVAL: Option<String>,
    /**
    * The amount of time, in milliseconds, that the CU should wait before creating a process Checkpoint,
    * if it has already created a Checkpoint for that process.
    *
    * This is effectively a throttle on Checkpoint creation, for a given process
    */
    pub PROCESS_CHECKPOINT_CREATION_THROTTLE: Option<String>,
    /**
    * Whether to disable Process Checkpoint creation entirely. Great for when developing locally,
    * of for an ephemeral CU
    */
    pub DISABLE_PROCESS_CHECKPOINT_CREATION: Option<String>,
    /**
    * If an evaluation stream evaluates this amount of messages,
    * then it will immediately create a Checkpoint at the end of the
    * evaluation stream
    */
    pub EAGER_CHECKPOINT_THRESHOLD: Option<String>,
    /**
    * The number of workers to use for evaluating messages
    */
    pub WASM_EVALUATION_MAX_WORKERS: Option<String>,
    /**
    * The maximum size of the in-memory cache used for wasm instances
    */
    pub WASM_INSTANCE_CACHE_MAX_SIZE: Option<String>,
    /**
    * The maximum size of the in-memory cache used for Wasm modules
    */
    pub WASM_MODULE_CACHE_MAX_SIZE: Option<String>,
    /**
    * The directory to place wasm binaries downloaded from arweave.
    */
    pub WASM_BINARY_FILE_DIRECTORY: Option<String>,
    /**
    * An array of process ids that should not use Checkpoints
    * on Arweave.
    */
    pub PROCESS_IGNORE_ARWEAVE_CHECKPOINTS: Option<String>,
    /**
    * The directory to cache Checkpoints created on Arweave
    */
    pub PROCESS_CHECKPOINT_FILE_DIRECTORY: Option<String>,
    /**
    * The maximum size, in bytes, of the cache used to cache the latest memory
    * evaluated for an ao process
    */
    pub PROCESS_MEMORY_CACHE_MAX_SIZE: Option<String>,
    /**
    * The time to live for a cache entry in the process latest memory cache.
    * An entries ttl is rest each time it is accessed
    */
    pub PROCESS_MEMORY_CACHE_TTL: Option<String>,
    /**
     * The interval at which the CU should Checkpoint all processes stored in it's
     * cache.
     *
     * Set to 0 to disable
     */
    pub PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL: Option<String>,
    /**
    * The amount of time in milliseconds, the CU should wait for evaluation to complete
    * before responding with a "busy" message to the client
    */
    pub BUSY_THRESHOLD: Option<String>,
    /**
     * A list of process ids that the CU should restrict
     * aka. blacklist
     */
    pub RESTRICT_PROCESSES: Option<String>,
    /**
     * A list of process ids that the CU should exclusively allow
     * aka. whitelist
     */
    pub ALLOW_PROCESSES: Option<String>,
}

impl StartDomainConfigSchema {
    pub fn new(start_config_env: StartConfigEnv) -> Self {
        StartDomainConfigSchema {
            GRAPHQL_URL: start_config_env.GRAPHQL_URL,
            CHECKPOINT_GRAPHQL_URL: start_config_env.CHECKPOINT_GRAPHQL_URL,
            ARWEAVE_URL: start_config_env.ARWEAVE_URL,
            UPLOADER_URL: start_config_env.UPLOADER_URL,
            DB_URL: start_config_env.DB_URL,
            WALLET: start_config_env.WALLET,
            MEM_MONITOR_INTERVAL: start_config_env.MEM_MONITOR_INTERVAL,
            PROCESS_CHECKPOINT_CREATION_THROTTLE: start_config_env.PROCESS_CHECKPOINT_CREATION_THROTTLE,
            DISABLE_PROCESS_CHECKPOINT_CREATION: start_config_env.DISABLE_PROCESS_CHECKPOINT_CREATION,
            EAGER_CHECKPOINT_THRESHOLD: start_config_env.EAGER_CHECKPOINT_THRESHOLD,
            PROCESS_WASM_MEMORY_MAX_LIMIT: start_config_env.PROCESS_WASM_MEMORY_MAX_LIMIT,
            PROCESS_WASM_COMPUTE_MAX_LIMIT: start_config_env.PROCESS_WASM_COMPUTE_MAX_LIMIT,
            WASM_EVALUATION_MAX_WORKERS: start_config_env.WASM_EVALUATION_MAX_WORKERS,
            WASM_INSTANCE_CACHE_MAX_SIZE: start_config_env.WASM_INSTANCE_CACHE_MAX_SIZE,
            WASM_MODULE_CACHE_MAX_SIZE: start_config_env.WASM_MODULE_CACHE_MAX_SIZE,
            WASM_BINARY_FILE_DIRECTORY: start_config_env.WASM_BINARY_FILE_DIRECTORY,
            PROCESS_IGNORE_ARWEAVE_CHECKPOINTS: start_config_env.PROCESS_IGNORE_ARWEAVE_CHECKPOINTS,
            PROCESS_CHECKPOINT_FILE_DIRECTORY: start_config_env.PROCESS_CHECKPOINT_FILE_DIRECTORY,
            PROCESS_MEMORY_CACHE_MAX_SIZE: start_config_env.PROCESS_MEMORY_CACHE_MAX_SIZE,
            PROCESS_MEMORY_CACHE_TTL: start_config_env.PROCESS_MEMORY_CACHE_TTL,
            PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL: start_config_env.PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL,
            BUSY_THRESHOLD: start_config_env.BUSY_THRESHOLD,
            RESTRICT_PROCESSES: start_config_env.RESTRICT_PROCESSES,
            ALLOW_PROCESSES: start_config_env.ALLOW_PROCESSES
        }
    }
}

impl StartSchemaParser<DomainConfigSchema> for StartDomainConfigSchema {
    #[allow(non_snake_case)]
    fn parse(&self) -> Result<DomainConfigSchema, ValidationError> {
        let mut final_domain_config_schema = DomainConfigSchema::default();

        match parse_positive_int_schema(self.PROCESS_WASM_MEMORY_MAX_LIMIT.clone(), "PROCESS_WASM_MEMORY_MAX_LIMIT") {
            Ok(val) => final_domain_config_schema.PROCESS_WASM_MEMORY_MAX_LIMIT = val,
            Err(e) => return Err(e)
        };
        
        match parse_positive_int_schema(self.PROCESS_WASM_COMPUTE_MAX_LIMIT.clone(), "PROCESS_WASM_COMPUTE_MAX_LIMIT") {
            Ok(val) => final_domain_config_schema.PROCESS_WASM_COMPUTE_MAX_LIMIT = val,
            Err(e) => return Err(e)
        };
        
        match parse_url_parse_schema(self.GRAPHQL_URL.clone(), "GRAPHQL_URL") {
            Ok(val) => final_domain_config_schema.GRAPHQL_URL = val,
            Err(e) => return Err(e)
        };

        match parse_url_parse_schema(self.CHECKPOINT_GRAPHQL_URL.clone(), "CHECKPOINT_GRAPHQL_URL") {
            Ok(val) => final_domain_config_schema.CHECKPOINT_GRAPHQL_URL = val,
            Err(e) => return Err(e)
        };
        
        match parse_url_parse_schema(self.UPLOADER_URL.clone(), "UPLOADER_URL") {
            Ok(val) => final_domain_config_schema.UPLOADER_URL = val,
            Err(e) => return Err(e)
        };
        
        match parse_url_parse_schema(self.ARWEAVE_URL.clone(), "ARWEAVE_URL") {
            Ok(val) => final_domain_config_schema.ARWEAVE_URL = val,
            Err(e) => return Err(e)
        };
        
        match parse_db_url_schema(self.DB_URL.clone(), "DB_URL") {
            Ok(val) => final_domain_config_schema.DB_URL = val,
            Err(e) => return Err(e)
        };       
        
        
        match parse_wallet_schema(self.WALLET.clone()) {
            Ok(val) => final_domain_config_schema.WALLET = val,
            Err(e) => return Err(e)
        };

        match parse_positive_int_schema(self.MEM_MONITOR_INTERVAL.clone(), "MEM_MONITOR_INTERVAL") {
            Ok(val) => final_domain_config_schema.MEM_MONITOR_INTERVAL = val,
            Err(e) => return Err(e)
        };
        
        match parse_positive_int_schema(self.PROCESS_CHECKPOINT_CREATION_THROTTLE.clone(), "PROCESS_CHECKPOINT_CREATION_THROTTLE") {
            Ok(val) => final_domain_config_schema.PROCESS_CHECKPOINT_CREATION_THROTTLE = val,
            Err(e) => return Err(e)
        };
        
        match parse_boolean_schema(self.DISABLE_PROCESS_CHECKPOINT_CREATION.clone()) {
            Ok(val) => final_domain_config_schema.DISABLE_PROCESS_CHECKPOINT_CREATION = val,
            Err(e) => return Err(e)
        };
        match parse_positive_int_schema(self.EAGER_CHECKPOINT_THRESHOLD.clone(), "EAGER_CHECKPOINT_THRESHOLD") {
            Ok(val) => final_domain_config_schema.EAGER_CHECKPOINT_THRESHOLD = val,
            Err(e) => return Err(e)
        };
        match parse_positive_int_schema(self.WASM_EVALUATION_MAX_WORKERS.clone(), "WASM_EVALUATION_MAX_WORKERS") {
            Ok(val) => final_domain_config_schema.WASM_EVALUATION_MAX_WORKERS = val,
            Err(e) => return Err(e)
        };
        match parse_positive_int_schema(self.WASM_INSTANCE_CACHE_MAX_SIZE.clone(), "WASM_INSTANCE_CACHE_MAX_SIZE") {
            Ok(val) => final_domain_config_schema.WASM_INSTANCE_CACHE_MAX_SIZE = val,
            Err(e) => return Err(e)
        };
        match parse_positive_int_schema(self.WASM_MODULE_CACHE_MAX_SIZE.clone(), "WASM_MODULE_CACHE_MAX_SIZE") {
            Ok(val) => final_domain_config_schema.WASM_MODULE_CACHE_MAX_SIZE = val,
            Err(e) => return Err(e)
        };
        match parse_min_char_one_schema(self.WASM_BINARY_FILE_DIRECTORY.clone(), "WASM_BINARY_FILE_DIRECTORY") {
            Ok(val) => final_domain_config_schema.WASM_BINARY_FILE_DIRECTORY = val,
            Err(e) => return Err(e)
        };
        match parse_array_schema(self.PROCESS_IGNORE_ARWEAVE_CHECKPOINTS.clone()) {
            Ok(val) => final_domain_config_schema.PROCESS_IGNORE_ARWEAVE_CHECKPOINTS = val,
            Err(e) => return Err(e)
        };
        match parse_min_char_one_schema(self.PROCESS_CHECKPOINT_FILE_DIRECTORY.clone(), "PROCESS_CHECKPOINT_FILE_DIRECTORY") {
            Ok(val) => final_domain_config_schema.PROCESS_CHECKPOINT_FILE_DIRECTORY = val,
            Err(e) => return Err(e)
        };
        match parse_positive_int_schema(self.PROCESS_MEMORY_CACHE_MAX_SIZE.clone(), "PROCESS_MEMORY_CACHE_MAX_SIZE") {
            Ok(val) => final_domain_config_schema.PROCESS_MEMORY_CACHE_MAX_SIZE = val,
            Err(e) => return Err(e)
        };
        match parse_positive_int_schema(self.PROCESS_MEMORY_CACHE_TTL.clone(), "PROCESS_MEMORY_CACHE_TTL") {
            Ok(val) => final_domain_config_schema.PROCESS_MEMORY_CACHE_TTL = val,
            Err(e) => return Err(e)
        };
        match parse_positive_int_schema(self.PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL.clone(), "PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL") {
            Ok(val) => final_domain_config_schema.PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL = val,
            Err(e) => return Err(e)
        };
        match parse_positive_int_schema(self.BUSY_THRESHOLD.clone(), "BUSY_THRESHOLD") {
            Ok(val) => final_domain_config_schema.BUSY_THRESHOLD = val,
            Err(e) => return Err(e)
        };
        match parse_array_schema(self.RESTRICT_PROCESSES.clone()) {
            Ok(val) => final_domain_config_schema.RESTRICT_PROCESSES = val,
            Err(e) => return Err(e)
        };
        match parse_array_schema(self.ALLOW_PROCESSES.clone()) {
            Ok(val) => final_domain_config_schema.ALLOW_PROCESSES = val,
            Err(e) => return Err(e)
        };

        Ok(final_domain_config_schema)
    }
}


pub struct StartDomainConfigSchemaConstraint;
pub struct StartDomainConfigSchemaState;

impl<'a> Validate<StartDomainConfigSchemaConstraint, State<&'a StartDomainConfigSchemaState>> for StartDomainConfigSchema {
    // todo: finish
    fn validate(self, _context: impl Into<State<&'a StartDomainConfigSchemaState>>, _constraint: &StartDomainConfigSchemaConstraint) -> Validation<StartDomainConfigSchemaConstraint, Self> {
        let mut violations: Vec<ConstraintViolation> = vec![];

        if self.clone().PROCESS_WASM_MEMORY_MAX_LIMIT.validate("PROCESS_WASM_MEMORY_MAX_LIMIT", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "PROCESS_WASM_MEMORY_MAX_LIMIT", "".to_string(), "".to_string()));
        }
        if self.clone().PROCESS_WASM_COMPUTE_MAX_LIMIT.validate("PROCESS_WASM_COMPUTE_MAX_LIMIT", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "PROCESS_WASM_COMPUTE_MAX_LIMIT", "".to_string(), "".to_string()));
        }
        if self.clone().GRAPHQL_URL.validate("GRAPHQL_URL", &UrlConstraint::new()).result().is_err() {
            violations.push(invalid_value(INVALID_URL, "GRAPHQL_URL", "".to_string(), "".to_string()));
        }
        if self.clone().UPLOADER_URL.validate("UPLOADER_URL", &UrlConstraint::new()).result().is_err() {
            violations.push(invalid_value(INVALID_URL, "UPLOADER_URL", "".to_string(), "".to_string()));
        }
        if self.clone().CHECKPOINT_GRAPHQL_URL.validate("CHECKPOINT_GRAPHQL_URL", &UrlConstraint::new()).result().is_err() {
            violations.push(invalid_value(INVALID_URL, "CHECKPOINT_GRAPHQL_URL", "".to_string(), "".to_string()));
        }
        if self.clone().ARWEAVE_URL.validate("ARWEAVE_URL", &UrlConstraint::new()).result().is_err() {
            violations.push(invalid_value(INVALID_URL, "ARWEAVE_URL", "".to_string(), "".to_string()));
        }
        if self.clone().DB_URL.validate("DB_URL", &NotEmpty).result().is_err()
            || self.clone().DB_URL.unwrap().validate("DB_URL", &CharCount::Min(1)).result().is_err() {
            violations.push(invalid_value(INVALID_URL, "DB_URL", "".to_string(), "".to_string()));
        }
        if self.clone().WALLET.validate("WALLET", &NotEmpty).result().is_err()
            || self.clone().WALLET.unwrap().validate("WALLET", &CharCount::Min(1)).result().is_err() {
            violations.push(invalid_value(INVALID_WALLET, "WALLET", "".to_string(), "".to_string()));
        }
        if self.clone().MEM_MONITOR_INTERVAL.validate("MEM_MONITOR_INTERVAL", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "MEM_MONITOR_INTERVAL", "".to_string(), "".to_string()));
        }
        if self.clone().PROCESS_CHECKPOINT_CREATION_THROTTLE.validate("PROCESS_CHECKPOINT_CREATION_THROTTLE", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "PROCESS_CHECKPOINT_CREATION_THROTTLE", "".to_string(), "".to_string()));
        }
        if self.clone().DISABLE_PROCESS_CHECKPOINT_CREATION.validate("DISABLE_PROCESS_CHECKPOINT_CREATION", &TruthyConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_NOT_BOOLEAN, "DISABLE_PROCESS_CHECKPOINT_CREATION", "".to_string(), "".to_string()));
        }
        if self.clone().EAGER_CHECKPOINT_THRESHOLD.validate("EAGER_CHECKPOINT_THRESHOLD", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "EAGER_CHECKPOINT_THRESHOLD", "".to_string(), "".to_string()));
        }
        if self.clone().WASM_EVALUATION_MAX_WORKERS.validate("WASM_EVALUATION_MAX_WORKERS", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "WASM_EVALUATION_MAX_WORKERS", "".to_string(), "".to_string()));
        }
        if self.clone().WASM_INSTANCE_CACHE_MAX_SIZE.validate("WASM_INSTANCE_CACHE_MAX_SIZE", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "WASM_INSTANCE_CACHE_MAX_SIZE", "".to_string(), "".to_string()));
        }
        if self.clone().WASM_MODULE_CACHE_MAX_SIZE.validate("WASM_MODULE_CACHE_MAX_SIZE", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "WASM_MODULE_CACHE_MAX_SIZE", "".to_string(), "".to_string()));
        }
        if self.clone().WASM_BINARY_FILE_DIRECTORY.validate("WASM_BINARY_FILE_DIRECTORY", &NotEmpty).result().is_err()
            || self.clone().WASM_BINARY_FILE_DIRECTORY.unwrap().validate("WASM_BINARY_FILE_DIRECTORY", &CharCount::Min(1)).result().is_err() {
            violations.push(invalid_value(INVALID_CHAR_COUNT_MIN, "WASM_BINARY_FILE_DIRECTORY", "".to_string(), "".to_string()));
        }
        if self.clone().PROCESS_IGNORE_ARWEAVE_CHECKPOINTS.validate("PROCESS_IGNORE_ARWEAVE_CHECKPOINTS", &UuidArrayConstraint::new()).result().is_err() {
            violations.push(invalid_value(INVALID_ARRAY, "PROCESS_IGNORE_ARWEAVE_CHECKPOINTS", "".to_string(), "".to_string()));
        }
        if self.clone().PROCESS_CHECKPOINT_FILE_DIRECTORY.validate("PROCESS_CHECKPOINT_FILE_DIRECTORY", &NotEmpty).result().is_err()
            || self.clone().DB_URL.unwrap().validate("PROCESS_CHECKPOINT_FILE_DIRECTORY", &CharCount::Min(1)).result().is_err() {
            violations.push(invalid_value(INVALID_CHAR_COUNT_MIN, "PROCESS_CHECKPOINT_FILE_DIRECTORY", "".to_string(), "".to_string()));
        }
        if self.clone().PROCESS_MEMORY_CACHE_MAX_SIZE.validate("PROCESS_MEMORY_CACHE_MAX_SIZE", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "PROCESS_MEMORY_CACHE_MAX_SIZE", "".to_string(), "".to_string()));
        }
        if self.clone().PROCESS_MEMORY_CACHE_TTL.validate("PROCESS_MEMORY_CACHE_TTL", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "PROCESS_MEMORY_CACHE_TTL", "".to_string(), "".to_string()));
        }
        if self.clone().PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL.validate("PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL", "".to_string(), "".to_string()));
        }
        if self.clone().BUSY_THRESHOLD.validate("BUSY_THRESHOLD", &PositiveIntSchemaConstraint).result().is_err() {
            violations.push(invalid_value(INVALID_DIGITS_INTEGER, "BUSY_THRESHOLD", "".to_string(), "".to_string()));
        }
        if self.clone().RESTRICT_PROCESSES.validate("RESTRICT_PROCESSES", &UuidArrayConstraint::new()).result().is_err() {
            violations.push(invalid_value(INVALID_ARRAY, "RESTRICT_PROCESSES", "".to_string(), "".to_string()));
        }
        if self.clone().ALLOW_PROCESSES.validate("ALLOW_PROCESSES", &UuidArrayConstraint::new()).result().is_err() {
            violations.push(invalid_value(INVALID_ARRAY, "ALLOW_PROCESSES", "".to_string(), "".to_string()));
        }

        if violations.len() > 0 {
            return Validation::failure(violations);
        }
        Validation::success(self)
    }
}

#[derive(Clone, Debug)]
#[allow(non_snake_case)]
pub struct DomainConfigSchema {
    /**
    * The maximum Memory-Limit, in bytes, supported for ao processes
    *
    * ie. '1000' or '1_000'
    */
    pub PROCESS_WASM_MEMORY_MAX_LIMIT: i64,
    /**
    * The maximum Compute-Limit, in bytes, supported for ao processes
    *
    * ie. '1000' or '1_000'
    */
    pub PROCESS_WASM_COMPUTE_MAX_LIMIT: i64,
    /**
     * The url for the graphql server to be used by the CU
     * to query for metadata from an Arweave Gateway
     *
     * ie. https://arweave.net/graphql
     */
    pub GRAPHQL_URL: String,
    /**
     * The url for the graphql server to be used by the CU
     * to query for process Checkpoints.
     *
     * ie. https://arweave.net/graphql
     */
    pub CHECKPOINT_GRAPHQL_URL: String,
    /**
     * The url for the server that hosts the Arweave http API
     *
     * ie. https://arweave.net
     */
    pub ARWEAVE_URL: String,
    /**
    * The url of the uploader to use to upload Process Checkpoints to Arweave
    */
    pub UPLOADER_URL: String,
    /**
    * The connection string to the database
    */
    pub DB_URL: String,
    /**
    * The wallet for the CU
    */
    pub WALLET: String,
    /**
    * The interval, in milliseconds, at which to log memory usage on this CU.
    */
    pub MEM_MONITOR_INTERVAL: i64,
    /**
    * The amount of time, in milliseconds, that the CU should wait before creating a process Checkpoint,
    * if it has already created a Checkpoint for that process.
    *
    * This is effectively a throttle on Checkpoint creation, for a given process
    */
    pub PROCESS_CHECKPOINT_CREATION_THROTTLE: i64,
    /**
    * Whether to disable Process Checkpoint creation entirely. Great for when developing locally,
    * of for an ephemeral CU
    */
    pub DISABLE_PROCESS_CHECKPOINT_CREATION: bool,
    /**
    * If an evaluation stream evaluates this amount of messages,
    * then it will immediately create a Checkpoint at the end of the
    * evaluation stream
    */
    pub EAGER_CHECKPOINT_THRESHOLD: i64,
    /**
    * The number of workers to use for evaluating messages
    */
    pub WASM_EVALUATION_MAX_WORKERS: i64,
    /**
    * The maximum size of the in-memory cache used for wasm instances
    */
    pub WASM_INSTANCE_CACHE_MAX_SIZE: i64,
    /**
    * The maximum size of the in-memory cache used for Wasm modules
    */
    pub WASM_MODULE_CACHE_MAX_SIZE: i64,
    /**
    * The directory to place wasm binaries downloaded from arweave.
    */
    pub WASM_BINARY_FILE_DIRECTORY: String,
    /**
    * An array of process ids that should not use Checkpoints
    * on Arweave.
    */
    pub PROCESS_IGNORE_ARWEAVE_CHECKPOINTS: Vec<String>,
    /**
    * The directory to cache Checkpoints created on Arweave
    */
    pub PROCESS_CHECKPOINT_FILE_DIRECTORY: String,
    /**
    * The maximum size, in bytes, of the cache used to cache the latest memory
    * evaluated for an ao process
    */
    pub PROCESS_MEMORY_CACHE_MAX_SIZE: i64,
    /**
    * The time to live for a cache entry in the process latest memory cache.
    * An entries ttl is rest each time it is accessed
    */
    pub PROCESS_MEMORY_CACHE_TTL: i64,
    /**
     * The interval at which the CU should Checkpoint all processes stored in it's
     * cache.
     *
     * Set to 0 to disable
     */
    pub PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL: i64,
    /**
    * The amount of time in milliseconds, the CU should wait for evaluation to complete
    * before responding with a "busy" message to the client
    */
    pub BUSY_THRESHOLD: i64,
    /**
     * A list of process ids that the CU should restrict
     * aka. blacklist
     */
    pub RESTRICT_PROCESSES: Vec<String>,
    /**
     * A list of process ids that the CU should exclusively allow
     * aka. whitelist
     */
    pub ALLOW_PROCESSES: Vec<String>
}

impl Default for DomainConfigSchema {
    fn default() -> Self {
        DomainConfigSchema {
            PROCESS_WASM_MEMORY_MAX_LIMIT: 0,
            PROCESS_WASM_COMPUTE_MAX_LIMIT: 0,
            GRAPHQL_URL: "".to_string(),
            CHECKPOINT_GRAPHQL_URL: "".to_string(),
            ARWEAVE_URL: "".to_string(),
            UPLOADER_URL: "".to_string(),                        
            DB_URL: "".to_string(),            
            WALLET: "".to_string(),
            MEM_MONITOR_INTERVAL: 0,
            PROCESS_CHECKPOINT_CREATION_THROTTLE: 0,
            DISABLE_PROCESS_CHECKPOINT_CREATION: false,
            EAGER_CHECKPOINT_THRESHOLD: 0,
            WASM_EVALUATION_MAX_WORKERS: 0,
            WASM_INSTANCE_CACHE_MAX_SIZE: 0,
            WASM_MODULE_CACHE_MAX_SIZE: 0,
            WASM_BINARY_FILE_DIRECTORY: "".to_string(),
            PROCESS_IGNORE_ARWEAVE_CHECKPOINTS: vec![],
            PROCESS_CHECKPOINT_FILE_DIRECTORY: "".to_string(),
            PROCESS_MEMORY_CACHE_MAX_SIZE: 0,
            PROCESS_MEMORY_CACHE_TTL: 0,
            PROCESS_MEMORY_CACHE_CHECKPOINT_INTERVAL: 0,
            BUSY_THRESHOLD: 0,
            RESTRICT_PROCESSES: vec![],
            ALLOW_PROCESSES: vec![]
        }
    }
}