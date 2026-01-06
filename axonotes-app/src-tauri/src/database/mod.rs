#![allow(dead_code)]
#![allow(clippy::needless_pass_by_value)] // API design: database functions often take ownership for simplicity

pub(crate) mod batches;
mod helpers;
pub(crate) mod keys;
pub(crate) mod profiles;
pub(crate) mod schema;
pub(crate) mod snapshots;

use crate::batch_handler::block_getter::{invalidate_block_cache, invalidate_doc_cache};
use crate::crypto;
use crate::database::keys::Keys;
use crate::encryption::batch::DecryptedBatch;
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

// Store the key for connection initialization
static DB_KEY: RwLock<Option<String>> = RwLock::new(None);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub unlock_mode: String, // "none", "pin", "pass"
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

    DB_PATH
        .set(db_path)
        .map_err(|_| "Database path already initialized")?;

    APP_CONFIG_PATH
        .set(config_path)
        .map_err(|_| "Config path already initialized")?;

    Ok(())
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

    Ok(())
}

/// Lock the database (clear the pool)
pub async fn lock_db() -> Result<(), String> {
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
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        profiles::save(&conn, &profile, set_active).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub async fn set_active_profile(profile_id: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        profiles::set_active(&conn, profile_id.as_str()).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
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
    let new_access_token = crate::workos_auth::refresh_access_token(&refresh_token).await?;
    let new_access_token_copy = new_access_token.clone();

    let profile_id = profile.id.clone();
    tokio::task::spawn_blocking(move || {
        let conn = get_conn()?;
        profiles::update_access_token(&conn, &profile_id, &new_access_token_copy)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(new_access_token)
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
