//! # Local Database Commands
//!
//! Tauri commands for managing the local SQLite database encryption.
//!
//! ## Database Encryption
//!
//! The local database (SQLCipher) can be encrypted with a password.
//! This protects data at rest if the device is compromised.
//!
//! ## Unlock Modes
//!
//! - **password**: Require password to unlock database
//! - **auto**: Automatically unlock (no password required)
//!
//! ## Commands
//!
//! - `get_unlock_mode`: Check current unlock mode
//! - `switch_unlock_mode`: Change between password/auto modes
//! - `unlock_database`: Unlock with password
//! - `lock_database`: Lock the database
//! - `is_database_unlocked`: Check lock state
//! - `wipe_database`: Delete all local data
//! - `set_database_encryption`: Set/change database password
//! - `remove_database_encryption`: Remove password requirement

use crate::database;

#[tauri::command]
pub async fn get_unlock_mode() -> Result<String, String> {
    database::get_unlock_mode()
}

#[tauri::command]
pub async fn switch_unlock_mode(new_mode: String) -> Result<(), String> {
    database::switch_unlock_mode_ui(new_mode)
}

#[tauri::command]
pub async fn unlock_database(password: String) -> Result<(), String> {
    database::unlock_db(password).await
}

#[tauri::command]
pub async fn lock_database() -> Result<(), String> {
    database::lock_db().await
}

#[tauri::command]
pub async fn is_database_unlocked() -> Result<bool, String> {
    Ok(database::is_unlocked().await)
}

#[tauri::command]
pub async fn wipe_database() -> Result<(), String> {
    database::wipe_db().await
}

#[tauri::command]
pub async fn set_database_encryption(new_password: String, mode: String) -> Result<(), String> {
    database::set_encryption(new_password, mode).await
}

#[tauri::command]
pub async fn remove_database_encryption() -> Result<(), String> {
    database::remove_encryption().await
}
