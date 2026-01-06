//! # Connection Status Events
//!
//! Events for SpacetimeDB connection lifecycle.
//!
//! ## Connection Flow
//!
//! ```text
//! Login → Connecting → stdb-connected
//!                   → stdb-connection-error (on failure)
//!
//! Connected → stdb-disconnected (unexpected)
//!          → stdb-disconnected (user logout)
//! ```
//!
//! ## Event Types
//!
//! | Event | Trigger | Payload |
//! |-------|---------|---------|
//! | `stdb-connected` | Connection established | profile_id, identity |
//! | `stdb-disconnected` | Connection lost/closed | profile_id, error? |
//! | `stdb-connection-error` | Connection failed | profile_id, error |
//!
//! ## Frontend Handling
//!
//! - Show connection status indicator
//! - Enable/disable sync features
//! - Display reconnection UI on disconnect

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_STDB_CONNECTED: &str = "stdb-connected";
pub const EVENT_STDB_DISCONNECTED: &str = "stdb-disconnected";
pub const EVENT_STDB_CONNECTION_ERROR: &str = "stdb-connection-error";

// ==========================================
// Payloads
// ==========================================

/// Emitted when SpacetimeDB connection is established
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StdbConnectedPayload {
    pub profile_id: String,
    pub identity: String,
}

/// Emitted when SpacetimeDB connection is lost
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StdbDisconnectedPayload {
    pub profile_id: String,
    /// Error message if disconnection was unexpected
    pub error: Option<String>,
}

/// Emitted when SpacetimeDB connection fails
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StdbConnectionErrorPayload {
    pub profile_id: String,
    pub error: String,
}

// ==========================================
// Emitters
// ==========================================

pub fn emit_stdb_connected(profile_id: String, identity: String) {
    let payload = StdbConnectedPayload {
        profile_id,
        identity,
    };
    if let Err(e) = app_handle::emit(EVENT_STDB_CONNECTED, &payload) {
        eprintln!("Failed to emit {EVENT_STDB_CONNECTED}: {e}");
    }
}

pub fn emit_stdb_disconnected(profile_id: String, error: Option<String>) {
    let payload = StdbDisconnectedPayload { profile_id, error };
    if let Err(e) = app_handle::emit(EVENT_STDB_DISCONNECTED, &payload) {
        eprintln!("Failed to emit {EVENT_STDB_DISCONNECTED}: {e}");
    }
}

pub fn emit_stdb_connection_error(profile_id: String, error: String) {
    let payload = StdbConnectionErrorPayload { profile_id, error };
    if let Err(e) = app_handle::emit(EVENT_STDB_CONNECTION_ERROR, &payload) {
        eprintln!("Failed to emit {EVENT_STDB_CONNECTION_ERROR}: {e}");
    }
}
