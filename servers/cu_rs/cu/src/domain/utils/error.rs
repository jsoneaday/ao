use std::{error::Error, fmt::Display};

#[allow(unused)]
#[derive(Debug)]
pub enum CuErrors {
    BlockMeta(Option<Box<dyn Error + 'static + Send>>)
}

impl Error for CuErrors {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BlockMeta(err) => {
                match err {
                    Some(err) => Some(err.as_ref()),
                    None => None
                }
            }
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
            })
        }        
    }
}