//! Request and response types for the Storage API.

#![allow(clippy::must_use_candidate)] // Type methods - callers understand return semantics

use serde::{Deserialize, Serialize};

// ============================================================================
// Document Types
// ============================================================================

/// Request body for creating a new document.
#[derive(Debug, Clone, Serialize)]
pub struct CreateDocumentRequest {
    /// Unique identifier for the document.
    pub document_id: String,
    /// Hex-encoded Ed25519 public key for signature verification.
    pub public_key: String,
}

/// Request body for updating a document's public key.
#[derive(Debug, Clone, Serialize)]
pub struct UpdatePublicKeyRequest {
    /// New hex-encoded Ed25519 public key.
    pub public_key: String,
}

// ============================================================================
// Blob Types
// ============================================================================

/// Parameters for uploading a blob.
///
/// The signature must be created by signing `hash || document_id || timestamp`
/// with the Ed25519 private key corresponding to the document's public key.
#[derive(Debug, Clone)]
pub struct UploadBlobParams {
    /// Document ID that owns this blob.
    pub document_id: String,
    /// Hex-encoded Ed25519 signature of `hash || document_id || timestamp`.
    pub signature: String,
    /// Unix timestamp (seconds since epoch) when the signature was created.
    /// Must be within 5 minutes of the server's current time.
    pub timestamp: i64,
}

/// Response from a successful blob upload.
#[derive(Debug, Clone, Deserialize)]
pub struct UploadBlobResponse {
    /// Blake3 hash of the uploaded blob (hex-encoded, 64 characters).
    pub hash: String,
}

/// Response from a successful blob download.
#[derive(Debug, Clone)]
pub struct DownloadBlobResponse {
    /// Raw blob data.
    pub data: Vec<u8>,
    /// Content length in bytes.
    pub content_length: u64,
}

// ============================================================================
// Quota Types
// ============================================================================

/// Response from the quota endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct QuotaResponse {
    /// Total quota in bytes.
    pub quota_bytes: u64,
    /// Currently used storage in bytes.
    pub used_bytes: u64,
    /// Available storage in bytes.
    pub available_bytes: u64,
    /// The quota rule that matched (e.g., "free", "pro").
    pub matched_rule: String,
}

// ============================================================================
// Error Response Types
// ============================================================================

/// Generic error response from the API.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    /// Error message.
    #[serde(default)]
    pub error: Option<String>,
    /// Alternative error message field.
    #[serde(default)]
    pub message: Option<String>,
}

impl ErrorResponse {
    /// Get the error message from either field.
    pub fn get_message(&self) -> String {
        self.error
            .clone()
            .or_else(|| self.message.clone())
            .unwrap_or_else(|| "Unknown error".to_string())
    }
}
