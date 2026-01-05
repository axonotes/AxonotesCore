use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

/// Application-level errors with HTTP status code mappings
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Payload too large: {0}")]
    PayloadTooLarge(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Internal server error: {0}")]
    Internal(String),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("S3 error: {0}")]
    S3Error(String),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid hash format")]
    InvalidHash,

    #[error("JWT error: {0}")]
    JwtError(#[from] jsonwebtoken::errors::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// JSON error response format
#[derive(Serialize)]
struct ErrorResponse {
    error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, details) = match self {
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "Unauthorized", Some(msg)),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "Forbidden", Some(msg)),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "Not Found", Some(msg)),
            AppError::PayloadTooLarge(msg) => {
                (StatusCode::PAYLOAD_TOO_LARGE, "Quota Exceeded", Some(msg))
            }
            AppError::RateLimitExceeded(msg) => (
                StatusCode::TOO_MANY_REQUESTS,
                "Rate Limit Exceeded",
                Some(msg),
            ),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "Bad Request", Some(msg)),
            AppError::InvalidSignature => (
                StatusCode::FORBIDDEN,
                "Invalid Signature",
                Some("Ed25519 signature verification failed".to_string()),
            ),
            AppError::InvalidHash => (
                StatusCode::BAD_REQUEST,
                "Invalid Hash",
                Some("Hash must be 64 hex characters".to_string()),
            ),
            AppError::Database(ref e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Database Error",
                Some(e.to_string()),
            ),
            AppError::S3Error(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Storage Error",
                Some(msg),
            ),
            AppError::JwtError(ref e) => (
                StatusCode::UNAUTHORIZED,
                "Invalid Token",
                Some(e.to_string()),
            ),
            AppError::Config(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Configuration Error",
                Some(msg),
            ),
            AppError::Io(ref e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "I/O Error",
                Some(e.to_string()),
            ),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal Server Error",
                Some(msg),
            ),
        };

        let body = Json(ErrorResponse {
            error: error_message.to_string(),
            details,
        });

        (status, body).into_response()
    }
}

/// Helper to convert anyhow errors to AppError
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, AppError>;
