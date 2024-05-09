use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct SchemaValidationError {
    pub message: String
}

impl Error for SchemaValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl Display for SchemaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)     
    }
}

#[derive(Debug)]
pub struct HttpError {
    pub status: u32,
    pub message: String
}

impl Error for HttpError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}

impl Display for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)     
    }
}

#[allow(unused)]
#[derive(Debug)]
pub enum CuErrors {
    BlockMeta(Option<Box<dyn Error + 'static + Send>>),
    SchemaValidation(SchemaValidationError),
    HttpStatus(HttpError),
    DatabaseError(sqlx::error::Error)
}

impl Error for CuErrors {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BlockMeta(err) => {
                match err {
                    Some(err) => Some(err.as_ref()),
                    None => None
                }
            },
            Self::SchemaValidation(err) => Some(err),
            Self::HttpStatus(err) => Some(err),
            Self::DatabaseError(err) => Some(err)
        }
    }
}

impl Display for CuErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlockMeta(e) => write!(f, "{}", if let Some(err) = e {
                    err.as_ref().to_string()
                } else {
                    "".to_string()
                }
            ),
            Self::SchemaValidation(err) => write!(f, "{}", err.message),
            Self::HttpStatus(err) => write!(f, "{}", err.message),
            Self::DatabaseError(err) => write!(f, "{}", err.to_string())
        }        
    }
}