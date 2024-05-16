use std::sync::Arc;
use async_trait::async_trait;
use rand::RngCore;
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

    fn create_find_module_query (module_id: &str) -> Query {
        Query {
          sql: format!(r"
            SELECT id, tags, owner
            FROM {}
            WHERE
              id = ?
          ", MODULES_TABLE),
          parameters: vec![module_id.to_string()]
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

    fn stream_id() -> [u8; 8] {
        let mut buffer = [0u8; 8];
        rand::thread_rng().fill_bytes(&mut buffer);
        buffer
    }
}

#[async_trait]
impl FindModuleSchema for AoModule {
    async fn find_module(&self, module_id: &str) -> Result<Option<ModuleSchema>, CuErrors> {
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

// todo: need to implement EvaluatorSchema, but let's do after the Docker infra

#[cfg(test)]
mod tests {
    use super::*;

    mod ao_module {
        use super::*;

        mod find_module {
            use super::*;

            mod find_the_module {
                use super::*;

                struct MockFindTheModule;
                #[async_trait]
                impl FindModuleSchema for MockFindTheModule {
                    async fn find_module(&self, _module_id: &str) -> Result<Option<ModuleSchema>, CuErrors> {
                        Ok(
                            Some(
                                ModuleSchema {
                                    id: "mod-123".to_string(),
                                    tags: vec![
                                        RawTagSchema {
                                            name: "foo".to_string(),
                                            value: "bar".to_string()
                                        }
                                    ],
                                    owner: "owner-123".to_string()
                                }
                            )
                        )
                    }
                }

                #[tokio::test]
                async fn test_find_the_module() {
                    let mock = MockFindTheModule;
                    match mock.find_module("mod-123").await {
                        Ok(res) => {
                            assert!(res.clone().unwrap().id == "mod-123");
                            assert!(res.clone().unwrap().tags[0].name == "foo");
                            assert!(res.clone().unwrap().tags[0].value == "bar");
                            assert!(res.unwrap().owner == "owner-123");
                        },
                        Err(e) => panic!("{}", e)
                    }
                }
            }

            mod return_404_status_if_not_found {
                use super::*;

                struct MockReturn404IfNotFound;
                #[async_trait]
                impl FindModuleSchema for MockReturn404IfNotFound {
                    async fn find_module(&self, _module_id: &str) -> Result<Option<ModuleSchema>, CuErrors> {
                        Err(CuErrors::HttpStatus(HttpError { status: 404, message: "Not found".to_string() }))
                    }
                }

                #[tokio::test]
                async fn test_return_404_status_if_not_found() {
                    let mock = MockReturn404IfNotFound;
                    match mock.find_module("mod-123").await {
                        Ok(_) => panic!("Returned unexpected row item"),
                        Err(e) => if let CuErrors::HttpStatus(e) = e {
                            assert!(e.status == 404);
                        } else {
                            panic!("Returned unexpected error")
                        }
                    }
                }
            }

            mod bubble_error {
                use super::*;

                struct MockBubbleError;
                #[async_trait]
                impl FindModuleSchema for MockBubbleError {
                    async fn find_module(&self, _module_id: &str) -> Result<Option<ModuleSchema>, CuErrors> {
                        Err(CuErrors::HttpStatus(HttpError { status: 500, message: "Internal Server Error".to_string() }))
                    }
                }

                #[tokio::test]
                async fn test_bubble_error() {
                    let mock = MockBubbleError;
                    match mock.find_module("mod-123").await {
                        Ok(_) => panic!("Returned unexpected row item"),
                        Err(e) => if let CuErrors::HttpStatus(e) = e {
                            assert!(e.status == 500);
                        } else {
                            panic!("Returned unexpected error")
                        }
                    }
                }
            }
        }
    }

    mod save_module {
        use super::*;

        mod save_the_module {
            use super::*;

            struct MockSaveTheModule;
            #[async_trait]
            impl SaveModuleSchema for MockSaveTheModule {
                async fn save_module(&self, module_schema: ModuleSchema) -> Result<String, CuErrors> {
                    let query = AoModule::create_save_module_query(AoModule::to_module_doc(module_schema.clone()));

                    assert!(query.parameters[0] == "mod-123");
                    assert!(query.parameters[1] == serde_json::to_string(&vec![
                        RawTagSchema {
                            name: "Module-Format".to_string(),
                            value: "wasm32-unknown-emscripten".to_string()
                        }
                    ]).unwrap());
                    assert!(query.parameters[2] == "owner-123");

                    Ok(module_schema.id)
                }
            }

            #[tokio::test]
            async fn test_save_the_module() {
                let mock = MockSaveTheModule;
                match mock.save_module(ModuleSchema {
                    id: "mod-123".to_string(), 
                    tags: vec![
                        RawTagSchema {
                            name: "Module-Format".to_string(),
                            value: "wasm32-unknown-emscripten".to_string()
                        }
                    ], 
                    owner: "owner-123".to_string()
                }).await {
                    Ok(id) => assert!(id == "mod-123"),
                    Err(e) => panic!("{}", e)
                }
            }
        }

        mod noop_if_the_module_already_exists {
            use super::*;

            struct MockNoopIfAlreadyExists;
            #[async_trait]
            impl SaveModuleSchema for MockNoopIfAlreadyExists {
                async fn save_module(&self, module_schema: ModuleSchema) -> Result<String, CuErrors> {
                    let query = AoModule::create_save_module_query(AoModule::to_module_doc(module_schema.clone()));

                    assert!(query.sql.contains("INSERT OR IGNORE"));

                    Ok(module_schema.id)
                }
            }

            #[tokio::test]
            async fn test_noop_if_the_module_already_exists() {
                let mock = MockNoopIfAlreadyExists;
                match mock.save_module(ModuleSchema {
                    id: "mod-123".to_string(), 
                    tags: vec![
                        RawTagSchema {
                            name: "Module-Format".to_string(),
                            value: "wasm32-unknown-emscripten".to_string()
                        }
                    ], 
                    owner: "owner-123".to_string()
                }).await {
                    Ok(id) => assert!(id == "mod-123"),
                    Err(e) => panic!("{}", e)
                }
            }
        }
    }
}