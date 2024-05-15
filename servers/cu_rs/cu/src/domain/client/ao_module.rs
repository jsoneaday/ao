use std::sync::Arc;
use async_trait::async_trait;
use serde::Serialize;
use sqlx::{prelude::{Row, FromRow}, sqlite::SqliteRow, Sqlite};
use validator::Validate;
use crate::domain::{dal::{FindModuleSchema, SaveModuleSchema}, model::model::{ModuleSchema, RawTagSchema}, utils::error::{CuErrors, HttpError}};
use super::sqlite::{ConnGetter, SqliteClient, MODULES_TABLE};

#[derive(Validate, Serialize, Clone)]
#[allow(unused)]
struct ModuleDocSchema {
    #[validate(length(min = 1))]
    id: String,
    /// address
    #[validate(length(min = 1))]
    owner: String,
    tags: Vec<RawTagSchema>
}

#[derive(Validate, Serialize, Clone)]
#[allow(unused)]
struct ModuleQuerySchema {
    id: String,
    /// address
    owner: String,
    tags: String
}

impl<'r> FromRow<'r, SqliteRow> for ModuleQuerySchema {
    fn from_row(row: &'r SqliteRow) -> Result<Self, sqlx::Error> {
        Ok(
            ModuleQuerySchema { 
                id: row.try_get("id")?, 
                tags: row.try_get("tags")?,
                owner: row.try_get("owner")?
            }
        )
    }
}

struct Query {
    sql: String,
    parameters: Vec<String>
}

pub struct AoModule {
    sql_client: Arc<SqliteClient>
}

impl AoModule {
    pub fn new(sql_client: Arc<SqliteClient>) -> Self {
        AoModule {
            sql_client
        }
    }

    fn to_module_doc(module: ModuleSchema) -> ModuleDocSchema {
        ModuleDocSchema {
            id: module.id,
            owner: module.owner,
            tags: module.tags
        }
    }

    fn create_find_module_query (module_id: String) -> Query {
        Query {
          sql: format!(r"
            SELECT id, tags, owner
            FROM {}
            WHERE
              id = ?
          ", MODULES_TABLE),
          parameters: vec![module_id]
        }
    }

    fn create_save_module_query(module: ModuleDocSchema) -> Query {
        Query {
            sql: format!(r"
              INSERT OR IGNORE INTO {}
              (id, tags, owner)
              VALUES (?, ?, ?)
            ", MODULES_TABLE),
            parameters: vec![
              module.id,
              serde_json::to_string(&module.tags).unwrap(),
              module.owner
            ]
        }
    }
}

#[async_trait]
impl FindModuleSchema for AoModule {
    async fn find_module(&self, module_id: String) -> Result<Option<ModuleSchema>, CuErrors> {
        let query = AoModule::create_find_module_query(module_id);

        let mut raw_query = sqlx::query_as::<_, ModuleQuerySchema>(&query.sql);
        for param in query.parameters.iter() {
            raw_query = raw_query.bind(param);
        }

        match raw_query.fetch_all(self.sql_client.get_conn()).await {
            Ok(res) => match res.first() {
                Some(res) => Ok(Some(ModuleSchema {
                    id: res.id.clone(),
                    tags: serde_json::from_str(&res.tags).unwrap(),
                    owner: res.owner.clone()
                })),
                None => Err(CuErrors::HttpStatus(HttpError { status: 404, message: "Module not found".to_string() }))
            },
            Err(e) => Err(CuErrors::DatabaseError(e))
        }
    }
}

#[async_trait]
impl SaveModuleSchema for AoModule {
    async fn save_module(&self, module: ModuleSchema) -> Result<String, CuErrors> {
        self.sql_client.logger.log(format!("Creating module doc for module {}", module.id));
        let module_doc = AoModule::to_module_doc(module);
        let query = AoModule::create_save_module_query(module_doc.clone());

        let mut raw_query = sqlx::query::<Sqlite>(&query.sql);
        for param in query.parameters.iter() {
            raw_query = raw_query.bind(param);
        }

        match raw_query.execute(self.sql_client.get_conn()).await {
            Ok(res) => {
                println!("res: {:?}", res);
                Ok(module_doc.id.clone())
            },
            Err(e) => Err(CuErrors::DatabaseError(e))
        }
    }
}