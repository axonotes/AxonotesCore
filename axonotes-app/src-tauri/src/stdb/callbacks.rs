//! # SpacetimeDB Callbacks
//!
//! Handles real-time database change events from SpacetimeDB subscriptions.
//!
//! ## Callback Types
//!
//! | Table | Event | Action |
//! |-------|-------|--------|
//! | `accessible_live_blocks` | insert | Emit `block-locked` event |
//! | `accessible_live_blocks` | delete | Emit `block-unlocked` event |
//! | `user_metadata` | insert | Emit `document-access-granted` |
//! | `user_metadata` | delete | Emit `document-access-revoked` |
//!
//! ## Lock Expiry
//!
//! A background task runs every 10 seconds to check for expired locks.
//! Locks expire after 60 seconds of inactivity. The `EXPIRED_LOCKS` set
//! prevents duplicate expiry events for the same lock.
//!
//! ## Architecture
//!
//! Callbacks are registered once during connection setup and fire
//! automatically when SpacetimeDB pushes row changes via WebSocket.

use super::*;
use crate::encryption::document::DecryptDocumentMetaAndKeyVec;
use crate::events::{
    emit_block_lock_expired, emit_block_locked, emit_block_unlocked, emit_document_access_granted,
    emit_document_access_revoked,
};
use crate::stdb;
use crate::utils::timestamp::timestamp;
use once_cell::sync::OnceCell;
use spacetimedb_sdk::Table;
use std::collections::HashSet;
use std::sync::{Arc, Condvar, Mutex};

/// Lock timeout in milliseconds (60 seconds)
const LOCK_TIMEOUT_MS: u128 = 60_000;
/// How often to check for expired locks (10 seconds)
const EXPIRY_CHECK_INTERVAL_MS: u64 = 10_000;
/// Timeout for waiting for initial subscriptions (10 seconds)
const SUBSCRIPTION_TIMEOUT_MS: u64 = 10_000;

/// Track which locks we've already emitted expiry events for.
///
/// This prevents emitting multiple `block-lock-expired` events for the same
/// lock before the server cleans it up. Entries are removed when the lock
/// is released or re-acquired.
static EXPIRED_LOCKS: OnceCell<Arc<Mutex<HashSet<String>>>> = OnceCell::new();

fn get_expired_locks() -> Arc<Mutex<HashSet<String>>> {
    EXPIRED_LOCKS
        .get_or_init(|| Arc::new(Mutex::new(HashSet::new())))
        .clone()
}

// ==========================================
// Subscription Readiness Tracking
// ==========================================

/// Tracks whether initial subscriptions have been applied.
/// Uses a Condvar to allow waiting for subscriptions without busy-polling.
static SUBSCRIPTION_READY: OnceCell<Arc<(Mutex<bool>, Condvar)>> = OnceCell::new();

fn get_subscription_ready() -> Arc<(Mutex<bool>, Condvar)> {
    SUBSCRIPTION_READY
        .get_or_init(|| Arc::new((Mutex::new(false), Condvar::new())))
        .clone()
}

/// Resets the subscription ready flag (call before setting up new connection)
pub fn reset_subscription_ready() {
    let ready = get_subscription_ready();
    if let Ok(mut is_ready) = ready.0.lock() {
        *is_ready = false;
    };
}

/// Waits for initial subscriptions to be applied (with timeout).
/// Returns Ok(()) if subscriptions were applied, Err if timeout.
pub fn wait_for_subscriptions() -> Result<(), String> {
    let ready = get_subscription_ready();
    let (lock, cvar) = &*ready;

    let guard = lock
        .lock()
        .map_err(|e| format!("Failed to acquire subscription lock: {e}"))?;

    // Wait with timeout
    let timeout = std::time::Duration::from_millis(SUBSCRIPTION_TIMEOUT_MS);
    let result = cvar
        .wait_timeout_while(guard, timeout, |is_ready| !*is_ready)
        .map_err(|e| format!("Failed to wait for subscriptions: {e}"))?;

    if result.1.timed_out() {
        Err("Timeout waiting for initial subscriptions".to_string())
    } else {
        Ok(())
    }
}

/// Signals that subscriptions are ready (called from on_subscription_applied callback)
fn signal_subscription_ready() {
    let ready = get_subscription_ready();
    if let Ok(mut is_ready) = ready.0.lock() {
        if !*is_ready {
            *is_ready = true;
            ready.1.notify_all();
        }
    };
}

/// Registers all SpacetimeDB table change callbacks.
///
/// Called once during connection setup. Callbacks are triggered
/// automatically by the SpacetimeDB SDK when subscribed data changes.
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

            // Remove from expired set if re-locked (sync, runs on STDB thread)
            let lock_id = live_block.live_block_id.clone();
            let expired_locks = get_expired_locks();
            if let Ok(mut set) = expired_locks.lock() {
                set.remove(&lock_id);
            };
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

            // Remove from expired set when deleted (sync, runs on STDB thread)
            let lock_id = live_block.live_block_id.clone();
            let expired_locks = get_expired_locks();
            if let Ok(mut set) = expired_locks.lock() {
                set.remove(&lock_id);
            };
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
    let mut expired_set = expired_locks_set
        .lock()
        .map_err(|e| format!("Failed to lock expired set: {e}"))?;

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

/// Sync all STDB cached data to local SQLite for the current user identity.
/// Called only on subscription applied.
fn sync_stdb_to_local_db(ctx: &SubscriptionEventContext) {
    // Get the current user's identity
    let Some(identity) = ctx.try_identity() else {
        eprintln!("[sync] No identity available for sync");
        return;
    };

    // Pull all data from STDB's in-memory cache
    let keys: Vec<_> = ctx.db.user_document_keys().iter().collect();
    let permissions: Vec<_> = ctx.db.manageable_permissions().iter().collect();
    let documents: Vec<_> = ctx.db.accessible_documents().iter().collect();

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("[sync] Failed to create tokio runtime: {e}");
                return;
            }
        };

        rt.block_on(async {
            // Get user's private key for decrypting document keys
            let user_keys = match crate::database::get_active_user_keys().await {
                Ok(Some(keys)) => keys,
                Ok(None) => {
                    eprintln!("[sync] No user keys available for decryption");
                    return;
                }
                Err(e) => {
                    eprintln!("[sync] Failed to get user keys: {e}");
                    return;
                }
            };

            // Decrypt document keys before storing
            let private_key = match user_keys.private_encryption_key.as_slice().try_into() {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("[sync] Failed to convert private key: {e:?}");
                    return;
                }
            };
            let private_key: &[u8; 32] = private_key;

            let decrypted_keys = match keys.decrypt_all(private_key) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("[sync] Failed to decrypt document keys: {e}");
                    return;
                }
            };

            // Overwrite local SQLite for THIS identity only
            if let Err(e) = crate::database::sync_document_keys(identity, decrypted_keys).await {
                eprintln!("[sync] Failed to sync document keys: {e}");
            } else {
                log::debug!("[sync] Document keys synced");
            }

            if let Err(e) = crate::database::sync_permissions(identity, permissions).await {
                eprintln!("[sync] Failed to sync permissions: {e}");
            } else {
                log::debug!("[sync] Permissions synced");
            }

            if let Err(e) = crate::database::sync_documents(identity, documents).await {
                eprintln!("[sync] Failed to sync documents: {e}");
            } else {
                log::debug!("[sync] Documents synced");
            }

            log::info!("[sync] ✓ STDB data synced to local SQLite");
        });
    });
}

pub fn on_subscription_applied(ctx: &SubscriptionEventContext) {
    log::info!("✓ Subscriptions applied");

    // Sync all STDB data to local SQLite
    sync_stdb_to_local_db(ctx);

    // Signal that initial subscriptions are ready (first callback wins)
    signal_subscription_ready();
}

#[allow(clippy::needless_pass_by_value)] // Signature constrained by SpacetimeDB callback API
pub fn on_subscription_error(_ctx: &ErrorContext, err: Error) {
    log::error!("✗ Subscription error: {err}");
}
