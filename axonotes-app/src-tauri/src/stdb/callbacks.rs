use super::*;
use crate::events::{
    emit_block_lock_expired, emit_block_locked, emit_block_unlocked, emit_document_access_granted,
    emit_document_access_revoked,
};
use crate::stdb;
use crate::utils::timestamp::timestamp;
use once_cell::sync::OnceCell;
use spacetimedb_sdk::Table;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Lock timeout in milliseconds (60 seconds)
const LOCK_TIMEOUT_MS: u128 = 60_000;
/// How often to check for expired locks (10 seconds)
const EXPIRY_CHECK_INTERVAL_MS: u64 = 10_000;

/// Track which locks we've already emitted expiry events for
static EXPIRED_LOCKS: OnceCell<Arc<Mutex<HashSet<String>>>> = OnceCell::new();

fn get_expired_locks() -> Arc<Mutex<HashSet<String>>> {
    EXPIRED_LOCKS
        .get_or_init(|| Arc::new(Mutex::new(HashSet::new())))
        .clone()
}

pub fn register_callbacks(conn: &DbConnection) {
    // Register live block insert callback (block locked)
    conn.db
        .accessible_live_blocks()
        .on_insert(|_ctx, live_block| {
            // Only emit if the block is actually locked (has locked_at)
            if let Some(locked_at) = live_block.locked_at {
                emit_block_locked(
                    live_block.doc_id.clone(),
                    live_block.block_id,
                    live_block.user_id.to_hex().to_string(),
                    locked_at,
                );
            }

            // Remove from expired set if re-locked
            let lock_id = live_block.live_block_id.clone();
            tokio::spawn(async move {
                let expired_locks = get_expired_locks();
                let mut set = expired_locks.lock().await;
                set.remove(&lock_id);
            });
        });

    // Register live block delete callback (block unlocked)
    conn.db
        .accessible_live_blocks()
        .on_delete(|_ctx, live_block| {
            emit_block_unlocked(
                live_block.doc_id.clone(),
                live_block.block_id,
                live_block.user_id.to_hex().to_string(),
            );

            // Remove from expired set when deleted
            let lock_id = live_block.live_block_id.clone();
            tokio::spawn(async move {
                let expired_locks = get_expired_locks();
                let mut set = expired_locks.lock().await;
                set.remove(&lock_id);
            });
        });

    // Register user metadata insert callback (document access granted)
    conn.db.user_metadata().on_insert(|_ctx, metadata| {
        emit_document_access_granted(metadata.doc_id.clone());
    });

    // Register user metadata delete callback (document access revoked)
    conn.db.user_metadata().on_delete(|_ctx, metadata| {
        emit_document_access_revoked(metadata.doc_id.clone());
    });

    // Start the lock expiry checker background task
    start_lock_expiry_checker();
}

/// Start a background task that periodically checks for expired locks
fn start_lock_expiry_checker() {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(EXPIRY_CHECK_INTERVAL_MS)).await;

            if let Err(e) = check_expired_locks().await {
                eprintln!("Error checking expired locks: {e}");
            }
        }
    });
}

/// Check all cached live blocks for expired locks
async fn check_expired_locks() -> Result<(), String> {
    let live_blocks = match stdb::active_profile().get_cached_live_blocks().await {
        Ok(blocks) => blocks,
        Err(_) => return Ok(()), // Not connected yet, skip
    };

    let now = timestamp();
    let expired_locks_set = get_expired_locks();
    let mut expired_set = expired_locks_set.lock().await;

    for block in live_blocks {
        // Only check blocks that are actually locked
        if let Some(locked_at) = block.locked_at {
            let lock_age = now.saturating_sub(locked_at);

            if lock_age >= LOCK_TIMEOUT_MS {
                // Check if we've already emitted for this lock
                let lock_key = format!("{}_{}", block.doc_id, block.block_id);
                if !expired_set.contains(&lock_key) {
                    expired_set.insert(lock_key);
                    emit_block_lock_expired(
                        block.doc_id.clone(),
                        block.block_id,
                        block.user_id.to_hex().to_string(),
                    );
                }
            }
        }
    }

    Ok(())
}

// ==========================================
// Subscription Callbacks
// ==========================================

pub fn on_subscription_applied(_ctx: &SubscriptionEventContext) {
    log::info!("✓ Subscriptions applied");
}

#[allow(clippy::needless_pass_by_value)] // Signature constrained by SpacetimeDB callback API
pub fn on_subscription_error(_ctx: &ErrorContext, err: Error) {
    log::error!("✗ Subscription error: {err}");
}
