use crate::batch_handler::block_types::Block;
use crate::encryption::live_block::DecryptedLiveBlock;
use crate::stdb;

/// Call this function to try to lock a block. If successful the block is now locked for this user
/// and the frontend can show the complete line
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

/// CHANGE TO DOTTED LINE
/// Call this when the user just removes the lock but still is focused on the block
/// so others see the dotted line.
#[tauri::command]
pub async fn release_lock_focused(doc_id: String, block_id: u64) -> Result<(), String> {
    stdb::active_profile()
        .unlock_block(doc_id, block_id, false)
        .await
}

/// REMOVE LINE
/// Call this when the user removes the lock AND unfocuses/blurs/goes out of the block
/// so others DON'T see any line anymore
#[tauri::command]
pub async fn release_lock_blur(doc_id: String, block_id: u64) -> Result<(), String> {
    stdb::active_profile()
        .unlock_block(doc_id, block_id, true)
        .await
}

/// Get all locks on a document
#[tauri::command]
pub async fn get_document_locks(doc_id: String) -> Result<Vec<DecryptedLiveBlock>, String> {
    let all_live_blocks: Vec<DecryptedLiveBlock> =
        stdb::active_profile().get_cached_live_blocks().await?;
    Ok(all_live_blocks
        .into_iter()
        .filter(|block| block.doc_id == doc_id)
        .collect())
}

/// Update the content for a live block. This does not create a batch or store the content permanently
/// this is just for visuals so others can see the content in realtime.
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
