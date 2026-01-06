//! Sync status events
//!
//! Events for batch synchronization status.

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_SYNC_STARTED: &str = "sync-started";
pub const EVENT_SYNC_PROGRESS: &str = "sync-progress";
pub const EVENT_SYNC_COMPLETED: &str = "sync-completed";
pub const EVENT_SYNC_ERROR: &str = "sync-error";

// ==========================================
// Payloads
// ==========================================

/// Emitted when sync starts for a document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStartedPayload {
    pub doc_id: String,
    pub batch_count: u32,
}

/// Emitted for sync progress updates
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncProgressPayload {
    pub doc_id: String,
    pub synced_count: u32,
    pub total_count: u32,
}

/// Emitted when sync completes for a document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncCompletedPayload {
    pub doc_id: String,
    pub batch_count: u32,
}

/// Emitted when sync fails
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncErrorPayload {
    pub doc_id: String,
    pub error: String,
}

// ==========================================
// Emitters
// ==========================================

pub fn emit_sync_started(doc_id: String, batch_count: u32) {
    let payload = SyncStartedPayload {
        doc_id,
        batch_count,
    };
    if let Err(e) = app_handle::emit(EVENT_SYNC_STARTED, &payload) {
        eprintln!("Failed to emit {EVENT_SYNC_STARTED}: {e}");
    }
}

pub fn emit_sync_progress(doc_id: String, synced_count: u32, total_count: u32) {
    let payload = SyncProgressPayload {
        doc_id,
        synced_count,
        total_count,
    };
    if let Err(e) = app_handle::emit(EVENT_SYNC_PROGRESS, &payload) {
        eprintln!("Failed to emit {EVENT_SYNC_PROGRESS}: {e}");
    }
}

pub fn emit_sync_completed(doc_id: String, batch_count: u32) {
    let payload = SyncCompletedPayload {
        doc_id,
        batch_count,
    };
    if let Err(e) = app_handle::emit(EVENT_SYNC_COMPLETED, &payload) {
        eprintln!("Failed to emit {EVENT_SYNC_COMPLETED}: {e}");
    }
}

pub fn emit_sync_error(doc_id: String, error: String) {
    let payload = SyncErrorPayload { doc_id, error };
    if let Err(e) = app_handle::emit(EVENT_SYNC_ERROR, &payload) {
        eprintln!("Failed to emit {EVENT_SYNC_ERROR}: {e}");
    }
}
