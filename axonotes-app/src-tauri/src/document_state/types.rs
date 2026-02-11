//! # Document State Types
//!
//! All types used by the document state management system, including
//! the event payload types sent to the frontend via `block_update_{doc_id}`.
//!
//! ## Event Payload
//!
//! The frontend receives `Vec<BlockUpdate>` on the `block_update_{doc_id}` event.
//! Each `BlockUpdate` is either a `Set` (upsert) or `Remove` (delete).

use crate::batch_handler::block_types::Block;
use serde::{Deserialize, Serialize};

// ==========================================
// Block Update Event Types
// ==========================================

/// A single block update sent to the frontend via the event channel.
///
/// Events are batched: a single emission contains `Vec<BlockUpdate>`.
///
/// ## Variants
///
/// - **`Set`**: A block was added or its content/lock state changed.
///   The frontend should upsert this block in its local state.
/// - **`Remove`**: A block was removed (deleted or absent at current timestamp).
///   The frontend should remove this block from its local state.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum BlockUpdate {
    /// A block was added or its content/lock state changed.
    #[serde(rename_all = "camelCase")]
    Set {
        block_id: u64,
        block: Block,
        lock: Option<LockInfo>,
    },

    /// A block was removed (deleted or no longer exists at current timestamp).
    #[serde(rename_all = "camelCase")]
    Remove { block_id: u64 },
}

// ==========================================
// Lock Types
// ==========================================

/// Lock information for a block, describing who holds the lock and its state.
///
/// Sent as part of `BlockUpdate::Set` to inform the frontend about
/// collaborative editing state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LockInfo {
    /// Identity hex string of the user holding the lock
    pub user_id: String,
    /// Display name of the user
    pub username: String,
    /// Whether the lock is active (editing) or just focused
    pub state: LockState,
}

/// Whether a lock represents active editing or just cursor presence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum LockState {
    /// Solid line — user is actively editing
    Locked,
    /// Dotted line — user's cursor is in block but not typing
    Focused,
}

// ==========================================
// Document Mode
// ==========================================

/// The mode a document is currently in.
///
/// Determines whether live updates are forwarded and whether edits are allowed.
#[derive(Debug, Clone)]
pub enum DocumentMode {
    /// Live mode: forwarding events, editing allowed.
    Live,
    /// History mode: frozen snapshot, read-only.
    /// The `u128` is the historical timestamp being viewed.
    History { timestamp: u128 },
}
