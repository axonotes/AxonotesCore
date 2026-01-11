//! # SpacetimeDB Integration Module
//!
//! Manages connections to SpacetimeDB for real-time synchronization.
//!
//! ## Architecture
//!
//! Each user profile has its own SpacetimeDB connection, allowing multiple
//! accounts to be logged in simultaneously. Connections are created lazily
//! when first accessed and maintained until explicitly disconnected.
//!
//! ## Submodules
//!
//! - **`callbacks`**: SpacetimeDB subscription and reducer callbacks
//! - **`context`**: Profile-scoped context for database operations
//! - **`reducer_helper`**: Helper macros for calling reducers with await
//!
//! ## Subscriptions
//!
//! The client subscribes to filtered views:
//! - `user`: Current user's account data
//! - `user_metadata`: Document metadata visible to user
//! - `user_document_keys`: Encrypted document keys for user
//! - `accessible_live_blocks`: Real-time collaborative edits
//! - `manageable_permissions`: Permissions for owned/editable documents
//! - `public_user_keys`: Public keys of collaborators
//! - `accessible_version_tags`: Version tags for accessible documents
//!
//! ## Usage
//!
//! ```ignore
//! // Get context for active profile
//! let ctx = stdb::active_profile();
//! ctx.create_document(...).await?;
//!
//! // Get context for specific profile
//! let ctx = stdb::profile("profile_123");
//! ctx.disconnect().await?;
//! ```

#![allow(dead_code)]

use crate::config::StdbConfig;
use crate::events::{emit_stdb_connected, emit_stdb_connection_error, emit_stdb_disconnected};
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
use crate::share::sync::setup_share_sync;
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
        log::info!("Disconnected SpacetimeDB for profile {profile_id}");
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
            log::error!("Failed to disconnect profile {profile_id}: {e}");
        }
    }

    log::info!("Disconnected all SpacetimeDB connections");
    Ok(())
}

/// Reconnect the active profile with a fresh token from the database.
/// This is used after token refresh to apply the new token to the `SpacetimeDB` connection.
pub async fn reconnect_active_profile() -> Result<(), String> {
    // Get the active profile with the fresh token
    let profile = database::get_active_profile()
        .await?
        .ok_or("No active profile")?;

    // Check if there's an existing connection to reconnect
    let was_connected = is_profile_connected(&profile.id).await;

    if was_connected {
        // Disconnect the old connection
        disconnect_profile(&profile.id).await?;

        // Create a new connection with the fresh token
        ensure_connection_for_profile(&profile.id, &profile.access_token).await?;
    }
    // If not connected, no need to reconnect

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
///
/// Initially only subscribes to the `user` table. Call `apply_document_subscriptions`
/// after user keys are available to subscribe to document-related tables.
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

    log::info!("Creating SpacetimeDB connection for profile {profile_id}");

    // Reset subscription ready flag before setting up new connection
    callbacks::reset_subscription_ready();

    // Build new connection
    let config = StdbConfig::new();
    let conn = build_connection(&config, access_token, profile_id).await?;

    // Register callbacks
    callbacks::register_callbacks(&conn);

    // Check if user keys already exist locally (returning user on this device)
    let keys_exist = database::get_active_user_keys()
        .await
        .map(|k| k.is_some())
        .unwrap_or(false);

    if keys_exist {
        // User has keys - subscribe to everything at once
        log::info!("User keys found, subscribing to all tables");
        conn.subscription_builder()
            .on_applied(callbacks::on_subscription_applied)
            .on_error(callbacks::on_subscription_error)
            .subscribe([
                "SELECT * FROM user",
                "SELECT * FROM user_metadata",
                "SELECT * FROM user_document_keys",
                "SELECT * FROM accessible_live_blocks",
                "SELECT * FROM manageable_permissions",
                "SELECT * FROM public_user_keys",
                "SELECT * FROM accessible_version_tags",
            ]);

        // Setup batch sync
        let start_time = database::get_last_active_profile_sync_time().await?;
        if let Some(start_time) = start_time {
            setup_batch_sync(&conn, start_time)?;
        }

        // Setup share sync (for auto-accepting joiners)
        setup_share_sync(&conn)?;
    } else {
        // New device - only subscribe to user table initially
        // Document subscriptions will be applied after key sync via apply_document_subscriptions()
        log::info!("No user keys found, subscribing to user table only (phase 1)");
        conn.subscription_builder()
            .on_applied(callbacks::on_initial_subscription_applied)
            .on_error(callbacks::on_subscription_error)
            .subscribe(["SELECT * FROM user"]);
    }

    // Run in background thread
    conn.run_threaded();

    // Store connection (before waiting, so other code can access it)
    map.insert(profile_id.to_string(), Arc::new(Mutex::new(conn)));

    // Release the map lock before blocking wait
    drop(map);

    // Wait for initial subscriptions to be applied
    // This ensures the cache is populated before we return
    log::debug!("Waiting for initial subscriptions...");
    callbacks::wait_for_subscriptions()?;

    log::info!("✓ SpacetimeDB connection established for profile {profile_id}");
    Ok(())
}

/// Apply document-related subscriptions after user keys are synced.
/// Call this after `sync_stdb_keys_with_pwd` or `sync_stdb_keys_with_mnemonic`.
pub async fn apply_document_subscriptions(profile_id: &str) -> Result<(), String> {
    let conn_arc = get_connection_for_profile(profile_id).await?;
    let conn = conn_arc.lock().await;

    log::info!("Applying document subscriptions (phase 2) for profile {profile_id}");

    // Reset subscription ready flag before applying new subscriptions
    callbacks::reset_subscription_ready();

    // Subscribe to document-related tables
    conn.subscription_builder()
        .on_applied(callbacks::on_subscription_applied)
        .on_error(callbacks::on_subscription_error)
        .subscribe([
            "SELECT * FROM user_metadata",
            "SELECT * FROM user_document_keys",
            "SELECT * FROM accessible_live_blocks",
            "SELECT * FROM manageable_permissions",
            "SELECT * FROM public_user_keys",
            "SELECT * FROM accessible_version_tags",
        ]);

    // Setup batch sync
    let start_time = database::get_last_active_profile_sync_time().await?;
    if let Some(start_time) = start_time {
        setup_batch_sync(&conn, start_time)?;
    }

    // Setup share sync (for auto-accepting joiners)
    setup_share_sync(&conn)?;

    // Release lock before waiting
    drop(conn);

    // Wait for document subscriptions to be applied
    callbacks::wait_for_subscriptions()?;

    log::info!("✓ Document subscriptions applied for profile {profile_id}");
    Ok(())
}

/// Get an existing connection for a profile
pub(crate) async fn get_connection_for_profile(
    profile_id: &str,
) -> Result<Arc<Mutex<DbConnection>>, String> {
    let connections = get_connections();
    let map = connections.lock().await;

    map.get(profile_id).cloned().ok_or_else(|| {
        format!("No SpacetimeDB connection for profile {profile_id}. Call connect() first.")
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
    let profile_id_3 = profile_id.to_string();
    DbConnection::builder()
        .on_connect(move |_ctx, identity, _token| {
            log::info!(
                "Connected to SpacetimeDB - Profile: {}, Identity: {}",
                profile_id_1,
                identity.to_hex()
            );
            emit_stdb_connected(profile_id_1.clone(), identity.to_hex().to_string());
        })
        .on_connect_error(move |_ctx, err| {
            log::error!("SpacetimeDB connection error: {err:?}");
            emit_stdb_connection_error(profile_id_3.clone(), format!("{err:?}"));
        })
        .on_disconnect(move |_ctx, err| {
            if let Some(ref err) = err {
                log::warn!("SpacetimeDB disconnected for profile {profile_id_2}: {err}");
            } else {
                log::info!("SpacetimeDB disconnected cleanly for profile {profile_id_2}");
            }
            emit_stdb_disconnected(profile_id_2.clone(), err.map(|e| e.to_string()));
        })
        .with_token(Some(access_token.to_string()))
        .with_module_name(config.default_module_name)
        .with_uri(config.default_host_uri)
        .build()
        .map_err(|e| format!("Failed to connect to SpacetimeDB: {e}"))
}
