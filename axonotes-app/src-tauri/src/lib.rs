use tauri::Manager;
use tauri_plugin_decorum::WebviewWindowExt;
use crate::oauth::OAuthState;

mod config;
mod oauth;
mod profiles;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_decorum::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .manage(OAuthState::default())
        .invoke_handler(tauri::generate_handler![
            oauth::start_oauth_flow,
            oauth::exchange_code_for_token,
            oauth::refresh_access_token,
            profiles::get_profiles,
            profiles::add_profile,
            profiles::remove_profile,
            profiles::set_active_profile,
            profiles::get_active_profile,
            profiles::update_profile_token,
        ])
        .setup(|app| {
            let main_window = app.get_webview_window("main").unwrap();
            main_window.create_overlay_titlebar().unwrap();

            #[cfg(target_os = "macos")]
            {
                main_window.make_transparent().unwrap();
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
