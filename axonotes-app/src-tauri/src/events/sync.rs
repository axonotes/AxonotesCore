//! # Sync Status Events
//!
//! Events for batch synchronization status.
//!
//! ## Sync Flow
//!
//! ```text
//! Batches pending → sync-started (batch_count)
//!       ↓
//! Each batch sent → sync-progress (synced/total)
//!       ↓
//! All complete → sync-completed
//!       or
//! Failure → sync-error
//! ```
//!
//! ## Event Types
//!
//! | Event | Trigger | Payload |
//! |-------|---------|---------|
//! | `sync-started` | Sync begins | doc_id, batch_count |
//! | `sync-progress` | Batch synced | doc_id, synced_count, total_count |
//! | `sync-completed` | All batches synced | doc_id, batch_count |
//! | `sync-error` | Sync failed | doc_id, error |
//! | `batch-conflicts` | Conflict detected | conflict_id, doc_id, block_id, user_state, server_state |
//!
//! ## Frontend Handling
//!
//! - Show sync indicator (spinner/progress)
//! - Display "Saved" on completion
//! - Show retry option on error

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_SYNC_STARTED: &str = "sync-started";
pub const EVENT_SYNC_PROGRESS: &str = "sync-progress";
pub const EVENT_SYNC_COMPLETED: &str = "sync-completed";
pub const EVENT_SYNC_ERROR: &str = "sync-error";
pub const EVENT_BATCH_CONFLICTS: &str = "batch-conflicts";

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

/// Emitted when a batch conflict is detected during sync.
/// Contains both user and server states for UI resolution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchConflictPayload {
    /// Unique conflict identifier
    pub conflict_id: String,
    /// Document where conflict occurred
    pub doc_id: String,
    /// Block where conflict occurred
    pub block_id: u64,
    /// User's block state (JSON serialized Block)
    pub user_state: String,
    /// Server's block state (JSON serialized Block)
    pub server_state: String,
    /// When the conflict was detected (milliseconds since epoch)
    pub timestamp: u128,
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

pub fn emit_batch_conflict(payload: &BatchConflictPayload) {
    if let Err(e) = app_handle::emit(EVENT_BATCH_CONFLICTS, payload) {
        eprintln!("Failed to emit {EVENT_BATCH_CONFLICTS}: {e}");
    }
}
