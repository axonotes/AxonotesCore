//! # Sync Conflict Commands
//!
//! Tauri commands for managing sync conflicts.
//!
//! ## Conflict Resolution
//!
//! When a batch conflict occurs:
//! 1. Frontend receives `batch-conflicts` event with user/server states
//! 2. User chooses resolution (accept server, keep theirs, or merge)
//! 3. Frontend calls `resolve_conflict` to mark it resolved
//! 4. If user chose their version or merge, frontend also sends new batches
//!
//! ## Commands
//!
//! | Command | Purpose |
//! |---------|---------|
//! | `get_pending_conflicts` | Get all unresolved conflicts |
//! | `get_pending_conflicts_for_doc` | Get conflicts for specific document |
//! | `resolve_conflict` | Mark a conflict as resolved |
//! | `check_pending_conflicts_on_startup` | Re-emit events for any unresolved conflicts |

use crate::database::{self, PendingSyncConflict};
use crate::events::{emit_batch_conflict, BatchConflictPayload};

/// Get all pending conflicts across all documents.
///
/// Returns conflicts sorted by timestamp descending (most recent first).
#[tauri::command]
pub async fn get_pending_conflicts() -> Result<Vec<PendingSyncConflict>, String> {
    database::get_all_pending_conflicts().await
}

/// Get pending conflicts for a specific document.
///
/// # Arguments
/// * `doc_id` - The document ID to get conflicts for
#[tauri::command]
pub async fn get_pending_conflicts_for_doc(
    doc_id: String,
) -> Result<Vec<PendingSyncConflict>, String> {
    database::get_pending_conflicts_by_doc(doc_id).await
}

/// Resolve (delete) a pending conflict.
///
/// Called after the user has chosen how to handle the conflict:
/// - Accept server: Just delete the conflict (server state already applied)
/// - Keep user's: Delete conflict, then UI sends batches to recreate user's state
/// - Merge: Delete conflict, then UI sends batches with merged state
///
/// # Arguments
/// * `conflict_id` - The conflict ID to resolve
#[tauri::command]
pub async fn resolve_conflict(conflict_id: String) -> Result<(), String> {
    database::delete_pending_conflict(conflict_id).await
}

/// Get a specific pending conflict by ID.
///
/// # Arguments
/// * `conflict_id` - The conflict ID to get
#[tauri::command]
pub async fn get_pending_conflict(
    conflict_id: String,
) -> Result<Option<PendingSyncConflict>, String> {
    database::get_pending_conflict(conflict_id).await
}

/// Check for any pending conflicts and re-emit events.
///
/// Should be called by the frontend after login/database unlock.
/// This ensures the user is notified of any conflicts that occurred
/// before the app was closed or if the app crashed during resolution.
///
/// # Returns
/// The number of pending conflicts that were re-emitted
#[tauri::command]
pub async fn check_pending_conflicts_on_startup() -> Result<u32, String> {
    let conflicts = database::get_all_pending_conflicts().await?;
    let count = conflicts.len() as u32;

    for conflict in conflicts {
        let payload = BatchConflictPayload {
            conflict_id: conflict.conflict_id,
            doc_id: conflict.doc_id,
            block_id: conflict.block_id,
            user_state: conflict.user_state,
            server_state: conflict.server_state,
            timestamp: conflict.timestamp,
        };
        emit_batch_conflict(&payload);
    }

    Ok(count)
}
