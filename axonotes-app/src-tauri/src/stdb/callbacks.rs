//! # SpacetimeDB Callbacks
//!
//! Handles real-time database change events from SpacetimeDB subscriptions.
//!
//! ## Important: Subscription Callback Types
//!
//! SpacetimeDB uses different callbacks for different purposes:
//! - `on_applied`: Fires **once** when subscription is first applied (initial data load)
//! - `on_insert`: Fires for **each new row** inserted (real-time updates)
//! - `on_delete`: Fires for **each row deleted** (real-time updates)
//!
//! Note: We subscribe to **views**, not tables. Views model updates as delete + insert,
//! so `on_insert` and `on_delete` cover all changes (no `on_update` needed).
//!
//! For real-time sync, you MUST use `on_insert`/`on_delete` callbacks, not just `on_applied`.
//!
//! ## Registered Callbacks
//!
//! | Table | Event | Action |
//! |-------|-------|--------|
//! | `accessible_live_blocks` | insert | Emit `block-locked` event |
//! | `accessible_live_blocks` | delete | Emit `block-unlocked` event |
//! | `user_metadata` | insert | Decrypt, save to local DB, emit `document-access-granted` |
//! | `user_metadata` | delete | Delete from local DB, emit `document-access-revoked` |
//! | `user_document_keys` | insert | Decrypt and save key to local DB |
//! | `user_document_keys` | delete | Delete key from local DB |
//! | `manageable_permissions` | insert | Save permission to local DB |
//! | `manageable_permissions` | delete | Delete permission from local DB |
//! | `accessible_documents` | insert | Save document to local DB |
//! | `accessible_documents` | delete | Delete document from local DB |
//! | `accessible_version_tags` | insert | Decrypt, save to local DB, emit event |
//! | `accessible_version_tags` | delete | Delete from local DB, emit event |
//! | `accessible_batches` | insert | Sync batch to local DB (in batch_handler/sync.rs) |
//! | `my_pending_shares` | insert | Emit share code ready (in share/sync.rs) |
//! | `pending_share_requests` | insert | Auto-accept joiner (in share/sync.rs) |
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
use crate::encryption::version_tag::DecryptVersionTagVec;
use crate::events::{
    emit_block_lock_expired, emit_block_locked, emit_block_unlocked, emit_document_access_granted,
    emit_document_metadata_updated, emit_document_path_changed_by_sync, emit_version_tag_created,
    emit_version_tag_deleted,
};
use crate::stdb;
use crate::stdb_bindings::{DocumentKey, DocumentPermission, DocumentVersionTag};
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
            log::debug!(
                "[callback:realtime] accessible_live_blocks on_insert - doc: {}, block: {}, user: {}",
                live_block.doc_id,
                live_block.block_id,
                live_block.user_id.to_hex()
            );
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
            log::debug!(
                "[callback:realtime] accessible_live_blocks on_delete - doc: {}, block: {}, user: {}",
                live_block.doc_id,
                live_block.block_id,
                live_block.user_id.to_hex()
            );
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

    // Register user metadata insert callback (sync to local DB + emit appropriate event)
    // NOTE: In SpacetimeDB views, updates are DELETE + INSERT. We check if the doc
    // already exists locally to determine if this is an update or a new grant.
    conn.db.user_metadata().on_insert(|ctx, metadata| {
        let metadata = metadata.clone();
        let identity = ctx.identity();
        let doc_id = metadata.doc_id.clone();

        // Decrypt and save to local DB
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                // Check if this doc already exists locally (update vs new grant)
                let is_new = crate::database::get_metadata_for_doc(identity, doc_id.clone())
                    .await
                    .map(|m| m.is_none())
                    .unwrap_or(true);

                if let Err(e) = sync_metadata_insert(identity, metadata).await {
                    eprintln!("[callback] Failed to sync metadata: {e}");
                    return;
                }

                // Only emit access-granted for truly new documents
                if is_new {
                    emit_document_access_granted(doc_id);
                }
                // For updates, sync_metadata_insert already emits metadata-updated
            });
        });
    });

    // Register user metadata delete callback
    // NOTE: In SpacetimeDB views, updates are DELETE + INSERT. We DON'T delete from
    // local DB or emit access-revoked here, because an INSERT might follow (for updates).
    // True access revocation is handled when document keys are revoked.
    conn.db.user_metadata().on_delete(|_ctx, metadata| {
        log::debug!(
            "[callback] user_metadata on_delete for doc {} (no action - waiting for potential insert)",
            metadata.doc_id
        );
        // Don't delete or emit here - let the document key revocation handle true access removal
    });

    // Register document key insert callback (new document key shared with user)
    conn.db.user_document_keys().on_insert(|ctx, key| {
        let key: DocumentKey = key.clone();
        let identity = ctx.identity();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                if let Err(e) = sync_document_key(identity, key).await {
                    eprintln!("[callback] Failed to sync document key: {e}");
                }
            });
        });
    });

    // Register document key delete callback (key revoked, e.g., user removed from document)
    conn.db.user_document_keys().on_delete(|ctx, key| {
        let key_id = key.key_id.clone();
        let identity = ctx.identity();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                if let Err(e) = crate::database::delete_document_key(identity, key_id).await {
                    eprintln!("[callback] Failed to delete document key: {e}");
                }
            });
        });
    });

    // Register permission insert callback (permission granted/changed)
    conn.db
        .manageable_permissions()
        .on_insert(|ctx, permission| {
            let permission: DocumentPermission = permission.clone();
            let identity = ctx.identity();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                    if let Err(e) = crate::database::save_permission(identity, permission).await {
                        eprintln!("[callback] Failed to save permission: {e}");
                    }
                });
            });
        });

    // Register permission delete callback (permission revoked)
    conn.db
        .manageable_permissions()
        .on_delete(|ctx, permission| {
            let permission_id = permission.permission_id.clone();
            let identity = ctx.identity();
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                    if let Err(e) =
                        crate::database::delete_permission(identity, permission_id).await
                    {
                        eprintln!("[callback] Failed to delete permission: {e}");
                    }
                });
            });
        });

    // Register document insert callback (new document accessible)
    conn.db.accessible_documents().on_insert(|ctx, document| {
        let document = document.clone();
        let identity = ctx.identity();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                if let Err(e) = crate::database::save_document(identity, document).await {
                    eprintln!("[callback] Failed to save document: {e}");
                }
            });
        });
    });

    // Register document delete callback (document access removed)
    conn.db.accessible_documents().on_delete(|ctx, document| {
        let doc_id = document.doc_id.clone();
        let identity = ctx.identity();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                if let Err(e) = crate::database::delete_document(identity, doc_id).await {
                    eprintln!("[callback] Failed to delete document: {e}");
                }
            });
        });
    });

    // Register version tag insert callback (new version tag created)
    conn.db.accessible_version_tags().on_insert(|ctx, tag| {
        let tag: DocumentVersionTag = tag.clone();
        let identity = ctx.identity();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                if let Err(e) = sync_version_tag_created(identity, tag).await {
                    eprintln!("[callback] Failed to sync version tag: {e}");
                }
            });
        });
    });

    // Register version tag delete callback (version tag deleted)
    conn.db.accessible_version_tags().on_delete(|ctx, tag| {
        let tag_id = tag.tag_id.clone();
        let doc_id = tag.doc_id.clone();
        let identity = ctx.identity();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                // Delete from local DB
                if let Err(e) = crate::database::delete_version_tag(identity, tag_id.clone()).await
                {
                    eprintln!("[callback] Failed to delete version tag: {e}");
                }
                // Emit event for frontend
                emit_version_tag_deleted(tag_id, doc_id);
            });
        });
    });

    // Start the lock expiry checker background task
    start_lock_expiry_checker();
}

/// Decrypts and saves a single metadata entry to local database.
async fn sync_metadata_insert(
    identity: spacetimedb_sdk::Identity,
    metadata: crate::stdb_bindings::DocumentMetadata,
) -> Result<(), String> {
    let doc_id = metadata.doc_id.clone();

    // Get user's private key for decryption
    let user_keys = crate::database::get_active_user_keys()
        .await?
        .ok_or("No user keys available for decryption")?;

    let private_key: &[u8; 32] = user_keys
        .private_encryption_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid private key length")?;

    // Decrypt the metadata
    let decrypted = metadata.decrypt(private_key)?;

    // Save to local database
    crate::database::save_metadata(identity, decrypted.clone()).await?;

    // Emit event so frontend can update UI (e.g., file rename)
    emit_document_metadata_updated(
        doc_id.clone(),
        decrypted.metadata.path,
        decrypted.metadata.tags,
    );

    log::debug!("[callback] Metadata synced for doc {doc_id}");
    Ok(())
}

/// Decrypts and saves a single document key to local database.
async fn sync_document_key(
    identity: spacetimedb_sdk::Identity,
    key: DocumentKey,
) -> Result<(), String> {
    // Get user's private key for decryption
    let user_keys = crate::database::get_active_user_keys()
        .await?
        .ok_or("No user keys available for decryption")?;

    let private_key: &[u8; 32] = user_keys
        .private_encryption_key
        .as_slice()
        .try_into()
        .map_err(|_| "Invalid private key length")?;

    // Decrypt the document key
    let decrypted_key = key.decrypt(private_key)?;

    // Save to local database
    crate::database::save_document_key(identity, decrypted_key).await?;

    log::debug!("[callback] Document key synced");
    Ok(())
}

/// Decrypts a version tag, saves to local DB, and emits the created event.
async fn sync_version_tag_created(
    identity: spacetimedb_sdk::Identity,
    tag: DocumentVersionTag,
) -> Result<(), String> {
    // Get document keys for decryption
    let document_keys = crate::database::get_document_keys_for_active_user().await?;

    // Decrypt the version tag
    let decrypted = vec![tag].decrypt_all(&document_keys)?;

    if let Some(decrypted_tag) = decrypted.into_iter().next() {
        // Save to local DB
        crate::database::save_version_tag(identity, decrypted_tag.clone()).await?;

        // Emit event for frontend
        emit_version_tag_created(
            decrypted_tag.tag_id,
            decrypted_tag.doc_id,
            decrypted_tag.data.tag_name,
            decrypted_tag.data.timestamp,
        );
        log::debug!("[callback] Version tag synced");
    }

    Ok(())
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
    let metadata: Vec<_> = ctx.db.user_metadata().iter().collect();
    let version_tags: Vec<_> = ctx.db.accessible_version_tags().iter().collect();

    std::thread::spawn(move || {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("[sync] Failed to create tokio runtime: {e}");
                return;
            }
        };

        rt.block_on(async {
            // Get user's private key for decrypting document keys and metadata
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

            // Decrypt and sync metadata with merge logic
            let decrypted_metadata = match metadata.decrypt_all(private_key) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("[sync] Failed to decrypt metadata: {e}");
                    return;
                }
            };

            match crate::database::sync_metadata_with_merge(identity, decrypted_metadata).await {
                Ok(path_changes) => {
                    log::debug!("[sync] Metadata synced");
                    // Emit events for path changes so frontend can notify user
                    for (doc_id, old_path, new_path) in path_changes {
                        emit_document_path_changed_by_sync(doc_id, old_path, new_path);
                    }
                }
                Err(e) => {
                    eprintln!("[sync] Failed to sync metadata: {e}");
                }
            }

            // Decrypt and sync version tags (need document keys, not user private key)
            let doc_keys = match crate::database::get_document_keys_for_active_user().await {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("[sync] Failed to get document keys for version tags: {e}");
                    log::info!("[sync] ✓ STDB data synced to local SQLite (version tags skipped)");
                    return;
                }
            };

            let decrypted_tags: Vec<_> = version_tags
                .into_iter()
                .filter_map(|tag| tag.decrypt(&doc_keys).ok())
                .collect();

            if let Err(e) = crate::database::sync_version_tags(identity, decrypted_tags).await {
                eprintln!("[sync] Failed to sync version tags: {e}");
            } else {
                log::debug!("[sync] Version tags synced");
            }

            log::info!("[sync] ✓ STDB data synced to local SQLite");
        });
    });
}

/// Called when the initial user-only subscription is applied (phase 1).
/// Does NOT sync documents - just signals that the user table is ready.
pub fn on_initial_subscription_applied(_ctx: &SubscriptionEventContext) {
    log::info!("✓ Initial subscription applied (user table only)");

    // Signal that initial subscriptions are ready
    // Document sync will happen after keys are synced via on_subscription_applied
    signal_subscription_ready();
}

/// Called when document subscriptions are applied (phase 2, or all at once if keys exist).
pub fn on_subscription_applied(ctx: &SubscriptionEventContext) {
    log::info!("✓ Subscriptions applied");

    // Sync all STDB data to local SQLite
    sync_stdb_to_local_db(ctx);

    // Signal that subscriptions are ready
    signal_subscription_ready();
}

#[allow(clippy::needless_pass_by_value)] // Signature constrained by SpacetimeDB callback API
pub fn on_subscription_error(_ctx: &ErrorContext, err: Error) {
    log::error!("✗ Subscription error: {err}");
}
