//! Blob cache management with `SQLite` metadata tracking.
//!
//! Cached blobs are stored encrypted in `$APP_DATA_DIR/blob_cache/{hash}.blob`
//! with metadata tracked in the `blob_cache` `SQLite` table.

#![allow(clippy::missing_errors_doc)] // Error conditions documented in prose
#![allow(clippy::must_use_candidate)] // Internal module, callers understand return semantics

use crate::database;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

pub use database::BlobCacheEntry;

/// Get the file path for a cached blob.
pub fn get_blob_path(hash: &str) -> PathBuf {
    database::get_blob_cache_dir().join(format!("{hash}.blob"))
}

/// Check if a blob is cached (file exists).
pub fn is_cached(hash: &str) -> bool {
    get_blob_path(hash).exists()
}

/// Save encrypted blob data to cache.
///
/// Also inserts metadata into the `blob_cache` table.
pub async fn cache_blob(hash: &str, doc_id: &str, encrypted_data: &[u8]) -> Result<(), String> {
    let path = get_blob_path(hash);

    // Write file
    let data = encrypted_data.to_vec();
    let path_clone = path.clone();
    tokio::task::spawn_blocking(move || {
        let mut file = fs::File::create(&path_clone)
            .map_err(|e| format!("Failed to create cache file: {e}"))?;
        file.write_all(&data)
            .map_err(|e| format!("Failed to write cache file: {e}"))?;
        file.flush()
            .map_err(|e| format!("Failed to flush cache file: {e}"))?;
        Ok::<_, String>(())
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))??;

    // Insert metadata
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Time error: {e}"))?
        .as_secs();

    database::save_blob_cache_entry(
        hash.to_string(),
        doc_id.to_string(),
        encrypted_data.len() as u64,
        now,
    )
    .await?;

    Ok(())
}

/// Save encrypted blob data to cache from a file (streaming, avoids loading all into memory).
pub async fn cache_blob_from_file(
    hash: &str,
    doc_id: &str,
    source_path: &std::path::Path,
) -> Result<u64, String> {
    let dest_path = get_blob_path(hash);
    let source = source_path.to_path_buf();

    // Copy file
    let size = tokio::task::spawn_blocking(move || {
        fs::copy(&source, &dest_path).map_err(|e| format!("Failed to copy blob to cache: {e}"))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))??;

    // Insert metadata
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Time error: {e}"))?
        .as_secs();

    database::save_blob_cache_entry(hash.to_string(), doc_id.to_string(), size, now).await?;

    Ok(size)
}

/// Get cached blob data.
///
/// Returns `None` if the blob is not cached.
pub async fn get_cached_blob(hash: &str) -> Result<Option<Vec<u8>>, String> {
    let path = get_blob_path(hash);
    if !path.exists() {
        return Ok(None);
    }

    tokio::task::spawn_blocking(move || {
        let mut file =
            fs::File::open(&path).map_err(|e| format!("Failed to open cache file: {e}"))?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)
            .map_err(|e| format!("Failed to read cache file: {e}"))?;
        Ok(Some(data))
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?
}

/// Open a cached blob file for streaming read.
///
/// Returns `None` if the blob is not cached.
pub fn open_cached_blob(hash: &str) -> Result<Option<fs::File>, String> {
    let path = get_blob_path(hash);
    if !path.exists() {
        return Ok(None);
    }

    let file = fs::File::open(&path).map_err(|e| format!("Failed to open cache file: {e}"))?;
    Ok(Some(file))
}

/// Get the document ID for a cached blob.
pub async fn get_blob_doc_id(hash: &str) -> Result<Option<String>, String> {
    database::get_blob_doc_id(hash.to_string()).await
}

/// Get cache entry metadata.
pub async fn get_cache_entry(hash: &str) -> Result<Option<BlobCacheEntry>, String> {
    database::get_blob_cache_entry(hash.to_string()).await
}

/// Delete a cached blob.
pub async fn delete_cached_blob(hash: &str) -> Result<(), String> {
    // Delete file
    let path = get_blob_path(hash);
    if path.exists() {
        tokio::task::spawn_blocking(move || {
            fs::remove_file(&path).map_err(|e| format!("Failed to delete cache file: {e}"))
        })
        .await
        .map_err(|e| format!("Task failed: {e}"))??;
    }

    // Delete metadata
    database::delete_blob_cache_entry(hash.to_string()).await?;

    Ok(())
}

/// Delete all cached blobs for a document.
pub async fn delete_blobs_for_document(doc_id: &str) -> Result<u64, String> {
    // Get all hashes for this document
    let hashes = database::get_blob_hashes_for_document(doc_id.to_string()).await?;
    let count = hashes.len() as u64;

    // Delete files
    for hash in &hashes {
        let path = get_blob_path(hash);
        if path.exists() {
            let _ = fs::remove_file(&path);
        }
    }

    // Delete metadata
    database::delete_blob_cache_for_document(doc_id.to_string()).await?;

    Ok(count)
}

/// Clear entire cache.
pub async fn clear_cache() -> Result<u64, String> {
    let cache_dir = database::get_blob_cache_dir().clone();

    // Count and delete files
    let count = tokio::task::spawn_blocking(move || {
        let mut deleted = 0u64;
        if let Ok(entries) = fs::read_dir(&cache_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() && fs::remove_file(entry.path()).is_ok() {
                        deleted += 1;
                    }
                }
            }
        }
        deleted
    })
    .await
    .map_err(|e| format!("Task failed: {e}"))?;

    // Clear metadata table
    database::clear_blob_cache().await?;

    Ok(count)
}

/// Get total cache size in bytes.
pub async fn get_cache_size() -> Result<u64, String> {
    database::get_blob_cache_size().await
}

/// Get number of cached blobs.
pub async fn get_cache_count() -> Result<u64, String> {
    database::get_blob_cache_count().await
}

/// List all cached blobs.
pub async fn list_cached_blobs() -> Result<Vec<BlobCacheEntry>, String> {
    database::list_blob_cache_entries().await
}

#[cfg(test)]
mod tests {
    // Tests require database initialization which needs integration test setup
}
