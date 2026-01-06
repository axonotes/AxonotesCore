//! Live lock events
//!
//! Events for real-time block locking during collaborative editing.

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_BLOCK_LOCKED: &str = "block-locked";
pub const EVENT_BLOCK_UNLOCKED: &str = "block-unlocked";
pub const EVENT_BLOCK_LOCK_EXPIRED: &str = "block-lock-expired";

// ==========================================
// Payloads
// ==========================================

/// Emitted when a block is locked by a user
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockLockedPayload {
    pub doc_id: String,
    pub block_id: u64,
    pub user_id: String,
    pub locked_at: u128,
}

/// Emitted when a block is unlocked
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockUnlockedPayload {
    pub doc_id: String,
    pub block_id: u64,
    pub user_id: String,
}

/// Emitted when a block lock expires (timeout)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockLockExpiredPayload {
    pub doc_id: String,
    pub block_id: u64,
    pub user_id: String,
}

// ==========================================
// Emitters
// ==========================================

pub fn emit_block_locked(doc_id: String, block_id: u64, user_id: String, locked_at: u128) {
    let payload = BlockLockedPayload {
        doc_id,
        block_id,
        user_id,
        locked_at,
    };
    if let Err(e) = app_handle::emit(EVENT_BLOCK_LOCKED, &payload) {
        eprintln!("Failed to emit {EVENT_BLOCK_LOCKED}: {e}");
    }
}

pub fn emit_block_unlocked(doc_id: String, block_id: u64, user_id: String) {
    let payload = BlockUnlockedPayload {
        doc_id,
        block_id,
        user_id,
    };
    if let Err(e) = app_handle::emit(EVENT_BLOCK_UNLOCKED, &payload) {
        eprintln!("Failed to emit {EVENT_BLOCK_UNLOCKED}: {e}");
    }
}

pub fn emit_block_lock_expired(doc_id: String, block_id: u64, user_id: String) {
    let payload = BlockLockExpiredPayload {
        doc_id,
        block_id,
        user_id,
    };
    if let Err(e) = app_handle::emit(EVENT_BLOCK_LOCK_EXPIRED, &payload) {
        eprintln!("Failed to emit {EVENT_BLOCK_LOCK_EXPIRED}: {e}");
    }
}
