use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// User with quota tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub user_id: Vec<u8>, // BLAKE3 hash (32 bytes)
    pub used_bytes: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub document_id: String,
    pub owner_id: Vec<u8>,   // User ID (BLAKE3 hash)
    pub public_key: Vec<u8>, // Ed25519 public key (32 bytes)
    pub created_at: i64,
}

/// Blob ownership tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobOwnership {
    pub hash: String,     // BLAKE3 hash (hex string, 64 chars)
    pub user_id: Vec<u8>, // User ID
    pub document_id: String,
    pub size_bytes: i64,
    pub created_at: i64,
}

/// Upload request data
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct UploadRequest {
    pub document_id: String,
    pub signature: String, // Hex-encoded Ed25519 signature (128 chars)
    pub timestamp: i64,
}

/// Upload response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UploadResponse {
    /// BLAKE3 hash of the uploaded blob (hex-encoded, 64 chars)
    pub hash: String,
    /// Size of the blob in bytes
    pub size: u64,
    /// Unix timestamp when the blob was uploaded
    pub uploaded_at: i64,
}

/// Quota information response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct QuotaResponse {
    /// User ID (BLAKE3 hash of sub||iss, hex-encoded)
    pub user_id: String,
    /// User's plan name (extracted from JWT claims)
    pub plan: Option<String>,
    /// Total quota in bytes
    pub quota_bytes: u64,
    /// Currently used bytes
    pub used_bytes: u64,
    /// Available bytes remaining
    pub available_bytes: u64,
    /// Name of the quota rule that matched
    pub matched_rule: String,
}

/// Document creation request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct CreateDocumentRequest {
    /// Unique document identifier
    pub document_id: String,
    /// Ed25519 public key (hex-encoded, 64 chars)
    pub public_key: String,
}

/// Update public key request
#[derive(Debug, Clone, Deserialize, ToSchema)]
pub struct UpdatePublicKeyRequest {
    /// New Ed25519 public key (hex-encoded, 64 chars)
    pub new_public_key: String,
    /// Ed25519 signature of new_public_key signed with old private key (hex-encoded, 128 chars)
    pub signature: String,
}

/// Update public key response
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct UpdatePublicKeyResponse {
    /// Document ID that was updated
    pub document_id: String,
    /// Whether the public key was successfully updated
    pub public_key_updated: bool,
}
