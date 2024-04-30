use std::sync::Arc;
use ao_common::domain::{dal::Log, UnitLog};
use lazy_static::lazy_static;

pub fn get_logger() -> Arc<dyn Log + 'static> {
    lazy_static! {
        static ref LOG: Arc<dyn Log> = {
            UnitLog::init()
        };
    }
    Arc::clone(&LOG)
}