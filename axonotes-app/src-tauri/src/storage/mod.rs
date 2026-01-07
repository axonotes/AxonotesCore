//! # Storage Module
//!
//! High-level wrapper around `storage_bindings` providing encrypted blob
//! storage with local caching and streaming HTTP access for the frontend.

#![allow(dead_code)] // Public API - functions will be used by frontend integration
#![allow(clippy::significant_drop_tightening)] // RwLock guards need to be held during operations
#![allow(clippy::missing_errors_doc)] // Error conditions documented in prose
#![allow(clippy::must_use_candidate)] // Module API - callers understand return semantics
//!
//! ## Features
//!
//! - **Local caching**: Blobs cached encrypted in `$APP_DATA_DIR/blob_cache/`
//! - **Streaming decryption**: HTTP server streams decrypted content to frontend
//! - **Profile-scoped context**: Operations bound to active user profile
//! - **Document lifecycle**: Auto-register/unregister documents with storage API
//!
//! ## Architecture
//!
//! ```text
//! Frontend Request: http://localhost:{port}/blob/{hash}?token={secret}
//!        ↓
//! HTTP Server (localhost only, token-protected)
//!        ↓
//! Cache Check ($APP_DATA_DIR/blob_cache/{hash}.blob)
//!        ↓ (miss)
//! Download from Storage API → Save encrypted to cache
//!        ↓
//! Stream-decrypt chunks → Stream to frontend
//! ```
//!
//! ## Usage
//!
//! ```ignore
//! // Get profile-scoped context
//! let ctx = storage::active_profile();
//!
//! // Upload a blob
//! let hash = ctx.upload_blob_from_path(&doc_id, "/path/to/image.png", MediaType::Png).await?;
//!
//! // Get URL for frontend
//! let url = ctx.get_blob_url(&hash)?;
//! ```

pub mod cache;
pub mod chunked_crypto;
mod context;
mod handler;
pub mod media_type;
mod server;

pub use context::ProfileStorageContext;
pub use handler::StorageHandler;
pub use media_type::MediaType;

use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global storage handler singleton.
static HANDLER: OnceCell<Arc<RwLock<Option<StorageHandler>>>> = OnceCell::new();

/// Initialize the global handler container.
fn get_handler_lock() -> &'static Arc<RwLock<Option<StorageHandler>>> {
    HANDLER.get_or_init(|| Arc::new(RwLock::new(None)))
}

/// Initialize the storage handler.
///
/// Must be called after database initialization and user authentication.
/// Starts the HTTP server for blob streaming.
pub async fn init(base_url: &str, jwt: &str) -> Result<(), String> {
    let handler = StorageHandler::new(base_url, jwt).await?;
    let lock = get_handler_lock();
    let mut guard = lock.write().await;
    *guard = Some(handler);
    Ok(())
}

/// Check if storage is initialized.
pub async fn is_initialized() -> bool {
    let lock = get_handler_lock();
    let guard = lock.read().await;
    guard.is_some()
}

/// Shutdown the storage handler.
///
/// Stops the HTTP server and clears the handler.
pub async fn shutdown() {
    let lock = get_handler_lock();
    let mut guard = lock.write().await;
    if let Some(handler) = guard.take() {
        handler.shutdown();
    }
}

/// Update JWT token (call after token refresh).
pub async fn set_jwt(jwt: &str) -> Result<(), String> {
    let lock = get_handler_lock();
    let guard = lock.read().await;
    let handler = guard
        .as_ref()
        .ok_or("Storage not initialized. Call storage::init first.")?;
    handler.set_jwt(jwt);
    Ok(())
}

/// Get a profile-scoped storage context for the active user.
///
/// The context provides high-level operations for blob management
/// scoped to the current user's profile.
pub const fn active_profile() -> ProfileStorageContext {
    ProfileStorageContext::new()
}

/// Get the storage handler directly (for internal use).
pub async fn get_handler() -> Result<StorageHandler, String> {
    let lock = get_handler_lock();
    let guard = lock.read().await;
    guard
        .clone()
        .ok_or_else(|| "Storage not initialized. Call storage::init first.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_not_initialized() {
        assert!(!is_initialized().await);
    }
}
