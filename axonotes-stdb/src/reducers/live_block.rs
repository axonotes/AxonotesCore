use crate::tables::{private_document_permission, private_live_block, LiveBlock};
use spacetimedb::{ReducerContext, Table, Timestamp};

/// Try to acquire a lock on a block for editing
/// Returns error if block is locked by someone else
#[spacetimedb::reducer]
pub fn try_lock_block(
    ctx: &ReducerContext,
    doc_id: String,
    block_id: u64,
    encrypted_content: Vec<u8>,
    encrypted_username: Vec<u8>,
) -> Result<(), String> {
    // Check access
    let has_access = ctx
        .db
        .private_document_permission()
        .by_doc()
        .filter(&doc_id)
        .any(|p| p.user_id == ctx.sender);

    if !has_access {
        return Err("No access to this document".to_string());
    }

    // Find existing live block for this (doc_id, block_id)
    let existing = ctx
        .db
        .private_live_block()
        .by_doc_and_block()
        .filter(&doc_id)
        .find(|lb| lb.block_id == block_id);

    let now = ctx
        .timestamp
        .duration_since(Timestamp::UNIX_EPOCH)
        .expect("timestamp should exist")
        .as_millis();

    if let Some(live_block) = existing {
        // Check if locked by someone else
        if let Some(locked_time) = live_block.locked_at {
            // Check timeout (60 seconds)
            if now - locked_time < 60_000 && live_block.user_id != ctx.sender {
                return Err("Block locked by another user".to_string());
            }
        }

        // Take over or refresh lock
        ctx.db
            .private_live_block()
            .live_block_id()
            .update(LiveBlock {
                user_id: ctx.sender,
                encrypted_content,
                encrypted_username,
                locked_at: Some(now),
                ..live_block
            });
    } else {
        // Create new lock
        ctx.db.private_live_block().insert(LiveBlock {
            live_block_id: uuid::Uuid::new_v4().to_string(),
            doc_id,
            block_id,
            user_id: ctx.sender,
            encrypted_content,
            encrypted_username,
            locked_at: Some(now),
        });
    }

    Ok(())
}

/// Update live block content (while maintaining lock)
#[spacetimedb::reducer]
pub fn update_live_block(
    ctx: &ReducerContext,
    doc_id: String,
    block_id: u64,
    encrypted_content: Vec<u8>,
) -> Result<(), String> {
    // Find the live block
    let live_block = ctx
        .db
        .private_live_block()
        .by_doc_and_block()
        .filter(&doc_id)
        .find(|lb| lb.block_id == block_id)
        .ok_or("Live block not found")?;

    // Must be the owner of the lock
    if live_block.user_id != ctx.sender {
        return Err("Not your lock".to_string());
    }

    // Update content and refresh timestamp
    let now = ctx
        .timestamp
        .duration_since(Timestamp::UNIX_EPOCH)
        .expect("timestamp should exist")
        .as_millis();
    ctx.db
        .private_live_block()
        .live_block_id()
        .update(LiveBlock {
            encrypted_content,
            locked_at: Some(now),
            ..live_block
        });

    Ok(())
}

/// Unlock a block (change to focused state or delete)
#[spacetimedb::reducer]
pub fn unlock_block(
    ctx: &ReducerContext,
    doc_id: String,
    block_id: u64,
    delete: bool, // true = delete row, false = set locked_at to None
) -> Result<(), String> {
    // Find the live block
    let live_block = ctx
        .db
        .private_live_block()
        .by_doc_and_block()
        .filter(&doc_id)
        .find(|lb| lb.block_id == block_id)
        .ok_or("Live block not found")?;

    // Must be the owner
    if live_block.user_id != ctx.sender {
        return Err("Not your lock".to_string());
    }

    if delete {
        // Delete live block
        ctx.db
            .private_live_block()
            .live_block_id()
            .delete(&live_block.live_block_id);
    } else {
        // Set to focused (locked_at = None)
        ctx.db
            .private_live_block()
            .live_block_id()
            .update(LiveBlock {
                locked_at: None,
                ..live_block
            });
    }

    Ok(())
}
