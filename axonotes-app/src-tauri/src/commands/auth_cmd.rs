//! # Authentication Commands
//!
//! Tauri commands for user authentication via WorkOS OAuth.
//!
//! ## Login Flow
//!
//! 1. Frontend calls `start_login` with dark mode preference
//! 2. Opens WorkOS OAuth page in browser
//! 3. User authenticates with their identity provider
//! 4. Callback saves profile to database and emits `login-success` event
//! 5. Frontend receives profile and proceeds to app
//!
//! ## Events
//!
//! - `login-success`: Emitted with user profile on successful auth
//! - `login-error`: Emitted with error message on auth failure

use crate::{database, workos_auth};
use tauri::{AppHandle, Emitter};

/// Initiates the OAuth login flow via WorkOS.
///
/// Opens the authentication URL in the user's browser. The result is
/// delivered asynchronously via Tauri events.
///
/// # Arguments
///
/// * `app` - Tauri app handle for event emission
/// * `dark_mode` - Whether to use dark mode for the auth page
///
/// # Returns
///
/// The OAuth authorization URL to open in browser.
#[tauri::command]
pub async fn start_login(app: AppHandle, dark_mode: bool) -> Result<String, String> {
    let auth_url = workos_auth::start_auth_flow(dark_mode, move |result| {
        match result {
            Ok(profile) => {
                // Save profile to database and set as active
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = database::save_profile(profile.clone(), true).await {
                        eprintln!("Failed to save profile: {e}");
                        return;
                    }

                    // Emit event to frontend
                    let _ = app_clone.emit("login-success", &profile);
                });
            }
            Err(error) => {
                eprintln!("Login failed: {error}");
                let _ = app.emit("login-error", &error);
            }
        }
    })
    .await?;

    Ok(auth_url)
}

/// Logs out a user by deleting their profile.
///
/// # Arguments
///
/// * `profile_id` - The WorkOS profile ID to delete
#[tauri::command]
pub async fn logout(profile_id: String) -> Result<(), String> {
    database::delete_profile(profile_id).await
}
