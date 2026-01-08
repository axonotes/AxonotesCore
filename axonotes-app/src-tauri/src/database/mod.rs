//! # Local Database Module
//!
//! Provides encrypted local storage using SQLCipher (SQLite with AES-256 encryption).
//!
//! ## Features
//!
//! - **At-rest encryption**: All data encrypted with user's password via SQLCipher
//! - **Connection pooling**: R2D2 pool for concurrent database access
//! - **WAL mode**: Write-Ahead Logging for better concurrency
//! - **Offline support**: Full functionality without network connection
//!
//! ## Submodules
//!
//! - **`batches`**: Document batch storage (patches, snapshots)
//! - **`keys`**: User encryption key storage
//! - **`profiles`**: User profile and authentication data
//! - **`schema`**: Database schema initialization and migrations
//! - **`snapshots`**: Document state snapshots for version history
//! - **`workspaces`**: UI workspace configurations (Dockview layouts)
//!
//! ## Security Model
//!
//! The database key is derived from the user's password using Argon2id.
//! Three unlock modes are supported:
//! - `"none"`: No password (empty key, not recommended for production)
//! - `"pin"`: Short numeric PIN
//! - `"pass"`: Full password
//!
//! ## Usage
//!
//! ```ignore
//! // Initialize paths (call once at startup)
//! init_paths(app_data_dir)?;
//!
//! // Unlock the database
//! unlock_db("user_password".to_string()).await?;
//!
//! // Use database operations...
//! let profile = get_active_profile().await?;
//!
//! // Lock when done
//! lock_db().await?;
//! ```

#![allow(dead_code)]
#![allow(clippy::needless_pass_by_value)] // API design: database functions often take ownership for simplicity

pub(crate) mod batches;
pub(crate) mod document_keys;
pub(crate) mod documents;
mod helpers;
pub(crate) mod keys;
pub(crate) mod permissions;
pub(crate) mod profiles;
pub(crate) mod schema;
pub(crate) mod snapshots;
pub(crate) mod workspaces;

use crate::batch_handler::block_getter::{invalidate_block_cache, invalidate_doc_cache};
use crate::config::StorageConfig;
use crate::crypto;
use crate::database::keys::Keys;
pub use crate::database::workspaces::Workspace;
use crate::encryption::batch::DecryptedBatch;
use crate::storage;
use crate::workos_auth;
use crate::workos_auth::Profile;
use once_cell::sync::OnceCell;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OpenFlags;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::RwLock;

type DbPool = Pool<SqliteConnectionManager>;

static DB_POOL: RwLock<Option<DbPool>> = RwLock::new(None);
static DB_PATH: OnceCell<PathBuf> = OnceCell::new();
static APP_CONFIG_PATH: OnceCell<PathBuf> = OnceCell::new();
static BLOB_CACHE_DIR: OnceCell<PathBuf> = OnceCell::new();

// Store the key for connection initialization
static DB_KEY: RwLock<Option<String>> = RwLock::new(None);

/// Application configuration stored in a TOML file.
///
/// This is separate from the encrypted database to allow reading
/// the unlock mode before the database is decrypted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// The authentication mode: `"none"`, `"pin"`, or `"pass"`
    pub unlock_mode: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            unlock_mode: "none".to_string(),
        }
    }
}

/// Initialize database paths (call this first in Tauri setup)
pub fn init_paths(app_data_dir: PathBuf) -> Result<(), String> {
    let db_path = app_data_dir.join("app.db");
    let config_path = app_data_dir.join("app_config.toml");
    let cache_dir = app_data_dir.join("blob_cache");

    // Create blob cache directory if it doesn't exist
    fs::create_dir_all(&cache_dir).map_err(|e| format!("Failed to create blob cache dir: {e}"))?;

    DB_PATH
        .set(db_path)
        .map_err(|_| "Database path already initialized")?;

    APP_CONFIG_PATH
        .set(config_path)
        .map_err(|_| "Config path already initialized")?;

    BLOB_CACHE_DIR
        .set(cache_dir)
        .map_err(|_| "Blob cache dir already initialized")?;

    Ok(())
}

/// Get the blob cache directory path
pub fn get_blob_cache_dir() -> &'static PathBuf {
    BLOB_CACHE_DIR
        .get()
        .expect("Blob cache dir not initialized. Call init_paths first.")
}

/// Get the current unlock mode from config file
pub fn get_unlock_mode() -> Result<String, String> {
    let config = read_app_config()?;

    let config_path = APP_CONFIG_PATH.get().ok_or("Config path not initialized")?;
    if !config_path.exists() {
        write_app_config(&config)?;
    }

    Ok(config.unlock_mode)
}

/// Set unlock mode (backend only - can set any mode including "none")
pub fn set_unlock_mode(mode: String) -> Result<(), String> {
    if mode != "none" && mode != "pin" && mode != "pass" {
        return Err("Invalid unlock mode. Must be 'none', 'pin', or 'pass'".to_string());
    }

    let mut config = read_app_config().unwrap_or_default();
    config.unlock_mode = mode;
    write_app_config(&config)?;

    Ok(())
}

/// Switch unlock mode UI (frontend - can only switch between pin ↔ pass)
pub fn switch_unlock_mode_ui(new_mode: String) -> Result<(), String> {
    let current_mode = get_unlock_mode()?;

    if (current_mode == "pin" && new_mode != "pass")
        || (current_mode == "pass" && new_mode != "pin")
    {
        return Err(format!(
            "Can only switch between 'pin' and 'pass'. Current mode: {current_mode}, requested: {new_mode}"
        ));
    }

    set_unlock_mode(new_mode)?;
    Ok(())
}

/// Unlock and initialize the database with connection pool
pub async fn unlock_db(password: String) -> Result<(), String> {
    // Check if already unlocked
    {
        let pool_guard = DB_POOL.read().map_err(|e| e.to_string())?;
        if pool_guard.is_some() {
            return Ok(());
        }
    }

    let db_path = DB_PATH.get().ok_or("Database path not initialized")?;

    // Derive and store the key
    let key = crypto::hash::derive_database_key(password.as_str());
    let key_hex = hex::encode_upper(&key);

    {
        let mut key_guard = DB_KEY.write().map_err(|e| e.to_string())?;
        *key_guard = Some(key_hex.clone());
    }

    // Create connection manager with initialization function
    let manager = SqliteConnectionManager::file(db_path)
        .with_flags(
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX, // Allow multi-threaded access
        )
        .with_init(move |conn| {
            // Set encryption key
            conn.execute_batch(&format!("PRAGMA key = \"x'{key_hex}'\";"))?;
            // Enable WAL mode for better concurrency
            conn.execute_batch("PRAGMA journal_mode = WAL;")?;
            // Normal sync is safe with WAL and faster
            conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
            // Wait up to 5 seconds if database is locked
            conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
            Ok(())
        });

    // Build the pool
    let pool = Pool::builder()
        .max_size(16) // Tune based on workload
        .min_idle(Some(2)) // Keep some connections warm
        .build(manager)
        .map_err(|e| format!("Failed to create connection pool: {e}"))?;

    // Test the connection and initialize schema
    {
        let conn = pool
            .get()
            .map_err(|e| format!("Failed to get connection: {e}"))?;

        // Test that we can access the database (validates the key)
        conn.execute_batch("CREATE TABLE IF NOT EXISTS _version (value INTEGER);")
            .map_err(|_| "Failed to unlock database. Incorrect password?".to_string())?;

        // Initialize schema
        schema::init_schema(&conn).map_err(|e| format!("Failed to initialize schema: {e}"))?;
    }

    // Store pool in global state
    {
        let mut pool_guard = DB_POOL.write().map_err(|e| e.to_string())?;
        *pool_guard = Some(pool);
    }

    // Start background token refresh task now that DB is unlocked
    workos_auth::start_token_refresh_task().await;

    // Initialize storage if there's an active profile
    if let Ok(Some(profile)) = get_active_profile().await {
        let config = StorageConfig::new();
        if let Err(e) = storage::init(config.base_url, &profile.access_token).await {
            eprintln!("[database] Failed to initialize storage: {e}");
            // Don't fail unlock_db, storage can be initialized later
        } else {
            eprintln!("[database] Storage initialized");
        }
    }

    Ok(())
}

/// Lock the database (clear the pool)
pub async fn lock_db() -> Result<(), String> {
    // Shutdown storage first
    storage::shutdown().await;

    {
        let mut pool_guard = DB_POOL.write().map_err(|e| e.to_string())?;
        *pool_guard = None;
    }
    {
        let mut key_guard = DB_KEY.write().map_err(|e| e.to_string())?;
        *key_guard = None;
    }
    Ok(())
}

/// Check if database is unlocked
pub async fn is_unlocked() -> bool {
    DB_POOL.read().map(|g| g.is_some()).unwrap_or(false)
}

/// Wipe the database (delete file and reset to 'none' mode)
pub async fn wipe_db() -> Result<(), String> {
    // Lock first to close all connections
    lock_db().await?;

    let db_path = DB_PATH.get().ok_or("Database path not initialized")?;

    // Delete database file and WAL/SHM files
    if db_path.exists() {
        fs::remove_file(db_path).map_err(|e| format!("Failed to delete database: {e}"))?;
    }

    let wal_path = db_path.with_extension("db-wal");
    if wal_path.exists() {
        let _ = fs::remove_file(wal_path);
    }

    let shm_path = db_path.with_extension("db-shm");
    if shm_path.exists() {
        let _ = fs::remove_file(shm_path);
    }

    set_unlock_mode("none".to_string())?;

    Ok(())
}

async fn reencrypt(new_password: &str) -> Result<(), String> {
    let new_key = crypto::hash::derive_database_key(new_password);
    let new_key_hex = hex::encode_upper(&new_key);

    // Rekey on existing connection
    {
        let conn = get_conn()?;
        conn.execute_batch(&format!("PRAGMA rekey = \"x'{new_key_hex}'\";"))
            .map_err(|e| format!("Failed to rekey database: {e}"))?;
    } // connection returns to pool

    // Update stored key
    {
        let mut key_guard = DB_KEY.write().map_err(|e| e.to_string())?;
        *key_guard = Some(new_key_hex.clone());
    }

    // Rebuild pool with new key (old pool dropped = all connections closed)
    let db_path = DB_PATH.get().ok_or("Database path not initialized")?;

    let manager = SqliteConnectionManager::file(db_path)
        .with_flags(
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .with_init(move |conn| {
            conn.execute_batch(&format!("PRAGMA key = \"x'{new_key_hex}'\";"))?;
            conn.execute_batch("PRAGMA journal_mode = WAL;")?;
            conn.execute_batch("PRAGMA synchronous = NORMAL;")?;
            conn.execute_batch("PRAGMA busy_timeout = 5000;")?;
            Ok(())
        });

    let pool = Pool::builder()
        .max_size(16)
        .min_idle(Some(2))
        .build(manager)
        .map_err(|e| format!("Failed to create connection pool: {e}"))?;

    // Swap in new pool, old one gets dropped
    {
        let mut pool_guard = DB_POOL.write().map_err(|e| e.to_string())?;
        *pool_guard = Some(pool);
    }

    Ok(())
}

/// Set encryption on database (transition from 'none' to 'pin'/'pass')
pub async fn set_encryption(new_password: String, mode: String) -> Result<(), String> {
    if mode != "pin" && mode != "pass" {
        return Err("Mode must be 'pin' or 'pass'".to_string());
    }

    reencrypt(new_password.as_str()).await?;
    set_unlock_mode(mode)?;

    Ok(())
}

/// Remove encryption from database (transition from 'pin'/'pass' to 'none')
pub async fn remove_encryption() -> Result<(), String> {
    reencrypt("").await?;
    set_unlock_mode("none".to_string())?;

    Ok(())
}

// ========================================
// Profile Operations
// ========================================

pub async fn get_active_profile() -> Result<Option<Profile>, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        profiles::get_active(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_last_active_profile_sync_time() -> Result<Option<u128>, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        profiles::get_last_active_sync_time(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn set_last_active_profile_sync_time(last_sync_time: u128) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        profiles::set_last_active_sync_time(&conn, last_sync_time).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_all_profiles() -> Result<Vec<Profile>, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        profiles::get_all(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn save_profile(profile: Profile, set_active: bool) -> Result<(), String> {
    let access_token = profile.access_token.clone();

    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        profiles::save(&conn, &profile, set_active).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    // Initialize storage if this profile is being set as active and storage isn't initialized
    if set_active && !storage::is_initialized().await {
        let config = StorageConfig::new();
        if let Err(e) = storage::init(config.base_url, &access_token).await {
            eprintln!("[database] Failed to initialize storage on profile save: {e}");
            // Don't fail profile save, storage can be initialized later
        } else {
            eprintln!("[database] Storage initialized on profile save");
        }
    }

    Ok(())
}

pub async fn set_active_profile(profile_id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        profiles::set_active(&conn, profile_id.as_str()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    // Reinitialize storage with the new profile's token
    if let Ok(Some(profile)) = get_active_profile().await {
        // Shutdown existing storage first
        storage::shutdown().await;

        let config = StorageConfig::new();
        if let Err(e) = storage::init(config.base_url, &profile.access_token).await {
            eprintln!("[database] Failed to reinitialize storage on profile switch: {e}");
        } else {
            eprintln!("[database] Storage reinitialized for new profile");
        }
    }

    Ok(())
}

pub async fn delete_profile(profile_id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        profiles::delete(&conn, profile_id.as_str()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn refresh_active_profile_token() -> Result<String, String> {
    let profile = get_active_profile().await?.ok_or("No active profile")?;
    let refresh_token = profile.refresh_token.ok_or("No refresh token available")?;
    let result = crate::workos_auth::refresh_access_token(&refresh_token).await?;
    let new_access_token = result.access_token.clone();

    let profile_id = profile.id.clone();
    let new_refresh_token = result.refresh_token.clone();
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        profiles::update_tokens(
            &conn,
            &profile_id,
            &result.access_token,
            new_refresh_token.as_deref(),
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(new_access_token)
}

pub async fn update_profile_tokens(
    profile_id: &str,
    access_token: &str,
    refresh_token: Option<&str>,
) -> Result<(), String> {
    let profile_id = profile_id.to_string();
    let access_token = access_token.to_string();
    let refresh_token = refresh_token.map(String::from);

    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        profiles::update_tokens(&conn, &profile_id, &access_token, refresh_token.as_deref())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ========================================
// Keys Operations
// ========================================

pub async fn save_keys(keys: Keys) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        keys::save(&conn, keys).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn save_active_user_keys(keys: Keys) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        keys::save_active_user_keys(&conn, keys).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_keys(user_id: String) -> Result<Option<Keys>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        keys::get_keys(&conn, user_id.as_str()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_active_user_keys() -> Result<Option<Keys>, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        keys::get_active_user_keys(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ========================================
// Batch Operations
// ========================================

pub async fn save_batch(batch: DecryptedBatch) -> Result<(), String> {
    let doc_id = batch.doc_id.clone();
    let block_id = batch.batch_data.block_id;

    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::save(&conn, &batch).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    invalidate_block_cache(doc_id, block_id).await;
    Ok(())
}

pub async fn save_pending_batch(batch: DecryptedBatch) -> Result<(), String> {
    let doc_id = batch.doc_id.clone();
    let block_id = batch.batch_data.block_id;

    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::save_pending(&conn, &batch).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    invalidate_block_cache(doc_id, block_id).await;
    Ok(())
}

pub async fn save_batches(batches_list: Vec<DecryptedBatch>) -> Result<(), String> {
    let to_invalidate: Vec<_> = batches_list
        .iter()
        .map(|b| (b.doc_id.clone(), b.batch_data.block_id))
        .collect();

    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::save_all(&conn, &batches_list).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    for (doc_id, block_id) in to_invalidate {
        invalidate_block_cache(doc_id, block_id).await;
    }
    Ok(())
}

pub async fn get_batches_by_doc(doc_id: String) -> Result<Vec<DecryptedBatch>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::get_by_doc_id(&conn, doc_id.as_str()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_batches_by_doc_and_block(
    doc_id: String,
    block_id: u64,
) -> Result<Vec<DecryptedBatch>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::get_by_doc_and_block(&conn, doc_id.as_str(), block_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_batches_up_to_timestamp(
    doc_id: String,
    up_to: u128,
) -> Result<Vec<DecryptedBatch>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::get_up_to_timestamp(&conn, doc_id.as_str(), up_to).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_by_doc_and_block_up_to_timestamp(
    doc_id: String,
    block_id: u64,
    timestamp: u128,
) -> Result<Vec<DecryptedBatch>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::get_by_doc_and_block_up_to_timestamp(&conn, doc_id.as_str(), block_id, timestamp)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_batches_from_latest_initial_up_to_timestamp(
    doc_id: String,
    up_to: u128,
) -> Result<Vec<DecryptedBatch>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::get_from_latest_initial_up_to_timestamp(&conn, doc_id.as_str(), up_to)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_block_batches_from_latest_initial_up_to_timestamp(
    doc_id: String,
    block_id: u64,
    up_to: u128,
) -> Result<Vec<DecryptedBatch>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::get_block_from_latest_initial_up_to_timestamp(
            &conn,
            doc_id.as_str(),
            block_id,
            up_to,
        )
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_block_batches_in_range(
    doc_id: String,
    block_id: u64,
    from_ts: u128,
    to_ts: u128,
) -> Result<Vec<DecryptedBatch>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::get_block_batches_in_range(&conn, doc_id.as_str(), block_id, from_ts, to_ts)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_latest_initial_timestamp(
    doc_id: String,
    block_id: u64,
    up_to: u128,
) -> Result<Option<u128>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::get_latest_initial_timestamp(&conn, doc_id.as_str(), block_id, up_to)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_pending_batches_by_doc_and_block(
    doc_id: String,
    block_id: u64,
) -> Result<Vec<DecryptedBatch>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::get_pending_by_doc_and_block(&conn, doc_id.as_str(), block_id)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_pending_batches_by_doc(doc_id: String) -> Result<Vec<DecryptedBatch>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::get_pending_by_doc(&conn, doc_id.as_str()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_all_pending_batches() -> Result<Vec<DecryptedBatch>, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        batches::get_all_pending(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn mark_batch_synced(batch_id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::mark_synced(&conn, batch_id.as_str())
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn delete_batches_by_doc(doc_id: String) -> Result<(), String> {
    let doc_id_copy = doc_id.clone();

    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::delete_by_doc_id(&conn, doc_id_copy.as_str())
            .map(|_| ())
            .map_err(|e| e.to_string())?;
        snapshots::delete_by_doc_id(&conn, doc_id_copy.as_str())
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    invalidate_doc_cache(doc_id);
    Ok(())
}

pub async fn delete_batch(batch: &DecryptedBatch) -> Result<(), String> {
    let batch_id = batch.batch_id.clone();
    let doc_id = batch.doc_id.clone();
    let block_id = batch.batch_data.block_id;

    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::delete(&conn, &batch_id)
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    invalidate_block_cache(doc_id, block_id).await;
    Ok(())
}

pub async fn count_batches_by_doc(doc_id: String) -> Result<u64, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        batches::count_by_doc(&conn, doc_id.as_str()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn count_pending_batches() -> Result<u64, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        batches::count_pending(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ========================================
// Snapshot Operations
// ========================================

pub async fn save_snapshot(batch: DecryptedBatch) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        snapshots::save(&conn, &batch).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn delete_snapshots_by_doc(doc_id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        snapshots::delete_by_doc_id(&conn, doc_id.as_str())
            .map(|_| ())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn count_snapshots_by_doc(doc_id: String) -> Result<u64, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        snapshots::count_by_doc(&conn, doc_id.as_str()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ========================================
// Workspace Operations
// ========================================

pub async fn create_workspace(id: String, config: String) -> Result<Workspace, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        workspaces::create(&conn, &id, &config).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn get_workspace(id: String) -> Result<Option<Workspace>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        workspaces::get_by_id(&conn, &id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn list_workspaces() -> Result<Vec<Workspace>, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        workspaces::get_all(&conn).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn update_workspace(id: String, config: String) -> Result<Workspace, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        workspaces::update(&conn, &id, &config).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn delete_workspace(id: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        workspaces::delete(&conn, &id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

// ========================================
// Blob Cache Operations
// ========================================

/// Blob cache entry metadata.
#[derive(Debug, Clone)]
pub struct BlobCacheEntry {
    pub hash: String,
    pub doc_id: String,
    pub size_bytes: u64,
    pub cached_at: u64,
}

/// Insert or update blob cache metadata.
pub async fn save_blob_cache_entry(
    hash: String,
    doc_id: String,
    size_bytes: u64,
    cached_at: u64,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO blob_cache (hash, doc_id, size_bytes, cached_at) VALUES (?, ?, ?, ?)",
            rusqlite::params![hash, doc_id, size_bytes as i64, cached_at as i64],
        )
        .map_err(|e| format!("Failed to insert blob cache entry: {e}"))?;
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get blob cache entry by hash.
pub async fn get_blob_cache_entry(hash: String) -> Result<Option<BlobCacheEntry>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        let mut stmt = conn
            .prepare("SELECT hash, doc_id, size_bytes, cached_at FROM blob_cache WHERE hash = ?")
            .map_err(|e| format!("Failed to prepare statement: {e}"))?;

        use rusqlite::OptionalExtension;
        let entry = stmt
            .query_row([&hash], |row| {
                Ok(BlobCacheEntry {
                    hash: row.get(0)?,
                    doc_id: row.get(1)?,
                    size_bytes: row.get::<_, i64>(2)? as u64,
                    cached_at: row.get::<_, i64>(3)? as u64,
                })
            })
            .optional()
            .map_err(|e| format!("Failed to query blob cache: {e}"))?;

        Ok(entry)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get document ID for a cached blob.
pub async fn get_blob_doc_id(hash: String) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        use rusqlite::OptionalExtension;
        let doc_id = conn
            .query_row(
                "SELECT doc_id FROM blob_cache WHERE hash = ?",
                [&hash],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to query blob doc_id: {e}"))?;

        Ok(doc_id)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Delete blob cache entry by hash.
pub async fn delete_blob_cache_entry(hash: String) -> Result<bool, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        let rows = conn
            .execute("DELETE FROM blob_cache WHERE hash = ?", [&hash])
            .map_err(|e| format!("Failed to delete blob cache entry: {e}"))?;
        Ok(rows > 0)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Delete all blob cache entries for a document.
pub async fn delete_blob_cache_for_document(doc_id: String) -> Result<u64, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        let rows = conn
            .execute("DELETE FROM blob_cache WHERE doc_id = ?", [&doc_id])
            .map_err(|e| format!("Failed to delete blob cache entries: {e}"))?;
        Ok(rows as u64)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get all blob hashes for a document.
pub async fn get_blob_hashes_for_document(doc_id: String) -> Result<Vec<String>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        let mut stmt = conn
            .prepare("SELECT hash FROM blob_cache WHERE doc_id = ?")
            .map_err(|e| format!("Failed to prepare statement: {e}"))?;

        let hashes = stmt
            .query_map([&doc_id], |row| row.get(0))
            .map_err(|e| format!("Failed to query blob hashes: {e}"))?
            .collect::<Result<Vec<String>, _>>()
            .map_err(|e| format!("Failed to collect hashes: {e}"))?;

        Ok(hashes)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Clear all blob cache entries.
pub async fn clear_blob_cache() -> Result<u64, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        let rows = conn
            .execute("DELETE FROM blob_cache", [])
            .map_err(|e| format!("Failed to clear blob cache: {e}"))?;
        Ok(rows as u64)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get total blob cache size in bytes.
pub async fn get_blob_cache_size() -> Result<u64, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        let size: i64 = conn
            .query_row(
                "SELECT COALESCE(SUM(size_bytes), 0) FROM blob_cache",
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to query cache size: {e}"))?;
        Ok(size as u64)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get blob cache entry count.
pub async fn get_blob_cache_count() -> Result<u64, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM blob_cache", [], |row| row.get(0))
            .map_err(|e| format!("Failed to query cache count: {e}"))?;
        Ok(count as u64)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// List all blob cache entries.
pub async fn list_blob_cache_entries() -> Result<Vec<BlobCacheEntry>, String> {
    tokio::task::spawn_blocking(|| {
        let conn = get_conn()?;
        let mut stmt = conn
            .prepare(
                "SELECT hash, doc_id, size_bytes, cached_at FROM blob_cache ORDER BY cached_at DESC",
            )
            .map_err(|e| format!("Failed to prepare statement: {e}"))?;

        let entries = stmt
            .query_map([], |row| {
                Ok(BlobCacheEntry {
                    hash: row.get(0)?,
                    doc_id: row.get(1)?,
                    size_bytes: row.get::<_, i64>(2)? as u64,
                    cached_at: row.get::<_, i64>(3)? as u64,
                })
            })
            .map_err(|e| format!("Failed to query blob cache: {e}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect entries: {e}"))?;

        Ok(entries)
    })
    .await
    .map_err(|e| e.to_string())?
}

// ========================================
// STDB Sync Operations (Document Keys, Permissions, Documents)
// ========================================

use crate::encryption::document::DecryptedDocumentKey;
use crate::stdb_bindings::{Document, DocumentPermission};
use spacetimedb_sdk::Identity;

/// Sync document keys for a specific identity - clear and replace all.
/// Keys should be pre-decrypted before calling this function.
pub async fn sync_document_keys(
    identity: Identity,
    keys: Vec<DecryptedDocumentKey>,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        document_keys::sync(&conn, &identity, &keys).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Sync permissions for a specific identity - clear and replace all.
pub async fn sync_permissions(
    identity: Identity,
    perms: Vec<DocumentPermission>,
) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        permissions::sync(&conn, &identity, &perms).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Sync documents for a specific identity - clear and replace all.
pub async fn sync_documents(identity: Identity, docs: Vec<Document>) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        documents::sync(&conn, &identity, &docs).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get all document keys for a specific identity (already decrypted).
pub async fn get_document_keys_for_identity(
    identity: Identity,
) -> Result<Vec<DecryptedDocumentKey>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        document_keys::get_all_for_identity(&conn, &identity).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get document keys for a specific document and identity.
pub async fn get_document_keys_for_doc(
    identity: Identity,
    doc_id: String,
) -> Result<Vec<DecryptedDocumentKey>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        document_keys::get_by_doc_id(&conn, &identity, &doc_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get all permissions for a specific identity.
pub async fn get_permissions_for_identity(
    identity: Identity,
) -> Result<Vec<DocumentPermission>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        permissions::get_all_for_identity(&conn, &identity).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get permissions for a specific document and identity.
pub async fn get_permissions_for_doc(
    identity: Identity,
    doc_id: String,
) -> Result<Vec<DocumentPermission>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        permissions::get_by_doc_id(&conn, &identity, &doc_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get all documents for a specific identity.
pub async fn get_documents_for_identity(identity: Identity) -> Result<Vec<Document>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        documents::get_all_for_identity(&conn, &identity).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get a specific document by ID and identity.
pub async fn get_document_by_id(
    identity: Identity,
    doc_id: String,
) -> Result<Option<Document>, String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        documents::get_by_id(&conn, &identity, &doc_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Get all document keys for the active user (convenience function).
/// Gets identity from active STDB connection, reads from local SQLite.
pub async fn get_document_keys_for_active_user() -> Result<Vec<DecryptedDocumentKey>, String> {
    let identity = crate::stdb::active_profile()
        .get_identity()
        .await?
        .ok_or("No identity available")?;

    get_document_keys_for_identity(identity).await
}

/// Get document permissions for a specific document (convenience function).
/// Gets identity from active STDB connection.
pub async fn get_permissions_for_document(doc_id: String) -> Result<Vec<DocumentPermission>, String> {
    let identity = crate::stdb::active_profile()
        .get_identity()
        .await?
        .ok_or("No identity available")?;

    get_permissions_for_doc(identity, doc_id).await
}

/// Get all documents for the active user (convenience function).
/// Gets identity from active STDB connection.
pub async fn get_documents_for_active_user() -> Result<Vec<Document>, String> {
    let identity = crate::stdb::active_profile()
        .get_identity()
        .await?
        .ok_or("No identity available")?;

    get_documents_for_identity(identity).await
}

// ========================================
// Internal Helpers
// ========================================

fn get_conn() -> Result<PooledConnection<SqliteConnectionManager>, String> {
    let pool_guard = DB_POOL.read().map_err(|e| e.to_string())?;
    let pool = pool_guard
        .as_ref()
        .ok_or("Database not initialized. Call unlock_db first.")?;

    pool.get()
        .map_err(|e| format!("Failed to get connection from pool: {e}"))
}

fn read_app_config() -> Result<AppConfig, String> {
    let config_path = APP_CONFIG_PATH.get().ok_or("Config path not initialized")?;

    if !config_path.exists() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    toml::from_str(&contents).map_err(|e| format!("Failed to parse config: {e}"))
}

fn write_app_config(config: &AppConfig) -> Result<(), String> {
    let config_path = APP_CONFIG_PATH.get().ok_or("Config path not initialized")?;

    let contents = toml::to_string(config).map_err(|e| e.to_string())?;
    fs::write(config_path, contents).map_err(|e| e.to_string())?;

    Ok(())
}
