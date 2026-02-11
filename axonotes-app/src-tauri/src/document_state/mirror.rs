//! # Frontend Mirror
//!
//! Tracks what the backend believes the frontend currently has.
//! Used to compute diffs so we only send changed blocks, never duplicates.
//!
//! ## How It Works
//!
//! Every time the backend emits a `BlockUpdate::Set`, the mirror is updated
//! to match. Every time a `BlockUpdate::Remove` is emitted, the block is
//! removed from the mirror. Before emitting, the backend checks the mirror
//! to determine if an update is actually needed.

use crate::batch_handler::block_types::Block;
use crate::document_state::types::{BlockUpdate, LockInfo};
use std::collections::HashMap;

// ==========================================
// Mirrored Block
// ==========================================

/// A block as the frontend currently sees it, including lock state.
#[derive(Debug, Clone)]
pub struct MirroredBlock {
    pub block: Block,
    pub lock: Option<LockInfo>,
}

// ==========================================
// Frontend Mirror
// ==========================================

/// Tracks the block state that the frontend currently holds.
///
/// The mirror is the single source of truth for what the frontend has.
/// By diffing the desired state against the mirror, we avoid sending
/// redundant events and ensure the frontend stays in sync.
#[derive(Debug)]
pub struct FrontendMirror {
    blocks: HashMap<u64, MirroredBlock>,
}

impl FrontendMirror {
    /// Create an empty mirror (no blocks known to the frontend yet).
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
        }
    }

    /// Get a reference to a mirrored block.
    pub fn get(&self, block_id: u64) -> Option<&MirroredBlock> {
        self.blocks.get(&block_id)
    }

    /// Check if a block is present in the mirror.
    pub fn contains(&self, block_id: u64) -> bool {
        self.blocks.contains_key(&block_id)
    }

    /// Insert or update a block in the mirror.
    pub fn set(&mut self, block_id: u64, block: Block, lock: Option<LockInfo>) {
        self.blocks.insert(block_id, MirroredBlock { block, lock });
    }

    /// Remove a block from the mirror.
    pub fn remove(&mut self, block_id: u64) {
        self.blocks.remove(&block_id);
    }

    /// Update only the lock state for a block (content unchanged).
    /// Returns `false` if the block doesn't exist in the mirror.
    pub fn update_lock(&mut self, block_id: u64, lock: Option<LockInfo>) -> bool {
        if let Some(mirrored) = self.blocks.get_mut(&block_id) {
            mirrored.lock = lock;
            true
        } else {
            false
        }
    }

    /// Get all block IDs currently in the mirror.
    pub fn block_ids(&self) -> Vec<u64> {
        self.blocks.keys().copied().collect()
    }

    /// Clear all blocks from the mirror.
    pub fn clear(&mut self) {
        self.blocks.clear();
    }

    /// Diff the current mirror against a desired set of blocks.
    ///
    /// Produces the minimal set of `BlockUpdate`s needed to transition the
    /// frontend from its current state (the mirror) to the desired state.
    ///
    /// - Blocks in `desired` but not in the mirror → `Set` (new block)
    /// - Blocks in both but with different content → `Set` (updated block)
    /// - Blocks in the mirror but not in `desired` → `Remove`
    /// - Blocks in both with identical content → skipped
    ///
    /// After calling this, the caller should apply the returned updates
    /// to the mirror via [`apply_updates`].
    pub fn diff(&self, desired: &[(u64, Block, Option<LockInfo>)]) -> Vec<BlockUpdate> {
        let mut updates = Vec::new();
        let desired_ids: std::collections::HashSet<u64> =
            desired.iter().map(|(id, _, _)| *id).collect();

        // Find new or changed blocks
        for (block_id, block, lock) in desired {
            let needs_update = match self.blocks.get(block_id) {
                None => true, // New block
                Some(mirrored) => {
                    // Check if content changed (compare JSON serialization for deep equality)
                    let block_changed = !blocks_equal(&mirrored.block, block);
                    let lock_changed = mirrored.lock != *lock;
                    block_changed || lock_changed
                }
            };

            if needs_update {
                updates.push(BlockUpdate::Set {
                    block_id: *block_id,
                    block: block.clone(),
                    lock: lock.clone(),
                });
            }
        }

        // Find removed blocks
        for block_id in self.blocks.keys() {
            if !desired_ids.contains(block_id) {
                updates.push(BlockUpdate::Remove {
                    block_id: *block_id,
                });
            }
        }

        updates
    }

    /// Apply a set of updates to the mirror, keeping it in sync with
    /// what was just emitted to the frontend.
    pub fn apply_updates(&mut self, updates: &[BlockUpdate]) {
        for update in updates {
            match update {
                BlockUpdate::Set {
                    block_id,
                    block,
                    lock,
                } => {
                    self.set(*block_id, block.clone(), lock.clone());
                }
                BlockUpdate::Remove { block_id } => {
                    self.remove(*block_id);
                }
            }
        }
    }
}

// ==========================================
// Helpers
// ==========================================

/// Compare two blocks for semantic equality using JSON serialization.
///
/// This is a pragmatic approach: blocks are small, and comparing the serialized
/// form catches all field differences without needing manual `PartialEq` impls
/// that would break if new fields are added.
fn blocks_equal(a: &Block, b: &Block) -> bool {
    // Fast path: compare serialized JSON bytes
    match (serde_json::to_vec(a), serde_json::to_vec(b)) {
        (Ok(a_bytes), Ok(b_bytes)) => a_bytes == b_bytes,
        _ => false,
    }
}
