//! Storage handler - core blob storage operations.
//!
//! Manages the storage client, local cache, and HTTP server for blob streaming.

#![allow(clippy::missing_errors_doc)] // Error conditions documented in prose
#![allow(clippy::must_use_candidate)] // Handler methods - callers understand return semantics

use super::cache;
use super::chunked_crypto;
use super::media_type::MediaType;
use super::server;
use crate::storage_bindings::{StorageClient, UploadBlobParams};
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Core storage handler.
///
/// Manages blob uploads, downloads, caching, and the HTTP server for streaming.
#[derive(Clone)]
pub struct StorageHandler {
    /// Storage API client.
    client: Arc<StorageClient>,
    /// HTTP server port (random, localhost-only).
    server_port: u16,
    /// Secret token for HTTP server authentication.
    server_token: String,
    /// Shutdown signal for the HTTP server.
    shutdown_tx: Arc<RwLock<Option<tokio::sync::oneshot::Sender<()>>>>,
}

impl StorageHandler {
    /// Create a new storage handler.
    ///
    /// Initializes the storage client and starts the HTTP server.
    #[allow(clippy::unused_async)] // Async for API consistency with other handlers
    pub async fn new(base_url: &str, jwt: &str) -> Result<Self, String> {
        let client = Arc::new(StorageClient::new(base_url, jwt));

        // Start HTTP server
        let (port, token, shutdown_tx) = server::start_blob_server(client.clone())?;

        Ok(Self {
            client,
            server_port: port,
            server_token: token,
            shutdown_tx: Arc::new(RwLock::new(Some(shutdown_tx))),
        })
    }

    /// Shutdown the HTTP server.
    pub fn shutdown(&self) {
        // Try to send shutdown signal
        if let Ok(mut guard) = self.shutdown_tx.try_write() {
            if let Some(tx) = guard.take() {
                let _ = tx.send(());
            }
        }
    }

    /// Update the JWT token.
    pub fn set_jwt(&self, jwt: &str) {
        self.client.set_jwt(jwt);
    }

    /// Get the current JWT token.
    pub fn get_jwt(&self) -> String {
        self.client.get_jwt()
    }

    /// Get the storage client for direct API access.
    pub fn client(&self) -> &StorageClient {
        &self.client
    }

    /// Get a URL for accessing a cached blob via the HTTP server.
    ///
    /// Returns `http://localhost:{port}/blob/{hash}?token={secret}`
    pub fn get_blob_url(&self, hash: &str) -> String {
        format!(
            "http://127.0.0.1:{}/blob/{}?token={}",
            self.server_port, hash, self.server_token
        )
    }

    /// Get the HTTP server port.
    pub const fn server_port(&self) -> u16 {
        self.server_port
    }

    // ========================================================================
    // Upload Operations
    // ========================================================================

    /// Upload a blob from a file path (streaming, low memory).
    ///
    /// Encrypts the file in chunks and uploads to the storage API.
    /// The encrypted blob is also cached locally.
    pub async fn upload_blob_from_path(
        &self,
        doc_id: &str,
        file_path: &Path,
        encryption_key: &[u8; 32],
        signing_key: &ed25519_dalek::SigningKey,
        media_type: MediaType,
    ) -> Result<String, String> {
        // Create temp file for encrypted data
        let temp_dir = std::env::temp_dir();
        let temp_path = temp_dir.join(format!("blob_upload_{}.tmp", uuid::Uuid::new_v4()));

        // Encrypt file to temp
        let temp_path_clone = temp_path.clone();
        let file_path_owned = file_path.to_path_buf();
        let key = *encryption_key;
        let mt = media_type;

        tokio::task::spawn_blocking(move || {
            let mut input = std::fs::File::open(&file_path_owned)
                .map_err(|e| format!("Failed to open input file: {e}"))?;
            let mut output = std::fs::File::create(&temp_path_clone)
                .map_err(|e| format!("Failed to create temp file: {e}"))?;

            chunked_crypto::encrypt_chunked(&mut input, &mut output, &key, mt)
                .map_err(|e| format!("Encryption failed: {e}"))?;

            Ok::<_, String>(())
        })
        .await
        .map_err(|e| format!("Task failed: {e}"))??;

        // Calculate hash of encrypted data
        let temp_path_clone = temp_path.clone();
        #[allow(clippy::large_stack_arrays)] // 64KB buffer is intentional for performance
        let hash = tokio::task::spawn_blocking(move || {
            let mut file = std::fs::File::open(&temp_path_clone)
                .map_err(|e| format!("Failed to open temp file: {e}"))?;
            let mut hasher = blake3::Hasher::new();
            let mut buffer = [0u8; 65536];
            loop {
                let n = file
                    .read(&mut buffer)
                    .map_err(|e| format!("Failed to read temp file: {e}"))?;
                if n == 0 {
                    break;
                }
                hasher.update(&buffer[..n]);
            }
            Ok::<_, String>(hasher.finalize().to_hex().to_string())
        })
        .await
        .map_err(|e| format!("Task failed: {e}"))??;

        // Create signature
        #[allow(clippy::cast_possible_wrap)] // Timestamp won't overflow until year 292 billion
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Time error: {e}"))?
            .as_secs() as i64;

        let signature = create_signature(&hash, doc_id, timestamp, signing_key);

        // Read encrypted data for upload
        let temp_path_clone = temp_path.clone();
        let encrypted_data = tokio::task::spawn_blocking(move || {
            std::fs::read(&temp_path_clone).map_err(|e| format!("Failed to read temp file: {e}"))
        })
        .await
        .map_err(|e| format!("Task failed: {e}"))??;

        // Upload to storage API
        let params = UploadBlobParams {
            document_id: doc_id.to_string(),
            signature,
            timestamp,
        };

        self.client
            .upload_blob(params, encrypted_data.clone())
            .await
            .map_err(|e| format!("Upload failed: {e}"))?;

        // Cache locally
        cache::cache_blob(&hash, doc_id, &encrypted_data).await?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        Ok(hash)
    }

    /// Upload a blob from bytes (in-memory, for small data).
    pub async fn upload_blob_from_bytes(
        &self,
        doc_id: &str,
        data: &[u8],
        encryption_key: &[u8; 32],
        signing_key: &ed25519_dalek::SigningKey,
        media_type: MediaType,
    ) -> Result<String, String> {
        // Encrypt data
        let encrypted_data = chunked_crypto::encrypt_bytes(data, encryption_key, media_type)
            .map_err(|e| format!("Encryption failed: {e}"))?;

        // Calculate hash
        let hash = blake3::hash(&encrypted_data).to_hex().to_string();

        // Create signature
        #[allow(clippy::cast_possible_wrap)] // Timestamp won't overflow until year 292 billion
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Time error: {e}"))?
            .as_secs() as i64;

        let signature = create_signature(&hash, doc_id, timestamp, signing_key);

        // Upload to storage API
        let params = UploadBlobParams {
            document_id: doc_id.to_string(),
            signature,
            timestamp,
        };

        self.client
            .upload_blob(params, encrypted_data.clone())
            .await
            .map_err(|e| format!("Upload failed: {e}"))?;

        // Cache locally
        cache::cache_blob(&hash, doc_id, &encrypted_data).await?;

        Ok(hash)
    }

    // ========================================================================
    // Download Operations
    // ========================================================================

    /// Download and cache a blob.
    ///
    /// If the blob is already cached, this is a no-op.
    pub async fn prefetch_blob(&self, hash: &str, doc_id: &str) -> Result<(), String> {
        // Check if already cached
        if cache::is_cached(hash) {
            return Ok(());
        }

        // Download from storage API
        let response = self
            .client
            .download_blob(hash)
            .await
            .map_err(|e| format!("Download failed: {e}"))?;

        // Cache locally
        cache::cache_blob(hash, doc_id, &response.data).await?;

        Ok(())
    }

    /// Get cached blob data (encrypted).
    pub async fn get_cached_blob(&self, hash: &str) -> Result<Option<Vec<u8>>, String> {
        cache::get_cached_blob(hash).await
    }

    /// Check if a blob is cached.
    #[allow(clippy::unused_self)] // Method is intentionally on self for API consistency
    pub fn is_cached(&self, hash: &str) -> bool {
        cache::is_cached(hash)
    }

    // ========================================================================
    // Delete Operations
    // ========================================================================

    /// Delete a blob from both cache and storage API.
    pub async fn delete_blob(&self, hash: &str) -> Result<(), String> {
        // Delete from storage API
        self.client
            .delete_blob(hash)
            .await
            .map_err(|e| format!("Delete failed: {e}"))?;

        // Delete from cache
        cache::delete_cached_blob(hash).await?;

        Ok(())
    }

    /// Delete cached blob only (not from storage API).
    pub async fn delete_cached_blob(&self, hash: &str) -> Result<(), String> {
        cache::delete_cached_blob(hash).await
    }

    /// Delete all cached blobs for a document.
    pub async fn delete_blobs_for_document(&self, doc_id: &str) -> Result<u64, String> {
        cache::delete_blobs_for_document(doc_id).await
    }

    // ========================================================================
    // Cache Management
    // ========================================================================

    /// Clear entire blob cache.
    pub async fn clear_cache(&self) -> Result<u64, String> {
        cache::clear_cache().await
    }

    /// Get total cache size in bytes.
    pub async fn get_cache_size(&self) -> Result<u64, String> {
        cache::get_cache_size().await
    }

    /// Get number of cached blobs.
    pub async fn get_cache_count(&self) -> Result<u64, String> {
        cache::get_cache_count().await
    }
}

/// Create Ed25519 signature for blob operations.
///
/// Signs: `hash || doc_id || timestamp`
fn create_signature(
    hash: &str,
    doc_id: &str,
    timestamp: i64,
    signing_key: &ed25519_dalek::SigningKey,
) -> String {
    use ed25519_dalek::Signer;

    let mut message = Vec::new();
    message.extend_from_slice(hash.as_bytes());
    message.extend_from_slice(doc_id.as_bytes());
    message.extend_from_slice(&timestamp.to_le_bytes());

    let signature = signing_key.sign(&message);
    hex::encode(signature.to_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_creation() {
        use ed25519_dalek::SigningKey;
        use rand_core::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let hash = "abc123def456";
        let doc_id = "doc-001";
        let timestamp = 1234567890i64;

        let sig = create_signature(hash, doc_id, timestamp, &signing_key);

        // Signature should be 128 hex chars (64 bytes)
        assert_eq!(sig.len(), 128);
    }
}
