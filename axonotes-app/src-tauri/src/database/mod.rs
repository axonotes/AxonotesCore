pub(crate) mod keys;
pub(crate) mod profiles;
pub(crate) mod schema;

use crate::crypto;
use crate::database::keys::Keys;
use crate::workos_auth::Profile;
use once_cell::sync::OnceCell;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

static DB: RwLock<Option<Arc<Mutex<Database>>>> = RwLock::const_new(None);
static DB_PATH: OnceCell<PathBuf> = OnceCell::new();
static APP_CONFIG_PATH: OnceCell<PathBuf> = OnceCell::new();

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

pub struct Database {
    conn: Connection,
}

impl Database {
    fn new(conn: Connection) -> Self {
        Self { conn }
    }

    fn get_conn(&self) -> &Connection {
        &self.conn
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

    // If config file doesn't exist, create it with default
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

    // Validate: can only switch between pin and pass
    if (current_mode == "pin" && new_mode != "pass")
        || (current_mode == "pass" && new_mode != "pin")
    {
        return Err(format!(
            "Can only switch between 'pin' and 'pass'. Current mode: {}, requested: {}",
            current_mode, new_mode
        ));
    }

    set_unlock_mode(new_mode)?;
    Ok(())
}

/// Unlock and initialize the database
pub async fn unlock_db(password: String) -> Result<(), String> {
    let mut db_guard = DB.write().await;

    // Check if already unlocked
    if db_guard.is_some() {
        return Ok(());
    }

    let db_path = DB_PATH.get().ok_or("Database path not initialized")?;

    // Open connection
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    // Set encryption key if needed
    // The password string is allowed to be empty
    let key = crypto::hash::derive_database_key(password.as_str());
    let key_hex = hex::encode_upper(&key);

    conn.execute_batch(&format!("PRAGMA key = \"x'{}'\";", key_hex))
        .map_err(|e| format!("Failed to set encryption key: {}", e))?;

    // Initialize the database by creating the schema or performing a write operation
    conn.execute_batch("CREATE TABLE IF NOT EXISTS _version (value INTEGER);")
        .map_err(|_| "Failed to unlock database. Incorrect password?".to_string())?;

    // Initialize schema
    schema::init_schema(&conn).map_err(|e| format!("Failed to initialize schema: {}", e))?;

    // Store in global state
    let db = Database::new(conn);
    *db_guard = Some(Arc::new(Mutex::new(db)));

    Ok(())
}

/// Lock the database (clear the connection)
pub async fn lock_db() -> Result<(), String> {
    let mut db = DB.write().await;
    *db = None;
    Ok(())
}

/// Check if database is unlocked
pub async fn is_unlocked() -> bool {
    let db = DB.read().await;
    db.is_some()
}

/// Wipe the database (delete file and reset to 'none' mode)
pub async fn wipe_db() -> Result<(), String> {
    let db_path = DB_PATH.get().ok_or("Database path not initialized")?;

    // Delete database file
    if db_path.exists() {
        fs::remove_file(db_path).map_err(|e| format!("Failed to delete database: {}", e))?;
    }

    // Reset unlock mode to none
    set_unlock_mode("none".to_string())?;

    Ok(())
}

async fn reencrypt(new_password: &str) -> Result<(), String> {
    let db = get_db().await?;

    // Set new encryption key
    let new_key = crypto::hash::derive_database_key(new_password);
    let new_key_hex = hex::encode_upper(&new_key);

    // Re-encrypt DB
    let db_guard = db.lock().await;
    let conn = db_guard.get_conn();

    conn.execute_batch(&format!("PRAGMA rekey = \"x'{}'\";", new_key_hex))
        .map_err(|e| format!("Failed to rekey database: {}", e))?;

    Ok(())
}

/// Set encryption on database (transition from 'none' to 'pin'/'pass')
pub async fn set_encryption(new_password: String, mode: String) -> Result<(), String> {
    // Validate mode
    if mode != "pin" && mode != "pass" {
        return Err("Mode must be 'pin' or 'pass'".to_string());
    }

    // Re-encrypt
    reencrypt(new_password.as_str()).await?;

    // Update unlock mode to the specified mode
    set_unlock_mode(mode)?;

    Ok(())
}

/// Remove encryption from database (transition from 'pin'/'pass' to 'none')
pub async fn remove_encryption() -> Result<(), String> {
    // Just set encryption with empty new password
    reencrypt("").await?;

    // Update unlock mode to "none"
    set_unlock_mode("none".to_string())?;

    Ok(())
}

// ========================================
// Profile Operations (proxies to profiles module)
// ========================================

pub async fn get_active_profile() -> Result<Option<Profile>, String> {
    let db = get_db().await?;
    let db = db.lock().await;
    profiles::get_active(db.get_conn()).map_err(|e| e.to_string())
}

pub async fn get_all_profiles() -> Result<Vec<Profile>, String> {
    let db = get_db().await?;
    let db = db.lock().await;
    profiles::get_all(db.get_conn()).map_err(|e| e.to_string())
}

pub async fn save_profile(profile: Profile, set_active: bool) -> Result<(), String> {
    let db = get_db().await?;
    let db = db.lock().await;
    profiles::save(db.get_conn(), &profile, set_active).map_err(|e| e.to_string())
}

pub async fn set_active_profile(profile_id: String) -> Result<(), String> {
    let db = get_db().await?;
    let db = db.lock().await;
    profiles::set_active(db.get_conn(), &profile_id).map_err(|e| e.to_string())
}

pub async fn delete_profile(profile_id: String) -> Result<(), String> {
    let db = get_db().await?;
    let db = db.lock().await;
    profiles::delete(db.get_conn(), &profile_id).map_err(|e| e.to_string())
}

pub async fn refresh_active_profile_token() -> Result<String, String> {
    let profile = get_active_profile().await?.ok_or("No active profile")?;

    let refresh_token = profile.refresh_token.ok_or("No refresh token available")?;

    let new_access_token = crate::workos_auth::refresh_access_token(&refresh_token).await?;

    let db = get_db().await?;
    let db = db.lock().await;
    profiles::update_access_token(db.get_conn(), &profile.id, &new_access_token)
        .map_err(|e| e.to_string())?;

    Ok(new_access_token)
}

// ========================================
// Keys Operations (proxies to keys module)
// ========================================

pub async fn save_keys(keys: Keys) -> Result<(), String> {
    let db = get_db().await?;
    let db = db.lock().await;
    keys::save(db.get_conn(), keys).map_err(|e| e.to_string())
}

pub async fn save_active_user_keys(keys: Keys) -> Result<(), String> {
    let db = get_db().await?;
    let db = db.lock().await;
    keys::save_active_user_keys(db.get_conn(), keys).map_err(|e| e.to_string())
}

pub async fn get_keys(user_id: &str) -> Result<Option<Keys>, String> {
    let db = get_db().await?;
    let db = db.lock().await;
    keys::get_keys(db.get_conn(), user_id).map_err(|e| e.to_string())
}

pub async fn get_active_user_keys() -> Result<Option<Keys>, String> {
    let db = get_db().await?;
    let db = db.lock().await;
    keys::get_active_user_keys(db.get_conn()).map_err(|e| e.to_string())
}

// ========================================
// Internal Helpers
// ========================================

async fn get_db() -> Result<Arc<Mutex<Database>>, String> {
    let db = DB.read().await;
    db.as_ref()
        .ok_or_else(|| "Database not initialized. Call unlock_db first.".to_string())
        .cloned()
}

fn read_app_config() -> Result<AppConfig, String> {
    let config_path = APP_CONFIG_PATH.get().ok_or("Config path not initialized")?;

    if !config_path.exists() {
        return Ok(AppConfig::default());
    }

    let contents = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    toml::from_str(&contents).map_err(|e| format!("Failed to parse config: {}", e))
}

fn write_app_config(config: &AppConfig) -> Result<(), String> {
    let config_path = APP_CONFIG_PATH.get().ok_or("Config path not initialized")?;

    let contents = toml::to_string(config).map_err(|e| e.to_string())?;
    fs::write(config_path, contents).map_err(|e| e.to_string())?;

    Ok(())
}
