//! # Live Block Locking Commands
//!
//! Tauri commands for real-time collaborative block editing.
//!
//! ## Lock States
//!
//! Blocks can be in three visual states for collaborators:
//! - **Locked (solid line)**: User is actively editing, others see current content
//! - **Focused (dotted line)**: User's cursor is in block but not editing
//! - **Unlocked (no line)**: Block is free for anyone to edit
//!
//! ## Lock Lifecycle
//!
//! 1. `request_lock`: User starts editing → solid line for others
//! 2. `update_live_block`: User types → others see content in real-time
//! 3. `release_lock_focused`: User stops typing but stays in block → dotted line
//! 4. `release_lock_blur`: User leaves block → no line
//!
//! ## Timeout
//!
//! Locks automatically expire after 60 seconds to prevent orphaned locks
//! from disconnected users.

use crate::batch_handler::block_types::Block;
use crate::encryption::live_block::DecryptedLiveBlock;
use crate::stdb;

/// Requests a lock on a block for editing.
///
/// If successful, the block is locked for this user and collaborators
/// will see a solid line indicator with the live content.
///
/// # Arguments
///
/// * `doc_id` - Document containing the block
/// * `block_id` - Block to lock
/// * `content` - Current block content to show collaborators
/// * `username` - Display name for the lock indicator
#[tauri::command]
pub async fn request_lock(
    doc_id: String,
    block_id: u64,
    content: Block,
    username: String,
) -> Result<(), String> {
    stdb::active_profile()
        .try_lock_block(doc_id, block_id, &content, username)
        .await
}

/// Releases lock but keeps focus indicator (dotted line).
///
/// Call when user stops typing but cursor remains in the block.
/// Collaborators will see a dotted line instead of solid.
#[tauri::command]
pub async fn release_lock_focused(doc_id: String, block_id: u64) -> Result<(), String> {
    stdb::active_profile()
        .unlock_block(doc_id, block_id, false)
        .await
}

/// Fully releases lock and removes all indicators.
///
/// Call when user leaves the block entirely (blur/unfocus).
/// Collaborators will no longer see any indicator.
#[tauri::command]
pub async fn release_lock_blur(doc_id: String, block_id: u64) -> Result<(), String> {
    stdb::active_profile()
        .unlock_block(doc_id, block_id, true)
        .await
}

/// Gets all active locks on a document.
///
/// Returns live blocks showing who is editing which blocks.
#[tauri::command]
pub async fn get_document_locks(doc_id: String) -> Result<Vec<DecryptedLiveBlock>, String> {
    let all_live_blocks: Vec<DecryptedLiveBlock> =
        stdb::active_profile().get_cached_live_blocks().await?;
    Ok(all_live_blocks
        .into_iter()
        .filter(|block| block.doc_id == doc_id)
        .collect())
}

/// Updates live block content for real-time collaboration.
///
/// This does NOT persist the content - it's purely for showing
/// collaborators what is being typed in real-time. Call this
/// on each keystroke while holding a lock.
#[tauri::command]
pub async fn update_live_block(
    doc_id: String,
    block_id: u64,
    content: Block,
) -> Result<(), String> {
    stdb::active_profile()
        .update_live_block(doc_id, block_id, &content)
        .await
}
