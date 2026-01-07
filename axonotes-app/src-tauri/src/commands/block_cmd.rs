//! # Block Management Commands
//!
//! Tauri commands for creating, reading, updating, and deleting document blocks.
//!
//! ## Block CRUD Operations
//!
//! - `create_block`: Creates a new block with auto-generated ID
//! - `get_blocks`: Retrieves blocks at a specific point in time
//! - `update_block`: Updates an existing block's content
//! - `delete_block`: Soft-deletes a block (marks as deleted)
//!
//! ## Soft Delete
//!
//! Blocks are never physically deleted. Instead, they are marked with
//! `deleted: true`, preserving history for version navigation and undo.
//!
//! ## Batching
//!
//! All mutations go through the block setter which handles:
//! - Throttling rapid edits
//! - Batching multiple changes
//! - Delta compression
//! - Server synchronization

use crate::batch_handler::block_getter::{BlockData, DocumentData};
use crate::batch_handler::block_types::Block;
use crate::batch_handler::{block_getter, block_setter};
use crate::utils::timestamp::timestamp;

/// Creates a new block in a document.
///
/// A cryptographically random block ID is automatically generated.
#[tauri::command]
pub async fn create_block(doc_id: String, block: Block) -> Result<(), String> {
    block_setter::update_block(doc_id, None, block);
    Ok(())
}

/// Retrieves blocks at a specific point in document history.
///
/// # Arguments
///
/// * `doc_id` - Document to query
/// * `block_ids` - Specific blocks to retrieve (empty for all)
/// * `timestamp` - Point in time to reconstruct blocks at
#[tauri::command]
pub async fn get_blocks(
    doc_id: String,
    block_ids: Vec<u64>,
    timestamp: u128,
) -> Result<DocumentData, String> {
    block_getter::get_blocks(doc_id, block_ids, timestamp).await
}

/// Updates an existing block's content.
///
/// Changes are throttled and batched automatically.
#[tauri::command]
pub async fn update_block(doc_id: String, block_id: u64, block: Block) -> Result<(), String> {
    block_setter::update_block(doc_id, Some(block_id), block);
    Ok(())
}

/// Soft-deletes a block by marking it as deleted.
///
/// The block remains in history and can be restored via version tags.
#[tauri::command]
pub async fn delete_block(doc_id: String, block_id: u64) -> Result<(), String> {
    let mut doc: DocumentData =
        block_getter::get_blocks(doc_id.clone(), vec![block_id], timestamp()).await?;
    if doc.blocks.is_empty() {
        return Err("Can't delete block that doesn't exist.".to_string());
    }
    let block: BlockData = doc.blocks.remove(0);
    block_setter::update_block(
        doc_id,
        Some(block_id),
        Block {
            deleted: Some(true),
            ..block.block
        },
    );
    Ok(())
}
