//! # Axonotes Application Library
//!
//! This is the core library for the Axonotes desktop application, built with Tauri.
//!
//! ## Architecture Overview
//!
//! The application follows a modular architecture with clear separation of concerns:
//!
//! - **`batch_handler`**: Manages document batches and block operations (CRDT-like sync)
//! - **`commands`**: Tauri command handlers exposed to the frontend
//! - **`crypto`**: Cryptographic primitives (ChaCha20-Poly1305, Ed25519, X25519, Argon2)
//! - **`database`**: Local SQLite database for offline storage
//! - **`encryption`**: High-level encryption/decryption for documents, batches, and user data
//! - **`events`**: Event emission to the frontend via Tauri's event system
//! - **`share`**: Document sharing and key rotation logic
//! - **`stdb`**: SpacetimeDB integration for real-time sync
//! - **`stdb_bindings`**: Generated bindings for SpacetimeDB tables and reducers
//! - **`utils`**: Common utilities (timestamps, byte array conversions)
//! - **`workos_auth`**: WorkOS-based authentication

use tauri::Manager;
use tauri_plugin_decorum::WebviewWindowExt;

mod app_handle;
mod batch_handler;
mod commands;
mod config;
mod crypto;
mod encryption;
mod events;
mod share;
mod stdb;
mod stdb_bindings;
mod utils;
mod workos_auth;

// Public modules for integration testing
pub mod database;
pub mod storage;
pub mod storage_bindings;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Debug)
                .clear_targets()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .build(),
        )
        .plugin(tauri_plugin_decorum::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            commands::auth_cmd::start_login,
            commands::auth_cmd::logout,
            commands::database_cmd::get_unlock_mode,
            commands::database_cmd::switch_unlock_mode,
            commands::database_cmd::unlock_database,
            commands::database_cmd::lock_database,
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
            commands::document_cmd::create_document,
            commands::document_cmd::delete_document,
            commands::document_cmd::get_document_meta,
            commands::document_cmd::list_documents,
            commands::document_cmd::update_document_metadata,
            commands::live_lock_cmd::request_lock,
            commands::live_lock_cmd::release_lock_focused,
            commands::live_lock_cmd::release_lock_blur,
            commands::live_lock_cmd::get_document_locks,
            commands::live_lock_cmd::update_live_block,
            commands::block_cmd::create_block,
            commands::block_cmd::get_blocks,
            commands::block_cmd::update_block,
            commands::block_cmd::delete_block,
            commands::share_cmd::create_share,
            commands::share_cmd::join_share,
            commands::share_cmd::close_share,
            commands::share_cmd::leave_share,
            commands::share_cmd::update_user_role,
            commands::share_cmd::transfer_ownership,
            commands::share_cmd::remove_user,
            commands::share_cmd::get_document_collaborators,
            commands::version_tag_cmd::create_version_tag,
            commands::version_tag_cmd::delete_version_tag,
            commands::version_tag_cmd::list_version_tags,
            commands::storage_cmd::storage_init,
            commands::storage_cmd::storage_shutdown,
            commands::storage_cmd::storage_is_initialized,
            commands::storage_cmd::upload_blob_from_path,
            commands::storage_cmd::upload_blob_from_bytes,
            commands::storage_cmd::get_blob_url,
            commands::storage_cmd::prefetch_blob,
            commands::storage_cmd::is_blob_cached,
            commands::storage_cmd::get_storage_quota,
            commands::storage_cmd::clear_blob_cache,
            commands::storage_cmd::get_blob_cache_info,
            commands::storage_cmd::delete_cached_blob,
            commands::storage_cmd::delete_cached_blobs_for_document,
            commands::storage_cmd::get_media_type_from_extension,
            commands::storage_cmd::get_mime_type,
            commands::workspace_cmd::create_workspace,
            commands::workspace_cmd::get_workspace,
            commands::workspace_cmd::list_workspaces,
            commands::workspace_cmd::update_workspace,
            commands::workspace_cmd::delete_workspace,
        ])
        .setup(|app| {
            app_handle::init(app.handle().clone());

            let main_window = app.get_webview_window("main").unwrap();
            main_window.create_overlay_titlebar().unwrap();

            #[cfg(target_os = "macos")]
            {
                main_window.make_transparent().unwrap();
            }

            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir).expect("Failed to create app_data_dir.");

            database::init_paths(app_data_dir)
                .map_err(|e| format!("Failed to initialize database paths: {e}"))?;

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
