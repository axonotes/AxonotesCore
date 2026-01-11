//! # Profile Management Commands
//!
//! Tauri commands for managing user profiles.
//!
//! ## Multi-Profile Support
//!
//! Axonotes supports multiple user profiles on a single device.
//! Each profile has its own:
//! - Email and identity
//! - OAuth tokens
//! - Encryption keys (stored separately in `keys` table)
//!
//! ## Commands
//!
//! - `get_active_profile`: Get the currently selected profile
//! - `get_all_profiles`: List all profiles on this device
//! - `switch_profile`: Change the active profile
//! - `refresh_token`: Refresh the active profile's OAuth token

use crate::{database, stdb, workos_auth};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendProfile {
    pub id: String,
    pub email: String,
    pub name: String,
}

impl From<workos_auth::Profile> for FrontendProfile {
    fn from(p: workos_auth::Profile) -> Self {
        FrontendProfile {
            id: p.id,
            email: p.email,
            name: p.name,
        }
    }
}

#[tauri::command]
pub async fn get_active_profile() -> Result<Option<FrontendProfile>, String> {
    database::get_active_profile()
        .await
        .map_err(|e| e.to_string())
        .map(|opt| opt.map(|workos_profile| workos_profile.into()))
}

#[tauri::command]
pub async fn get_all_profiles() -> Result<Vec<FrontendProfile>, String> {
    database::get_all_profiles()
        .await
        .map_err(|e| e.to_string())
        .map(|profiles| profiles.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn switch_profile(profile_id: String) -> Result<(), String> {
    database::set_active_profile(profile_id).await
}

#[tauri::command]
pub async fn refresh_token() -> Result<String, String> {
    database::refresh_active_profile_token().await
}

/// Returns the current user's SpacetimeDB identity as a byte array.
///
/// This is used for block authorship and other identity-dependent operations.
#[tauri::command]
pub async fn get_current_identity() -> Result<Vec<u8>, String> {
    let identity = stdb::active_profile()
        .get_identity()
        .await?
        .ok_or("No identity available")?;
    Ok(identity.to_byte_array().to_vec())
}
