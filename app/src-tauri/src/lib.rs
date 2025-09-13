use crate::app_state::AppState;

mod module_bindings;
mod user_profile;
mod app_state;

use crate::app_state::{add_and_auth_new_user, wait_for_user_auth, refresh_user_token, remove_user};

/// Bootstraps and runs the Tauri application.
///
/// This function initializes logging, configures the Tauri `Builder` (registering the
/// opener plugin, managing application state, and wiring RPC handlers), and then starts
/// the application event loop. When compiled for mobile, this function is the mobile
/// entry point.
///
/// # Examples
///
/// ```no_run
/// // Start the Tauri application (blocks until the app exits).
/// app_src_tauri::run();
/// ```
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    env_logger::init();

    let builder = tauri::Builder::default();

    builder
        .plugin(tauri_plugin_opener::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            add_and_auth_new_user,
            wait_for_user_auth,
            refresh_user_token,
            remove_user
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
