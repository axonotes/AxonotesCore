//! Share session events
//!
//! Events for document sharing workflow.

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_SHARE_CODE_READY: &str = "share-code-ready";
pub const EVENT_SHARE_USER_ADDED: &str = "share-user-added";
pub const EVENT_SHARE_ERROR: &str = "share-error";
pub const EVENT_SHARE_CLOSED: &str = "share-closed";

// ==========================================
// Payloads
// ==========================================

/// Emitted when a share code is ready after calling create_share
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareCodeReadyPayload {
    pub doc_id: String,
    pub share_code: String,
}

/// Emitted when a user has been successfully added to a document via share
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareUserAddedPayload {
    pub doc_id: String,
    pub user_id: String,
}

/// Emitted when an error occurs during share processing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareErrorPayload {
    pub doc_id: String,
    pub error: String,
}

/// Emitted when a share session is closed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareClosedPayload {
    pub doc_id: String,
    pub share_code: String,
}

// ==========================================
// Emitters
// ==========================================

pub fn emit_share_code_ready(doc_id: String, share_code: String) {
    let payload = ShareCodeReadyPayload { doc_id, share_code };
    if let Err(e) = app_handle::emit(EVENT_SHARE_CODE_READY, &payload) {
        eprintln!("Failed to emit {}: {}", EVENT_SHARE_CODE_READY, e);
    }
}

pub fn emit_share_user_added(doc_id: String, user_id: String) {
    let payload = ShareUserAddedPayload { doc_id, user_id };
    if let Err(e) = app_handle::emit(EVENT_SHARE_USER_ADDED, &payload) {
        eprintln!("Failed to emit {}: {}", EVENT_SHARE_USER_ADDED, e);
    }
}

pub fn emit_share_error(doc_id: String, error: String) {
    let payload = ShareErrorPayload { doc_id, error };
    if let Err(e) = app_handle::emit(EVENT_SHARE_ERROR, &payload) {
        eprintln!("Failed to emit {}: {}", EVENT_SHARE_ERROR, e);
    }
}

pub fn emit_share_closed(doc_id: String, share_code: String) {
    let payload = ShareClosedPayload { doc_id, share_code };
    if let Err(e) = app_handle::emit(EVENT_SHARE_CLOSED, &payload) {
        eprintln!("Failed to emit {}: {}", EVENT_SHARE_CLOSED, e);
    }
}
