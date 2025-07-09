use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;

mod auth;
mod commands;
mod config;
mod state;

use crate::commands::{
    get_auth_state, get_config_info, get_initiate_auth_url,
    handle_auth_callback, logout, refresh_token, update_config,
};
use config::AppConfig;
use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    env_logger::init();

    // Load configuration
    let config = AppConfig::load().expect("Failed to load configuration");
    log::info!(
        "Loaded configuration from: {}",
        AppConfig::get_config_path_string()
    );

    // Create application state
    let app_state = AppState::new(config);

    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(
            |app, args, cwd| {
                let _ = app
                    .get_webview_window("main")
                    .expect("no main window")
                    .set_focus();
            },
        ));
    }

    builder
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[cfg(desktop)]
            app.deep_link().register("axonotes")?;
            Ok(())
        })
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_auth_state,
            get_initiate_auth_url,
            logout,
            handle_auth_callback,
            get_config_info,
            update_config,
            refresh_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
