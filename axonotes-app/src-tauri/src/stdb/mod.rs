use crate::config::StdbConfig;
use crate::stdb_bindings::*;
use once_cell::sync::OnceCell;
use spacetimedb_sdk::__codegen::log;
use spacetimedb_sdk::{DbContext, Error};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// ==========================================
// Module Organization
// ==========================================

mod callbacks;
pub(crate) mod context;
mod reducer_helper;

use crate::batch_handler::sync::setup_batch_sync;
use crate::database;
pub use context::ProfileStdbContext;
// ==========================================
// Global State - One Connection Per Profile
// ==========================================

type ConnectionMap = Arc<Mutex<HashMap<String, Arc<Mutex<DbConnection>>>>>;

static STDB_CONNECTIONS: OnceCell<ConnectionMap> = OnceCell::new();

fn get_connections() -> ConnectionMap {
    STDB_CONNECTIONS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

// ==========================================
// Public API - Profile-Scoped Operations
// ==========================================

/// Get a context for the active profile
/// The connection is created lazily when you call methods on the context
pub fn active_profile() -> ProfileStdbContext {
    ProfileStdbContext::new_active()
}

/// Get a context for a specific profile by ID
/// The connection is created lazily when you call methods on the context
pub fn profile(profile_id: &str) -> ProfileStdbContext {
    ProfileStdbContext::new(profile_id.to_string())
}

/// Disconnect a specific profile
pub async fn disconnect_profile(profile_id: &str) -> Result<(), String> {
    let connections = get_connections();
    let mut map = connections.lock().await;

    if let Some(conn) = map.remove(profile_id) {
        let conn = conn.lock().await;
        conn.disconnect().map_err(|e| e.to_string())?;
        log::info!("Disconnected SpacetimeDB for profile {}", profile_id);
    }

    Ok(())
}

/// Disconnect all profiles
pub async fn disconnect_all() -> Result<(), String> {
    let connections = get_connections();
    let mut map = connections.lock().await;

    for (profile_id, conn) in map.drain() {
        let conn = conn.lock().await;
        if let Err(e) = conn.disconnect() {
            log::error!("Failed to disconnect profile {}: {}", profile_id, e);
        }
    }

    log::info!("Disconnected all SpacetimeDB connections");
    Ok(())
}

/// Check if a profile has an active connection
pub async fn is_profile_connected(profile_id: &str) -> bool {
    let connections = get_connections();
    let map = connections.lock().await;
    map.contains_key(profile_id)
}

/// Get list of all connected profile IDs
pub async fn get_connected_profiles() -> Vec<String> {
    let connections = get_connections();
    let map = connections.lock().await;
    map.keys().cloned().collect()
}

// ==========================================
// Internal API - Connection Management
// ==========================================

/// Ensure a connection exists for the given profile
/// Creates a new connection if needed
pub(crate) async fn ensure_connection_for_profile(
    profile_id: &str,
    access_token: &str,
) -> Result<(), String> {
    let connections = get_connections();
    let mut map = connections.lock().await;

    // Return early if connection already exists
    if map.contains_key(profile_id) {
        return Ok(());
    }

    log::info!("Creating SpacetimeDB connection for profile {}", profile_id);

    // Build new connection
    let config = StdbConfig::new();
    let conn = build_connection(&config, access_token, profile_id).await?;

    // Register callbacks
    callbacks::register_callbacks(&conn);

    // Subscribe to user view (filtered by identity/JWT)
    // Subscribe to document metadata view
    // Subscribe to document keys view
    conn.subscription_builder()
        .on_applied(callbacks::on_subscription_applied)
        .on_error(callbacks::on_subscription_error)
        .subscribe([
            "SELECT * FROM user",
            "SELECT * FROM user_metadata",
            "SELECT * FROM user_document_keys",
            "SELECT * FROM accessible_live_blocks",
        ]);

    // Setup batch sync
    let start_time = database::get_last_active_profile_sync_time().await?;
    if let Some(start_time) = start_time {
        setup_batch_sync(&conn, start_time)?;
    }

    // Run in background thread
    conn.run_threaded();

    // Store connection
    map.insert(profile_id.to_string(), Arc::new(Mutex::new(conn)));

    log::info!(
        "✓ SpacetimeDB connection established for profile {}",
        profile_id
    );
    Ok(())
}

/// Get an existing connection for a profile
pub(crate) async fn get_connection_for_profile(
    profile_id: &str,
) -> Result<Arc<Mutex<DbConnection>>, String> {
    let connections = get_connections();
    let map = connections.lock().await;

    map.get(profile_id).cloned().ok_or_else(|| {
        format!(
            "No SpacetimeDB connection for profile {}. Call connect() first.",
            profile_id
        )
    })
}

/// Build a new SpacetimeDB connection with JWT authentication
async fn build_connection(
    config: &StdbConfig,
    access_token: &str,
    profile_id: &str,
) -> Result<DbConnection, String> {
    let profile_id_1 = profile_id.to_string();
    let profile_id_2 = profile_id.to_string();
    DbConnection::builder()
        .on_connect(move |_ctx, identity, _token| {
            log::info!(
                "Connected to SpacetimeDB - Profile: {}, Identity: {}",
                profile_id_1,
                identity.to_hex()
            );
        })
        .on_connect_error(|_ctx, err| {
            log::error!("SpacetimeDB connection error: {:?}", err);
        })
        .on_disconnect(move |_ctx, err| {
            if let Some(err) = err {
                log::warn!(
                    "SpacetimeDB disconnected for profile {}: {}",
                    profile_id_2,
                    err
                );
            } else {
                log::info!(
                    "SpacetimeDB disconnected cleanly for profile {}",
                    profile_id_2
                );
            }
        })
        .with_token(Some(access_token.to_string()))
        .with_module_name(config.default_module_name)
        .with_uri(config.default_host_uri)
        .build()
        .map_err(|e| format!("Failed to connect to SpacetimeDB: {}", e))
}
