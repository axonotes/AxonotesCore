//! # Block Setter - Throttled Batch Creation
//!
//! Handles block updates with intelligent batching and throttling for efficient
//! network synchronization.
//!
//! ## Design Goals
//!
//! - **Throttling**: Coalesce rapid edits into fewer network requests (500ms window)
//! - **Batching**: Group multiple patches into a single batch (up to 20 patches)
//! - **Delta Compression**: Store only changes between block states
//! - **Automatic Finalization**: Background task finalizes stale batches
//!
//! ## Flow
//!
//! 1. `update_block()` queues the block in `PENDING_BLOCKS`
//! 2. After throttle delay, block is processed into patches
//! 3. Patches accumulate in `IN_PROGRESS_BATCHES`
//! 4. Batches finalize when full, timed out, or time gap exceeds threshold
//! 5. Finalized batches are sent to `sync::create_batch()` for upload
//!
//! ## Constants
//!
//! - `THROTTLE_MS`: 500ms delay before processing pending blocks
//! - `MAX_PATCHES_PER_BATCH`: 20 patches maximum per batch
//! - `MAX_TIME_DELTA_MS`: 1275ms max gap between patches (255 * 5ms units)
//! - `FINALIZE_TIMEOUT_MS`: 1300ms before forcing batch finalization

use crate::batch_handler::block_getter::get_blocks;
use crate::batch_handler::block_type_helpers::{encode_initial_patch, encode_patch};
use crate::batch_handler::block_types::Block;
use crate::batch_handler::sync::create_batch;
use crate::encryption::batch::{BatchData, DecryptedBatch, Patch};
use crate::utils::timestamp::timestamp;
use dashmap::DashMap;
use rand::TryRngCore;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;
use tokio::time::Duration;

/// Throttle delay before processing pending blocks (milliseconds).
const THROTTLE_MS: u64 = 500;

/// Maximum number of patches allowed in a single batch.
const MAX_PATCHES_PER_BATCH: usize = 20;

/// Maximum time gap between patches in a batch (milliseconds).
/// Patches with larger gaps trigger batch finalization.
const MAX_TIME_DELTA_MS: u128 = 255 * 5; // 1275ms max between patches

/// Timeout after which an in-progress batch is automatically finalized.
const FINALIZE_TIMEOUT_MS: u128 = 1300;

/// Cache key type: (document_id, block_id).
type CacheKey = (String, u64);

/// Pending blocks waiting to be processed (latest update wins).
/// Blocks are held here during the throttle window.
static PENDING_BLOCKS: LazyLock<DashMap<CacheKey, Block>> = LazyLock::new(DashMap::new);

/// In-progress batches being built up before finalization.
/// Each key maps to a batch accumulating patches.
static IN_PROGRESS_BATCHES: LazyLock<DashMap<CacheKey, InProgressBatch>> =
    LazyLock::new(DashMap::new);

/// Tracks which keys have an active throttle timer to prevent duplicate timers.
static ACTIVE_TIMERS: LazyLock<DashMap<CacheKey, ()>> = LazyLock::new(DashMap::new);

/// Ensures the background finalization task starts only once per process.
static BACKGROUND_TASK_STARTED: AtomicBool = AtomicBool::new(false);

/// A batch being built up before finalization.
///
/// Accumulates patches as the user edits, then converts to `DecryptedBatch`
/// when finalized for upload.
struct InProgressBatch {
    doc_id: String,
    block_id: u64,
    base_timestamp: u128,
    last_patch_timestamp: u128,
    patches: Vec<Patch>,
    is_initial: bool,
    last_block: Block, // State after last patch (for delta calculation)
}

impl InProgressBatch {
    fn new(
        doc_id: String,
        block_id: u64,
        timestamp: u128,
        is_initial: bool,
        first_patch: Patch,
        block: Block,
    ) -> Self {
        Self {
            doc_id,
            block_id,
            base_timestamp: timestamp,
            last_patch_timestamp: timestamp,
            patches: vec![first_patch],
            is_initial,
            last_block: block,
        }
    }

    fn can_accept_patch(&self, now: u128) -> bool {
        if self.patches.len() >= MAX_PATCHES_PER_BATCH {
            return false;
        }
        let time_since_last = now.saturating_sub(self.last_patch_timestamp);
        time_since_last <= MAX_TIME_DELTA_MS
    }

    fn into_decrypted_batch(self) -> DecryptedBatch {
        DecryptedBatch {
            batch_id: uuid::Uuid::new_v4().to_string(),
            doc_id: self.doc_id,
            timestamp: self.base_timestamp,
            is_initial: self.is_initial,
            batch_data: BatchData {
                block_id: self.block_id,
                patches: self.patches,
            },
        }
    }
}

// ==========================================
// Public API
// ==========================================

/// Generate a cryptographically random block ID
fn generate_block_id() -> u64 {
    let mut rng = rand::rngs::OsRng;
    rng.try_next_u64().unwrap()
}

/// Update a block. Changes are throttled to 500ms and batched automatically.
/// If block_id is None, a cryptographically random ID is generated.
/// Returns the block_id used (useful when auto-generated).
/// the id and timestamp in the block will always be overwritten with the correct values
pub fn update_block(doc_id: String, block_id: Option<u64>, block: Block) -> u64 {
    let block_id = block_id.unwrap_or_else(generate_block_id);
    let block = Block {
        id: block_id,
        timestamp: timestamp(),
        ..block
    };
    let key = (doc_id, block_id);

    // Store latest pending block (overwrites previous)
    PENDING_BLOCKS.insert(key.clone(), block);

    // Ensure background finalization task is running
    ensure_background_task();

    // Start throttle timer if not already running for this key
    if ACTIVE_TIMERS.insert(key.clone(), ()).is_none() {
        tokio::spawn(run_throttle_timer(key));
    }

    block_id
}

// ==========================================
// Throttle Timer
// ==========================================

fn ensure_background_task() {
    if !BACKGROUND_TASK_STARTED.swap(true, Ordering::SeqCst) {
        tokio::spawn(run_finalization_task());
    }
}

async fn run_throttle_timer(key: CacheKey) {
    loop {
        tokio::time::sleep(Duration::from_millis(THROTTLE_MS)).await;

        // Take pending block (if any)
        if let Some((_, block)) = PENDING_BLOCKS.remove(&key) {
            if let Err(e) = process_pending_block(&key, block).await {
                eprintln!("Failed to process pending block: {e}");
            }
        }

        // If new pending appeared while we were removing, restart
        if PENDING_BLOCKS.contains_key(&key) {
            continue;
        }

        // Check if in-progress batch needs finalization due to timeout
        let should_finalize = IN_PROGRESS_BATCHES
            .get(&key)
            .is_some_and(|b| timestamp() - b.last_patch_timestamp > FINALIZE_TIMEOUT_MS);

        if should_finalize {
            if let Err(e) = finalize_batch(&key).await {
                eprintln!("Failed to finalize batch: {e}");
            }
        }

        // If batch still exists (not timed out yet), keep timer alive
        if IN_PROGRESS_BATCHES.contains_key(&key) {
            continue;
        }

        // Remove from active timers
        ACTIVE_TIMERS.remove(&key);

        break;
    }
}

// ==========================================
// Patch Processing
// ==========================================

async fn process_pending_block(key: &CacheKey, block: Block) -> Result<(), String> {
    let (doc_id, block_id) = key;
    let now = timestamp();

    // Check if current batch needs finalization (can't accept more patches)
    let should_finalize = IN_PROGRESS_BATCHES
        .get(key)
        .is_some_and(|b| !b.can_accept_patch(now));

    if should_finalize {
        finalize_batch(key).await?;
    }

    // Try to add to existing batch
    if let Some(mut batch) = IN_PROGRESS_BATCHES.get_mut(key) {
        #[allow(clippy::cast_possible_truncation)]
        // Time delta fits in u8 due to MAX_TIME_DELTA_MS constraint
        let time_delta = ((now - batch.last_patch_timestamp) / 5) as u8;
        let patch = encode_patch(0, time_delta, &batch.last_block, &block)?;

        batch.patches.push(patch);
        batch.last_patch_timestamp = now;
        batch.last_block = block;

        return Ok(());
    }

    // No existing batch - create new one
    // Get last committed state from database
    let last_state = get_blocks(doc_id.clone(), vec![*block_id], u128::MAX)
        .await?
        .blocks
        .into_iter()
        .next();

    let is_initial = last_state.is_none();
    let time_delta = 0u8; // First patch in batch starts at base timestamp

    let patch = if let Some(ref last_block_data) = last_state {
        encode_patch(0, time_delta, &last_block_data.block, &block)?
    } else {
        encode_initial_patch(0, time_delta, &block)?
    };

    let new_batch = InProgressBatch::new(doc_id.clone(), *block_id, now, is_initial, patch, block);

    IN_PROGRESS_BATCHES.insert(key.clone(), new_batch);

    Ok(())
}

// ==========================================
// Batch Finalization
// ==========================================

async fn finalize_batch(key: &CacheKey) -> Result<(), String> {
    if let Some((_, batch)) = IN_PROGRESS_BATCHES.remove(key) {
        if !batch.patches.is_empty() {
            let decrypted_batch = batch.into_decrypted_batch();
            create_batch(decrypted_batch).await?;
        }
    }
    Ok(())
}

/// Background task that finalizes stale batches (where timer has exited)
async fn run_finalization_task() {
    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;

        let now = timestamp();
        let mut keys_to_finalize = Vec::new();

        for entry in IN_PROGRESS_BATCHES.iter() {
            let key = entry.key();
            let batch = entry.value();

            // Only finalize if timed out AND no active timer for this key
            let timed_out = now - batch.last_patch_timestamp > FINALIZE_TIMEOUT_MS;
            let has_active_timer = ACTIVE_TIMERS.contains_key(key);

            if timed_out && !has_active_timer {
                keys_to_finalize.push(key.clone());
            }
        }

        for key in keys_to_finalize {
            if let Err(e) = finalize_batch(&key).await {
                eprintln!("Failed to finalize stale batch: {e}");
            }
        }
    }
}

// ==========================================
// Testing Utilities
// ==========================================

#[cfg(test)]
#[allow(dead_code)]
pub async fn flush_all_batches() {
    let keys: Vec<CacheKey> = IN_PROGRESS_BATCHES
        .iter()
        .map(|e| e.key().clone())
        .collect();

    for key in keys {
        let _ = finalize_batch(&key).await;
    }
}

#[cfg(test)]
#[allow(dead_code)]
pub fn get_pending_count() -> usize {
    PENDING_BLOCKS.len()
}

#[cfg(test)]
#[allow(dead_code)]
pub fn get_in_progress_count() -> usize {
    IN_PROGRESS_BATCHES.len()
}
