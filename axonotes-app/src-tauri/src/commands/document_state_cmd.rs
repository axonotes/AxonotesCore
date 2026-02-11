//! # Document State Commands
//!
//! Tauri commands for the backend-managed document state API.
//!
//! ## Overview
//!
//! These commands replace the previous split of `block_cmd` + `live_lock_cmd`
//! with a unified API where the backend manages all document state. The frontend
//! sends user actions and receives block updates through a single event channel
//! (`block_update_{doc_id}`).
//!
//! ## Commands
//!
//! | Command | Purpose |
//! |---------|---------|
//! | `open_document` | Open a document, start receiving block updates |
//! | `close_document` | Close document, release locks, stop events |
//! | `set_time` | Time-travel to a historical timestamp or return to live |
//! | `ds_update_block` | Edit a block (backend handles lock + live + persist) |
//! | `ds_create_block` | Create a new block, returns real ID immediately |
//! | `ds_delete_block` | Soft-delete a block |
//!
//! ## Event Channel
//!
//! All block state changes are pushed through `block_update_{doc_id}` as
//! batched `Vec<BlockUpdate>` payloads. The frontend listens to this single
//! event and upserts/removes blocks accordingly.
//!
//! ## Lock Management
//!
//! Locks are fully automatic — no explicit lock/unlock commands needed:
//! - Acquired on first `ds_update_block` call
//! - Released when editing a different block, after 60s inactivity, or on close
//!
//! ## History Mode
//!
//! When `set_time` is called with a past timestamp, the document becomes
//! read-only. Editing commands return errors. Call `set_time` with `u64::MAX`
//! to return to live mode.

use crate::batch_handler::block_types::Block;
use crate::document_state::handle;

/// Opens a document for editing.
///
/// Reconstructs the current block state, sets up event forwarding for live
/// updates and sync completions, and pushes the initial state to the frontend
/// via the `block_update_{doc_id}` event channel.
///
/// If the document is already open, the previous session is closed first.
///
/// # Arguments
///
/// * `doc_id` - The document to open
///
/// # Events
///
/// Emits `block_update_{doc_id}` with all initial blocks as `Set` updates.
///
/// # Errors
///
/// - Document doesn't exist or no batches found
/// - No identity or encryption keys available
#[tauri::command]
pub async fn open_document(doc_id: String) -> Result<(), String> {
    handle::open_document(doc_id).await
}

/// Closes an open document.
///
/// Releases any locks held, flushes pending persists, tears down event
/// forwarding, and clears the frontend mirror. No-op if the document
/// is not currently open.
///
/// # Arguments
///
/// * `doc_id` - The document to close
#[tauri::command]
pub async fn close_document(doc_id: String) -> Result<(), String> {
    handle::close_document(doc_id).await
}

/// Sets the document time for time-travel.
///
/// Reconstructs the document at the given timestamp, diffs against what
/// the frontend currently has, and emits only the changes through the
/// event channel.
///
/// # Arguments
///
/// * `doc_id` - The document to time-travel
/// * `timestamp` - Target timestamp in milliseconds since epoch.
///   Use `u64::MAX` (or the JS equivalent `Number.MAX_SAFE_INTEGER`)
///   to return to live mode.
///
/// # Mode Switching
///
/// - **History mode** (`timestamp < now`): Document becomes read-only.
///   Live updates and sync events are ignored. Lock info is cleared.
/// - **Live mode** (`timestamp == u64::MAX`): Editing is re-enabled.
///   Live updates and sync events are forwarded again.
///
/// # Events
///
/// Emits `block_update_{doc_id}` with `Set` for new/changed blocks
/// and `Remove` for blocks that don't exist at the target timestamp.
///
/// # Errors
///
/// - Document not open
/// - Block reconstruction failure
#[tauri::command]
pub async fn set_time(doc_id: String, timestamp: u64) -> Result<(), String> {
    handle::set_time(doc_id, timestamp).await
}

/// Updates a block's content.
///
/// The backend handles everything automatically:
/// 1. **Lock acquisition** — acquired on first edit, auto-switched on block change
/// 2. **Persistence** — throttled and batched via the existing batch system
/// 3. **Live broadcast** — throttled broadcast to collaborators via SpacetimeDB
/// 4. **Inactivity timeout** — lock auto-released after 60s of silence
///
/// No echo event is emitted back to the calling frontend (the frontend already
/// has the content it just sent). Other collaborators receive the update via
/// the `live-block-updated` → `block_update_{doc_id}` forwarding chain.
///
/// # Arguments
///
/// * `doc_id` - Document containing the block
/// * `block_id` - Block to update
/// * `content` - New block content
///
/// # Errors
///
/// - Document not open
/// - Document in history mode (read-only)
/// - Block locked by another user
#[tauri::command]
pub async fn ds_update_block(
    doc_id: String,
    block_id: u64,
    content: Block,
) -> Result<(), String> {
    handle::update_block(doc_id, block_id, content).await
}

/// Creates a new block in the document.
///
/// A cryptographically random block ID is generated by the backend and
/// returned immediately. Unlike the previous frontend flow, there are no
/// temp IDs — the real ID is available synchronously.
///
/// # Arguments
///
/// * `doc_id` - Document to add the block to
/// * `block` - Block content (the `id` field will be overwritten)
///
/// # Returns
///
/// The generated block ID.
///
/// # Errors
///
/// - Document not open
/// - Document in history mode (read-only)
#[tauri::command]
pub async fn ds_create_block(doc_id: String, block: Block) -> Result<u64, String> {
    handle::create_block(doc_id, block).await
}

/// Soft-deletes a block by marking it as deleted.
///
/// The block remains in history and can be viewed via time-travel.
/// If we hold a lock on this block, it is released.
///
/// # Arguments
///
/// * `doc_id` - Document containing the block
/// * `block_id` - Block to delete
///
/// # Events
///
/// Emits `block_update_{doc_id}` with a `Remove` update.
///
/// # Errors
///
/// - Document not open
/// - Document in history mode (read-only)
/// - Block doesn't exist
#[tauri::command]
pub async fn ds_delete_block(doc_id: String, block_id: u64) -> Result<(), String> {
    handle::delete_block(doc_id, block_id).await
}
