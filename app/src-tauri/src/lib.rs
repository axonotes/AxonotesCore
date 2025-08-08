use crate::app_state::AppState;

mod module_bindings;
mod user_profile;
mod app_state;

use crate::app_state::{add_and_auth_new_user, wait_for_user_auth};

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
            wait_for_user_auth
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
