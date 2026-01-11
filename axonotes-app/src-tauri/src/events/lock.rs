//! # Live Lock Events
//!
//! Events for real-time block locking during collaborative editing.
//!
//! ## Lock Lifecycle
//!
//! ```text
//! User starts editing → block-locked
//!                    ↓
//! User stops editing → block-unlocked
//!                    or
//! 60 seconds timeout → block-lock-expired
//! ```
//!
//! ## Event Types
//!
//! | Event | Trigger | Payload |
//! |-------|---------|---------|
//! | `block-locked` | User acquired lock | doc_id, block_id, user_id, locked_at |
//! | `block-unlocked` | User released lock | doc_id, block_id, user_id |
//! | `block-lock-expired` | Lock timed out | doc_id, block_id, user_id |
//!
//! ## Frontend Handling
//!
//! - Show lock indicator on block (solid line = locked, dotted = focused)
//! - Display editor's name/avatar
//! - Disable editing for locked blocks (other users)

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_BLOCK_LOCKED: &str = "block-locked";
pub const EVENT_BLOCK_UNLOCKED: &str = "block-unlocked";
pub const EVENT_BLOCK_LOCK_EXPIRED: &str = "block-lock-expired";
pub const EVENT_LIVE_BLOCK_UPDATED: &str = "live-block-updated";
pub const EVENT_LIVE_BLOCK_RELEASED: &str = "live-block-released";

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

/// Emitted when a live block is updated (content change during editing)
/// This is the main event for real-time collaboration - sent with decrypted content
/// NOTE: Uses snake_case to match DecryptedLiveBlock type used by frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveBlockUpdatedPayload {
    pub live_block_id: String,
    pub doc_id: String,
    pub block_id: u64,
    pub user_id: String,
    pub username: String,
    pub locked_at: Option<u128>,
    pub content: crate::batch_handler::block_types::Block,
}

/// Emitted when a live block is released (user stopped editing)
/// NOTE: Uses snake_case to match frontend expectations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveBlockReleasedPayload {
    pub doc_id: String,
    pub block_id: u64,
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

/// Emits a live-block-updated event with decrypted content for real-time collaboration
pub fn emit_live_block_updated(
    live_block_id: String,
    doc_id: String,
    block_id: u64,
    user_id: String,
    username: String,
    locked_at: Option<u128>,
    content: crate::batch_handler::block_types::Block,
) {
    let payload = LiveBlockUpdatedPayload {
        live_block_id,
        doc_id,
        block_id,
        user_id,
        username,
        locked_at,
        content,
    };
    if let Err(e) = app_handle::emit(EVENT_LIVE_BLOCK_UPDATED, &payload) {
        eprintln!("Failed to emit {EVENT_LIVE_BLOCK_UPDATED}: {e}");
    }
}

/// Emits a live-block-released event when a user stops editing a block
pub fn emit_live_block_released(doc_id: String, block_id: u64) {
    let payload = LiveBlockReleasedPayload { doc_id, block_id };
    if let Err(e) = app_handle::emit(EVENT_LIVE_BLOCK_RELEASED, &payload) {
        eprintln!("Failed to emit {EVENT_LIVE_BLOCK_RELEASED}: {e}");
    }
}
