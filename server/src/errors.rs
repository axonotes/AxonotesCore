use std::fmt;

#[derive(Debug)]
pub enum AuthError {
    UserNotFound,
    NoPublicKey,
    InvalidKeyFormat(String),
    InvalidSignature,
    DecodeError(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::UserNotFound => write!(f, "User not found"),
            AuthError::NoPublicKey => write!(f, "User has no public signing key"),
            AuthError::InvalidKeyFormat(msg) => write!(f, "Invalid key format: {}", msg),
            AuthError::InvalidSignature => write!(f, "Invalid signature"),
            AuthError::DecodeError(msg) => write!(f, "Decode error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

#[derive(Debug)]
pub enum UserError {
    AlreadyInitialized,
    NotInitialized,
    InvalidData(String),
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserError::AlreadyInitialized => write!(f, "User keys already initialized"),
            UserError::NotInitialized => write!(f, "User keys not initialized"),
            UserError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl std::error::Error for UserError {}