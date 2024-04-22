use std::sync::Arc;
use env_logger::Env;
use log::{error, info};
use crate::domain::core::dal::Log;

pub struct UnitLog;

/*
Logging instance, using an instance of this
instead of the env_logger macros throughout
the code
*/

impl UnitLog {
    pub fn init() -> Arc<dyn Log> {
        env_logger::init_from_env(Env::default().default_filter_or("info"));
        Arc::new(UnitLog {})
    }
}

impl Log for UnitLog {
    fn log(&self, message: String) {
        info!("{}", message);
    }

    fn error(&self, message: String) {
        error!("{}", message);
    }
}
