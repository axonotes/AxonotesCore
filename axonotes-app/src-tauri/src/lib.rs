use tauri::Manager;
use tauri_plugin_decorum::WebviewWindowExt;

mod commands;
mod config;
mod crypto;
mod database;
mod encryption;
mod stdb;
mod stdb_bindings;
mod workos_auth;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_decorum::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::auth_cmd::start_login,
            commands::auth_cmd::logout,
            commands::database_cmd::get_unlock_mode,
            commands::database_cmd::switch_unlock_mode,
            commands::database_cmd::unlock_database,
            commands::database_cmd::is_database_unlocked,
            commands::database_cmd::wipe_database,
            commands::database_cmd::set_database_encryption,
            commands::database_cmd::remove_database_encryption,
            commands::profile_cmd::get_active_profile,
            commands::profile_cmd::get_all_profiles,
            commands::profile_cmd::switch_profile,
            commands::profile_cmd::refresh_token,
            commands::encryption_cmd::create_stdb_user,
            commands::encryption_cmd::does_stdb_user_exist,
            commands::encryption_cmd::do_stdb_keys_need_sync,
            commands::encryption_cmd::sync_stdb_keys_with_pwd,
            commands::encryption_cmd::sync_stdb_keys_with_mnemonic,
            commands::encryption_cmd::update_pwd_from_mnemonic,
            commands::encryption_cmd::update_mnemonic_from_pwd,
            commands::encryption_cmd::update_pwd_from_pwd,
            commands::encryption_cmd::update_mnemonic_from_mnemonic,
        ])
        .setup(|app| {
            let main_window = app.get_webview_window("main").unwrap();
            main_window.create_overlay_titlebar().unwrap();

            #[cfg(target_os = "macos")]
            {
                main_window.make_transparent().unwrap();
            }

            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app_data_dir.");

            database::init_paths(app_data_dir)
                .map_err(|e| format!("Failed to initialize database paths: {}", e))?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
