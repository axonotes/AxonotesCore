use crate::batch_handler::block_getter::{BlockData, DocumentData};
use crate::batch_handler::block_types::Block;
use crate::batch_handler::{block_getter, block_setter};
use crate::utils::timestamp::timestamp;

#[tauri::command]
pub async fn create_block(doc_id: String, block: Block) -> Result<(), String> {
    block_setter::update_block(doc_id, None, block);
    Ok(())
}

#[tauri::command]
pub async fn get_blocks(
    doc_id: String,
    block_ids: Vec<u64>,
    timestamp: u128,
) -> Result<DocumentData, String> {
    block_getter::get_blocks(doc_id, block_ids, timestamp).await
}

#[tauri::command]
pub async fn update_block(doc_id: String, block_id: u64, block: Block) -> Result<(), String> {
    block_setter::update_block(doc_id, Some(block_id), block);
    Ok(())
}

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
