use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use validator::Validate;
use crate::domain::{dal::SaveModuleSchema, model::model::{ModuleSchema, RawTagSchema}, utils::error::CuErrors};

use super::sqlite::SqliteClient;

#[derive(Validate, Serialize)]
#[allow(unused)]
pub struct ModuleDocSchema {
    #[validate(length(min = 1))]
    id: String,
    /// address
    #[validate(length(min = 1))]
    owner: String,
    tags: Vec<RawTagSchema>
}

struct Query {
    sql: String,
    parameters: Vec<String>
}

struct AoModule {
    sql_client: Arc<SqliteClient>
}

impl AoModule {
    pub fn new(sql_client: Arc<SqliteClient>) -> Self {
        AoModule {
            sql_client
        }
    }

    pub fn create_save_module_query(module: ModuleDocSchema) -> Query {
        Query {
            sql: r"
              INSERT OR IGNORE INTO ${MODULES_TABLE}
              (id, tags, owner)
              VALUES (?, ?, ?)
            ".to_string(),
            parameters: vec![
              module.id,
              serde_json::to_string(&module.tags).unwrap(),
              module.owner
            ]
        }
    }
}

#[async_trait]
impl SaveModuleSchema for AoModule {
    async fn save_module(module_schema: ModuleSchema) -> Result<(), CuErrors> {

        Ok(())
    }
}