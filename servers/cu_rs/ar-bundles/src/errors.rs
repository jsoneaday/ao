use std::fmt::Display;

#[derive(Debug)]
pub enum ArBundleErrors {
    KeyCreationFailed(Option<Box<dyn std::error::Error + 'static + Send>>),
    JsonWebKeyError(jsonwebkey::Error),
    ReadKeyPairFileFailed(Box<dyn std::error::Error + 'static + Send>)
}

impl Display for ArBundleErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeyCreationFailed(e) => write!(
                f, 
                "Key creation failed: {}", 
                match e {
                    Some(e) => e.to_string(),
                    None => "".to_string()
                }
            ),
            Self::JsonWebKeyError(e) => write!(f, "JsonWebKey error: {}", e.to_string()),
            Self::ReadKeyPairFileFailed(e) => write!(f, "Read keypair file failed: {}", e.to_string())
        }
    }
}

impl std::error::Error for ArBundleErrors {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::KeyCreationFailed(e) => match e {
                Some(e) => Some(e.as_ref()),
                None => None
            },
            Self::JsonWebKeyError(e) => Some(e),
            Self::ReadKeyPairFileFailed(e) => Some(e.as_ref())
        }
    }
}