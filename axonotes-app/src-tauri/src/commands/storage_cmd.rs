//! Tauri commands for blob storage operations.

use crate::storage::{self, MediaType};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Response from storage quota query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaInfo {
    pub quota_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub matched_rule: String,
}

/// Response from upload operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadResult {
    pub hash: String,
}

/// Response from cache info queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub size_bytes: u64,
    pub blob_count: u64,
}

// ============================================================================
// Initialization Commands
// ============================================================================

/// Initialize the storage module.
///
/// Call this after authentication to start the blob server.
#[tauri::command]
pub async fn storage_init() -> Result<(), String> {
    let config = crate::config::StorageConfig::new();
    let profile = crate::database::get_active_profile()
        .await?
        .ok_or("No active profile")?;

    storage::init(config.base_url, &profile.access_token).await
}

/// Shutdown the storage module.
#[tauri::command]
pub async fn storage_shutdown() -> Result<(), String> {
    storage::shutdown().await;
    Ok(())
}

/// Check if storage is initialized.
#[tauri::command]
pub async fn storage_is_initialized() -> Result<bool, String> {
    Ok(storage::is_initialized().await)
}

// ============================================================================
// Upload Commands
// ============================================================================

/// Upload a blob from a file path (streaming, low memory).
///
/// Use this when the user selects a file via file picker dialog.
#[tauri::command]
pub async fn upload_blob_from_path(
    doc_id: String,
    file_path: String,
    media_type: u8,
) -> Result<UploadResult, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {file_path}"));
    }

    let media_type = MediaType::from(media_type);
    let hash = storage::active_profile()
        .upload_blob_from_path(&doc_id, &path, media_type)
        .await?;

    Ok(UploadResult { hash })
}

/// Upload a blob from bytes (in-memory, for small data).
///
/// Use this when the user pastes from clipboard or drag-drops small data.
#[tauri::command]
pub async fn upload_blob_from_bytes(
    doc_id: String,
    data: Vec<u8>,
    media_type: u8,
) -> Result<UploadResult, String> {
    let media_type = MediaType::from(media_type);
    let hash = storage::active_profile()
        .upload_blob_from_bytes(&doc_id, &data, media_type)
        .await?;

    Ok(UploadResult { hash })
}

// ============================================================================
// Access Commands
// ============================================================================

/// Get URL to access a blob (for `<img src="...">`, `<video src="...">`, etc.).
#[tauri::command]
pub async fn get_blob_url(hash: String) -> Result<String, String> {
    storage::active_profile().get_blob_url(&hash).await
}

/// Pre-cache a blob (download without returning URL).
#[tauri::command]
pub async fn prefetch_blob(hash: String, doc_id: String) -> Result<(), String> {
    storage::active_profile()
        .prefetch_blob(&hash, &doc_id)
        .await
}

/// Check if a blob is cached locally.
#[tauri::command]
pub async fn is_blob_cached(hash: String) -> Result<bool, String> {
    storage::active_profile().is_cached(&hash).await
}

// ============================================================================
// Quota Commands
// ============================================================================

/// Get storage quota information.
#[tauri::command]
pub async fn get_storage_quota() -> Result<QuotaInfo, String> {
    let quota = storage::active_profile().get_quota().await?;
    Ok(QuotaInfo {
        quota_bytes: quota.quota_bytes,
        used_bytes: quota.used_bytes,
        available_bytes: quota.available_bytes,
        matched_rule: quota.matched_rule,
    })
}

// ============================================================================
// Cache Management Commands
// ============================================================================

/// Clear the entire blob cache.
#[tauri::command]
pub async fn clear_blob_cache() -> Result<u64, String> {
    storage::active_profile().clear_cache().await
}

/// Get blob cache info (size and count).
#[tauri::command]
pub async fn get_blob_cache_info() -> Result<CacheInfo, String> {
    let ctx = storage::active_profile();
    let size_bytes = ctx.get_cache_size().await?;
    let blob_count = ctx.get_cache_count().await?;

    Ok(CacheInfo {
        size_bytes,
        blob_count,
    })
}

/// Delete a single cached blob.
#[tauri::command]
pub async fn delete_cached_blob(hash: String) -> Result<(), String> {
    storage::active_profile().delete_cached_blob(&hash).await
}

/// Delete all cached blobs for a document.
#[tauri::command]
pub async fn delete_cached_blobs_for_document(doc_id: String) -> Result<u64, String> {
    storage::active_profile()
        .delete_blobs_for_document(&doc_id)
        .await
}

// ============================================================================
// Media Type Helpers
// ============================================================================

/// Get media type from file extension.
#[tauri::command]
pub fn get_media_type_from_extension(extension: String) -> u8 {
    MediaType::from_extension(&extension).to_byte()
}

/// Get MIME type string for a media type byte.
#[tauri::command]
pub fn get_mime_type(media_type: u8) -> String {
    MediaType::from(media_type).mime_type().to_string()
}
