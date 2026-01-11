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
use crate::database;
use crate::utils::timestamp::timestamp;
use std::collections::HashSet;

/// Creates a new block in a document.
///
/// Returns the generated block ID.
#[tauri::command]
pub async fn create_block(doc_id: String, block: Block) -> Result<u64, String> {
    let block_id = block_setter::update_block(doc_id, None, block);
    Ok(block_id)
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
    timestamp: u64,
) -> Result<DocumentData, String> {
    // If no block IDs specified, get all blocks for this document
    let ids_to_fetch = if block_ids.is_empty() {
        get_all_block_ids(&doc_id).await?
    } else {
        block_ids
    };

    block_getter::get_blocks(doc_id, ids_to_fetch, timestamp as u128).await
}

/// Get all unique block IDs for a document from local database
async fn get_all_block_ids(doc_id: &str) -> Result<Vec<u64>, String> {
    let batches = database::get_batches_by_doc(doc_id.to_string()).await?;
    let block_ids: HashSet<u64> = batches.iter().map(|b| b.batch_data.block_id).collect();
    Ok(block_ids.into_iter().collect())
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
