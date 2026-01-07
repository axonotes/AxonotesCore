//! Error types for the Storage API client.

use std::fmt;

/// Result type alias for storage operations.
pub type StorageResult<T> = Result<T, StorageError>;

/// Errors that can occur when interacting with the Storage API.
#[derive(Debug)]
pub enum StorageError {
    /// HTTP request failed.
    Request(reqwest::Error),

    /// Failed to parse JSON response.
    Json(serde_json::Error),

    /// Server returned an error status code.
    Api { status: u16, message: String },

    /// Authentication failed (401 Unauthorized).
    Unauthorized(String),

    /// Access denied (403 Forbidden).
    Forbidden(String),

    /// Resource not found (404 Not Found).
    NotFound(String),

    /// Conflict error (409 Conflict).
    Conflict(String),

    /// Quota exceeded (413 Payload Too Large).
    QuotaExceeded(String),

    /// Validation error (422 Unprocessable Entity).
    ValidationError(String),

    /// Rate limited (429 Too Many Requests).
    RateLimited(String),

    /// Invalid hash format.
    InvalidHash(String),

    /// Invalid timestamp.
    InvalidTimestamp(String),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::Request(e) => write!(f, "HTTP request failed: {}", e),
            StorageError::Json(e) => write!(f, "JSON parsing failed: {}", e),
            StorageError::Api { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
            StorageError::Unauthorized(msg) => write!(f, "Unauthorized: {}", msg),
            StorageError::Forbidden(msg) => write!(f, "Forbidden: {}", msg),
            StorageError::NotFound(msg) => write!(f, "Not found: {}", msg),
            StorageError::Conflict(msg) => write!(f, "Conflict: {}", msg),
            StorageError::QuotaExceeded(msg) => write!(f, "Quota exceeded: {}", msg),
            StorageError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            StorageError::RateLimited(msg) => write!(f, "Rate limited: {}", msg),
            StorageError::InvalidHash(msg) => write!(f, "Invalid hash: {}", msg),
            StorageError::InvalidTimestamp(msg) => write!(f, "Invalid timestamp: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StorageError::Request(e) => Some(e),
            StorageError::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for StorageError {
    fn from(err: reqwest::Error) -> Self {
        StorageError::Request(err)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::Json(err)
    }
}
