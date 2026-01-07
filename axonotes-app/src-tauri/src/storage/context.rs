//! Profile-scoped storage context for high-level blob operations.
//!
//! Provides a clean API for storage operations that automatically
//! handles key lookup, encryption, and storage API registration.

#![allow(clippy::missing_errors_doc)] // Error conditions documented in prose

use super::{cache, get_handler, MediaType};
use crate::encryption::key_data;
use crate::storage_bindings::{CreateDocumentRequest, UpdatePublicKeyRequest};
use std::path::Path;

/// Profile-scoped storage context.
///
/// Provides high-level storage operations for the active user profile.
pub struct ProfileStorageContext {
    _profile_id: Option<String>,
}

impl ProfileStorageContext {
    /// Create a new context (uses active profile).
    pub(crate) const fn new() -> Self {
        Self { _profile_id: None }
    }

    // ========================================================================
    // Storage API Document Lifecycle
    // ========================================================================

    /// Register a document with the storage API.
    ///
    /// Call this when a new document is created to enable blob storage for it.
    pub async fn create_storage_document(&self, doc_id: &str) -> Result<(), String> {
        let handler = get_handler().await?;

        // Get the document's public signing key
        let public_key = key_data::get_document_public_signing_key(doc_id)
            .await?
            .ok_or("Document signing key not found")?;

        let request = CreateDocumentRequest {
            document_id: doc_id.to_string(),
            public_key: hex::encode(public_key.as_bytes()),
        };

        handler
            .client()
            .create_document(request)
            .await
            .map_err(|e| format!("Failed to create storage document: {e}"))?;

        Ok(())
    }

    /// Unregister a document from the storage API.
    ///
    /// Call this when a document is deleted to clean up storage resources.
    pub async fn delete_storage_document(&self, doc_id: &str) -> Result<(), String> {
        let handler = get_handler().await?;

        handler
            .client()
            .delete_document(doc_id)
            .await
            .map_err(|e| format!("Failed to delete storage document: {e}"))?;

        // Also delete cached blobs for this document
        cache::delete_blobs_for_document(doc_id).await?;

        Ok(())
    }

    /// Update a document's public key after key rotation.
    ///
    /// Call this when document keys are rotated to maintain blob upload capability.
    pub async fn update_storage_document_public_key(&self, doc_id: &str) -> Result<(), String> {
        let handler = get_handler().await?;

        // Get the new document's public signing key
        let public_key = key_data::get_document_public_signing_key(doc_id)
            .await?
            .ok_or("Document signing key not found")?;

        let request = UpdatePublicKeyRequest {
            public_key: hex::encode(public_key.as_bytes()),
        };

        handler
            .client()
            .update_document_public_key(doc_id, request)
            .await
            .map_err(|e| format!("Failed to update storage document key: {e}"))?;

        Ok(())
    }

    // ========================================================================
    // Blob Upload Operations
    // ========================================================================

    /// Upload a blob from a file path (streaming, low memory usage).
    ///
    /// Use this when the user selects a file via file picker dialog.
    pub async fn upload_blob_from_path(
        &self,
        doc_id: &str,
        file_path: &Path,
        media_type: MediaType,
    ) -> Result<String, String> {
        let handler = get_handler().await?;

        // Get document keys
        let encryption_key = key_data::get_document_encryption_key(doc_id)
            .await?
            .ok_or("Document encryption key not found")?;

        let signing_key = key_data::get_document_signing_key(doc_id)
            .await?
            .ok_or("Document signing key not found (user may not have edit access)")?;

        handler
            .upload_blob_from_path(doc_id, file_path, &encryption_key, &signing_key, media_type)
            .await
    }

    /// Upload a blob from bytes (in-memory, for small data).
    ///
    /// Use this when the user pastes from clipboard or drag-drops small images.
    pub async fn upload_blob_from_bytes(
        &self,
        doc_id: &str,
        data: &[u8],
        media_type: MediaType,
    ) -> Result<String, String> {
        let handler = get_handler().await?;

        // Get document keys
        let encryption_key = key_data::get_document_encryption_key(doc_id)
            .await?
            .ok_or("Document encryption key not found")?;

        let signing_key = key_data::get_document_signing_key(doc_id)
            .await?
            .ok_or("Document signing key not found (user may not have edit access)")?;

        handler
            .upload_blob_from_bytes(doc_id, data, &encryption_key, &signing_key, media_type)
            .await
    }

    // ========================================================================
    // Blob Access Operations
    // ========================================================================

    /// Get URL for accessing a blob via the local HTTP server.
    ///
    /// Returns a URL like `http://127.0.0.1:{port}/blob/{hash}?token={secret}`
    /// that can be used in `<img src="...">`, `<video src="...">`, etc.
    pub async fn get_blob_url(&self, hash: &str) -> Result<String, String> {
        let handler = get_handler().await?;
        Ok(handler.get_blob_url(hash))
    }

    /// Pre-cache a blob (download and store locally).
    ///
    /// Call this to prefetch blobs before they're needed for display.
    pub async fn prefetch_blob(&self, hash: &str, doc_id: &str) -> Result<(), String> {
        let handler = get_handler().await?;
        handler.prefetch_blob(hash, doc_id).await
    }

    /// Check if a blob is cached locally.
    pub async fn is_cached(&self, hash: &str) -> Result<bool, String> {
        let handler = get_handler().await?;
        Ok(handler.is_cached(hash))
    }

    // ========================================================================
    // Quota Operations
    // ========================================================================

    /// Get storage quota information.
    pub async fn get_quota(&self) -> Result<crate::storage_bindings::QuotaResponse, String> {
        let handler = get_handler().await?;
        handler
            .client()
            .get_quota()
            .await
            .map_err(|e| format!("Failed to get quota: {e}"))
    }

    // ========================================================================
    // Cache Management
    // ========================================================================

    /// Clear the entire blob cache.
    pub async fn clear_cache(&self) -> Result<u64, String> {
        let handler = get_handler().await?;
        handler.clear_cache().await
    }

    /// Get total cache size in bytes.
    pub async fn get_cache_size(&self) -> Result<u64, String> {
        let handler = get_handler().await?;
        handler.get_cache_size().await
    }

    /// Get number of cached blobs.
    pub async fn get_cache_count(&self) -> Result<u64, String> {
        let handler = get_handler().await?;
        handler.get_cache_count().await
    }

    /// Delete a single cached blob.
    pub async fn delete_cached_blob(&self, hash: &str) -> Result<(), String> {
        let handler = get_handler().await?;
        handler.delete_cached_blob(hash).await
    }

    /// Delete all cached blobs for a document.
    pub async fn delete_blobs_for_document(&self, doc_id: &str) -> Result<u64, String> {
        let handler = get_handler().await?;
        handler.delete_blobs_for_document(doc_id).await
    }
}
