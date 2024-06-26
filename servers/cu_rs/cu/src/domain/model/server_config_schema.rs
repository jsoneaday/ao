use valid::{invalid_value, ConstraintViolation, State, Validate, Validation, ValidationError};
use crate::{config::StartConfigEnv, domain::model::domain_config_schema::{DomainConfigSchema, StartDomainConfigSchema}};
use super::{domain_config_schema::{StartDomainConfigSchemaConstraint, StartDomainConfigSchemaState}, parse_schema::StartSchemaParser};

#[derive(Clone)]
#[allow(non_snake_case)]
pub struct StartServerConfigSchema {
    pub GATEWAY_URL: Option<String>,
    pub base: StartDomainConfigSchema,
    pub MODE: Option<String>,
    pub port: Option<String>,
    pub DUMP_PATH: Option<String>
}

impl StartServerConfigSchema {
    pub fn new(start_config: StartConfigEnv, start_domain_config: StartDomainConfigSchema) -> Self {
        StartServerConfigSchema {
            GATEWAY_URL: start_config.GATEWAY_URL,
            base: start_domain_config,
            MODE: start_config.MODE,
            port: start_config.port,
            DUMP_PATH: start_config.DUMP_PATH
        }
    }
}

impl StartSchemaParser<ServerConfigSchema> for StartServerConfigSchema {
    // todo: finish
    fn parse(&self) -> Result<ServerConfigSchema, ValidationError> {
        let mut final_server_config = ServerConfigSchema::default();
        
        let base = self.base.parse();
        match base {
            Ok(base) => final_server_config.base = base,
            Err(e) => return Err(e)
        }

        match self.clone().validate(&ServerConfigState, &ServerConfigConstraint).result() {
            Ok(server_config) => {
                let unwrapped_config = server_config.unwrap();
                final_server_config.MODE = if unwrapped_config.MODE == Some("development".to_string()) { 
                    DevOrProd::Development
                } else {
                    DevOrProd::Production
                };
                final_server_config.port = unwrapped_config.port.unwrap().parse::<u16>().ok().and_then(|p| Some(p)).unwrap_or(0);
                final_server_config.DUMP_PATH = unwrapped_config.DUMP_PATH.unwrap();
            },
            Err(e) => return Err(e)
        };

        Ok(final_server_config)
    }
}

/**
 * The server config is an extension of the config required by the domain (business logic).
 * This prevents our domain from being aware of the environment it is running in ie.
 * An express server. Later, it could be anything
 */
#[allow(non_snake_case)]
#[allow(unused)]
pub struct ServerConfigSchema {
    pub GATEWAY_URL: String,
    pub base: DomainConfigSchema,
    pub MODE: DevOrProd,
    pub port: u16,
    pub DUMP_PATH: String
}

impl Default for ServerConfigSchema {
    fn default() -> Self {
        ServerConfigSchema {
            GATEWAY_URL: "".to_string(),
            base: DomainConfigSchema::default(),
            MODE: DevOrProd::Development,
            port: 0,
            DUMP_PATH: "".to_string()
        }
    }
}

#[allow(unused)]
#[derive(PartialEq)]
pub enum DevOrProd {
    Development,
    Production
}

struct ServerConfigConstraint;
impl ServerConfigConstraint {
    pub fn is_valid_gateway_url(&self, val: &StartServerConfigSchema) -> bool {
        if val.GATEWAY_URL.is_some() && val.GATEWAY_URL.as_ref().unwrap().len() > 0 {
            return true;
        }
        false
    }

    pub fn is_valid_mode(&self, val: &StartServerConfigSchema) -> bool {
        if val.MODE == Some("development".to_string()) || val.MODE == Some("production".to_string()) {
            return true;
        }
        false
    }

    pub fn is_valid_port(&self, val: &StartServerConfigSchema) -> bool {
        if val.port.is_some() && val.port.as_ref().unwrap().parse::<u16>().is_ok() {
            return true;
        }
        false
    }

    pub fn is_valid_dump_path(&self, val: &StartServerConfigSchema) -> bool {
        if val.DUMP_PATH.is_some() && val.DUMP_PATH.as_ref().unwrap().len() > 0 {
            return true;
        }
        false
    }
}
struct ServerConfigState;

impl<'a> Validate<ServerConfigConstraint, State<&'a ServerConfigState>> for StartServerConfigSchema {
    fn validate(self, _context: impl Into<State<&'a ServerConfigState>>, constraint: &ServerConfigConstraint) -> Validation<ServerConfigConstraint, Self> {
        let mut violations: Vec<ConstraintViolation> = vec![];
        _ = self.base.clone().validate(&StartDomainConfigSchemaState, &StartDomainConfigSchemaConstraint).result()
            .err()
            .and_then(|mut e| {
               violations.append(&mut e.violations);
               Some(e)
            });
        if !constraint.is_valid_mode(&self) {
            violations.push(invalid_value("invalid-mode", "MODE", "".to_string(), "".to_string()));
        } 
        if !constraint.is_valid_port(&self) {
            violations.push(invalid_value("invalid-port", "port", "".to_string(), "".to_string()));
        } 
        if !constraint.is_valid_dump_path(&self) {
            violations.push(invalid_value("invalid-dump-path", "DUMP_PATH", "".to_string(), "".to_string()));
        }
        if !constraint.is_valid_gateway_url(&self) {
            violations.push(invalid_value("invalid-gateway-url", "GATEWAY_URL", "".to_string(), "".to_string()));
        }

        if violations.len() > 0 {
            return Validation::failure(violations);
        }
        Validation::success(self)             
    }
}