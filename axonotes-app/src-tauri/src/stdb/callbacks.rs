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
    emit_document_metadata_updated, emit_document_path_changed_by_sync, emit_live_block_released,
    emit_live_block_updated, emit_version_tag_created, emit_version_tag_deleted,
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

// Pending doc type checks are now persisted to SQLite instead of being held
// in-memory. See database::add_pending_doc_type_check and friends.
// This ensures they survive app restarts.

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
    // Register live block insert callback (block locked/updated)
    // Decrypts the live block content and emits live-block-updated for real-time collaboration
    conn.db
        .accessible_live_blocks()
        .on_insert(|_ctx, live_block| {
            log::debug!(
                "[callback:realtime] accessible_live_blocks on_insert - doc: {}, block: {}, user: {}",
                live_block.doc_id,
                live_block.block_id,
                live_block.user_id.to_hex()
            );

            // Clone data for async task
            let live_block = live_block.clone();
            let lock_id = live_block.live_block_id.clone();

            // Remove from expired set if re-locked (sync, runs on STDB thread)
            let expired_locks = get_expired_locks();
            if let Ok(mut set) = expired_locks.lock() {
                set.remove(&lock_id);
            };

            // Spawn async task to decrypt and emit live-block-updated
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                    // Get document keys for decryption
                    let document_keys = match crate::database::get_document_keys_for_active_user().await {
                        Ok(keys) => keys,
                        Err(e) => {
                            log::warn!("[callback] Failed to get document keys for live block decryption: {}", e);
                            // Fall back to just emitting lock event without content
                            if let Some(locked_at) = live_block.locked_at {
                                emit_block_locked(
                                    live_block.doc_id.clone(),
                                    live_block.block_id,
                                    live_block.user_id.to_hex().to_string(),
                                    locked_at,
                                );
                            }
                            return;
                        }
                    };

                    // Decrypt the live block
                    match live_block.decrypt(&document_keys) {
                        Ok(decrypted) => {
                            // Emit live-block-updated with decrypted content
                            emit_live_block_updated(
                                decrypted.live_block_id,
                                decrypted.doc_id,
                                decrypted.block_id,
                                decrypted.user_id.to_hex().to_string(),
                                decrypted.username,
                                decrypted.locked_at,
                                decrypted.content,
                            );
                        }
                        Err(e) => {
                            log::warn!("[callback] Failed to decrypt live block: {}", e);
                            // Fall back to just emitting lock event without content
                            if let Some(locked_at) = live_block.locked_at {
                                emit_block_locked(
                                    live_block.doc_id.clone(),
                                    live_block.block_id,
                                    live_block.user_id.to_hex().to_string(),
                                    locked_at,
                                );
                            }
                        }
                    }
                });
            });
        });

    // Register live block delete callback (block unlocked/released)
    // Emits live-block-released for frontend - frontend handles debouncing via sliding window
    conn.db
        .accessible_live_blocks()
        .on_delete(|_ctx, live_block| {
            log::debug!(
                "[callback:realtime] accessible_live_blocks on_delete - doc: {}, block: {}, user: {}",
                live_block.doc_id,
                live_block.block_id,
                live_block.user_id.to_hex()
            );

            // Emit live-block-released for the frontend (clears liveInfo on the block)
            // Frontend uses sliding window to handle DELETE+INSERT update cycles
            emit_live_block_released(live_block.doc_id.clone(), live_block.block_id);

            // Also emit block-unlocked for backwards compatibility
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
    // For shared documents, this also creates default metadata if none exists
    conn.db.accessible_documents().on_insert(|ctx, document| {
        let document = document.clone();
        let identity = ctx.identity();
        let doc_id = document.doc_id.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                // Save the document
                if let Err(e) = crate::database::save_document(identity, document).await {
                    eprintln!("[callback] Failed to save document: {e}");
                    return;
                }

                // Check if this is a shared document without metadata
                // (shared documents don't get user_metadata created automatically)
                if let Err(e) = create_default_metadata_if_missing(&doc_id).await {
                    // Non-fatal: metadata might already exist or be created later
                    log::debug!(
                        "[callback] Could not create default metadata for {}: {}",
                        doc_id,
                        e
                    );
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
        decrypted.metadata.doc_type,
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

    // Retry any pending doc_type checks that survived from a previous session.
    // These are documents whose doc_type couldn't be determined at metadata
    // creation time because batches hadn't synced yet.
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
            resolve_all_pending_doc_type_checks().await;
        });
    });
}

#[allow(clippy::needless_pass_by_value)] // Signature constrained by SpacetimeDB callback API
pub fn on_subscription_error(_ctx: &ErrorContext, err: Error) {
    log::error!("✗ Subscription error: {err}");
}

// ==========================================
// Default Metadata for Shared Documents
// ==========================================

/// Creates default metadata for a document if the user doesn't have any yet.
///
/// This is used when a user joins a shared document - they get access to the
/// document and keys, but need to create their own user_metadata entry with
/// a default path so the document appears in their file tree.
///
/// Creates path like "/Shared/Shared Document.doc" with collision handling.
async fn create_default_metadata_if_missing(doc_id: &str) -> Result<(), String> {
    use crate::encryption::document::DecryptedMetadata;
    use crate::utils::vec_array::ByteArrayConversion;

    // Check if we already have metadata for this document in the STDB cache
    // get_cached_metadata returns already-decrypted metadata entries
    let existing_metadata = stdb::active_profile()
        .get_cached_metadata()
        .await
        .unwrap_or_default();

    let has_metadata = existing_metadata.iter().any(|m| m.doc_id == doc_id);
    if has_metadata {
        log::debug!(
            "[callback] Document {} already has metadata, skipping default creation",
            doc_id
        );
        return Ok(());
    }

    // Get user keys for encryption
    let user_keys = crate::database::get_active_user_keys()
        .await?
        .ok_or("No active user keys")?;

    let private_key = user_keys.private_encryption_key.as_array()?;
    let public_key = user_keys.public_encryption_key.as_array()?;

    // Get existing paths from the already-decrypted metadata
    let existing_paths: Vec<_> = existing_metadata.iter().map(|m| &m.metadata).collect();

    // Try to determine the document type from block content.
    // The creator writes a MetadataV1 block with field="doc_type".
    let doc_type = match read_doc_type_from_blocks(doc_id).await {
        DocTypeFromBlocks::Found(dt) => dt,
        DocTypeFromBlocks::NoBatches | DocTypeFromBlocks::NotFound => {
            // Either no batches yet, or batches exist but the doc_type block
            // hasn't been saved to local SQLite yet (race condition with
            // parallel batch sync). Defer the check.
            // When batches arrive for this doc_id, resolve_pending_doc_type()
            // will re-read the blocks and update the metadata.
            // Persisted to SQLite so it survives app restarts.
            log::debug!(
                "[callback] doc_type not resolvable yet for {}, deferring resolution",
                doc_id
            );
            if let Err(e) = crate::database::add_pending_doc_type_check(doc_id.to_string()).await {
                log::warn!(
                    "[callback] Failed to persist pending doc_type check for {}: {}",
                    doc_id,
                    e
                );
            }
            crate::encryption::document::DOC_TYPE_DOC.to_string()
        }
    };

    // Pick correct extension for the default path
    let extension = match doc_type.as_str() {
        crate::encryption::document::DOC_TYPE_TYPST => ".typst",
        _ => ".doc",
    };

    // Re-generate path with the correct extension
    let path = find_unique_shared_path_with_ext(&existing_paths, extension);

    // Create default metadata
    let default_metadata = DecryptedMetadata {
        version: 1,
        path: path.clone(),
        tags: vec![],
        doc_type,
    };

    // Encrypt for ourselves
    let encrypted_blob = default_metadata.encrypt(private_key, public_key, public_key)?;

    // Upload to server
    stdb::active_profile()
        .create_document_metadata(doc_id.to_string(), encrypted_blob)
        .await?;

    log::debug!(
        "[callback] Created default metadata for shared document {} at path {}",
        doc_id,
        path
    );

    Ok(())
}

/// Finds a unique path for a shared document with the given file extension.
/// Returns e.g. "/Shared/Shared Document.typst" or "/Shared/Shared Document (1).doc" etc.
fn find_unique_shared_path_with_ext(
    existing_metadata: &[&crate::encryption::document::DecryptedMetadata],
    extension: &str,
) -> String {
    let base_name = "Shared Document";
    let folder = "/Shared";

    // Collect all existing paths in lowercase for case-insensitive comparison
    let existing_paths: std::collections::HashSet<String> = existing_metadata
        .iter()
        .map(|m| m.path.to_lowercase())
        .collect();

    // Try the base name first
    let first_path = format!("{}/{}{}", folder, base_name, extension);
    if !existing_paths.contains(&first_path.to_lowercase()) {
        return first_path;
    }

    // Try with incrementing numbers
    for i in 1..1000 {
        let path = format!("{}/{} ({}){}", folder, base_name, i, extension);
        if !existing_paths.contains(&path.to_lowercase()) {
            return path;
        }
    }

    // Fallback with timestamp (should never happen)
    let ts = crate::utils::timestamp::timestamp();
    format!("{}/{} ({}){}", folder, base_name, ts, extension)
}

/// Result of reading doc_type from a document's block data.
enum DocTypeFromBlocks {
    /// No batches in local storage yet — can't determine doc_type.
    NoBatches,
    /// Batches exist but no MetadataV1 block with field="doc_type" was found.
    /// This could mean:
    ///   - Only partial batches have been saved (the doc_type block is still in flight)
    ///   - A pre-existing document created before doc_type was introduced
    NotFound,
    /// Explicit doc_type found in a MetadataV1 block.
    Found(String),
}

/// Tries to read the `doc_type` from a document's MetadataV1 blocks.
async fn read_doc_type_from_blocks(doc_id: &str) -> DocTypeFromBlocks {
    use crate::batch_handler::block_getter;
    use crate::batch_handler::block_types::BlockContent;

    // Get all block IDs for this document from local DB
    let batches = match crate::database::get_batches_by_doc(doc_id.to_string()).await {
        Ok(b) => b,
        Err(_) => return DocTypeFromBlocks::NoBatches,
    };

    if batches.is_empty() {
        return DocTypeFromBlocks::NoBatches;
    }

    let block_ids: std::collections::HashSet<u64> =
        batches.iter().map(|b| b.batch_data.block_id).collect();
    let block_ids: Vec<u64> = block_ids.into_iter().collect();

    // Reconstruct all blocks at latest timestamp
    let doc_data = match block_getter::get_blocks(doc_id.to_string(), block_ids, u128::MAX).await {
        Ok(d) => d,
        Err(_) => return DocTypeFromBlocks::NoBatches,
    };

    // Look for a MetadataV1 block with field="doc_type".
    // Also track if we find a "title" block — both "title" and "doc_type"
    // are created together in create_document. If "title" exists but
    // "doc_type" doesn't, it's a pre-existing document from before
    // doc_type was introduced (not a partial-sync race condition).
    let mut found_title = false;
    for block_data in &doc_data.blocks {
        if let BlockContent::MetadataV1(meta) = &block_data.block.content {
            match meta.field.as_str() {
                "doc_type" => {
                    if let Some(dt) = meta.value.as_str() {
                        return DocTypeFromBlocks::Found(dt.to_string());
                    }
                }
                "title" => {
                    found_title = true;
                }
                _ => {}
            }
        }
    }

    if found_title {
        // Title block exists but doc_type block doesn't → pre-existing
        // document created before doc_type was introduced. Default to "doc".
        DocTypeFromBlocks::Found(crate::encryption::document::DOC_TYPE_DOC.to_string())
    } else {
        // Neither title nor doc_type found — batches are still partial
        // (the MetadataV1 blocks haven't been saved yet).
        DocTypeFromBlocks::NotFound
    }
}

/// Resolves all pending doc_type checks that were persisted to SQLite.
/// Called on subscription applied to handle checks that survived app restarts.
///
/// For documents where batches haven't arrived yet (e.g. the timestamp-filtered
/// subscription missed them), subscribes to those documents' batches without
/// a timestamp filter. The existing `on_insert` callback will then pick up
/// the batches and call `resolve_pending_doc_type` for each.
async fn resolve_all_pending_doc_type_checks() {
    let pending = match crate::database::get_all_pending_doc_type_checks().await {
        Ok(ids) => ids,
        Err(e) => {
            log::warn!("[callback] Failed to load pending doc_type checks: {}", e);
            return;
        }
    };

    if pending.is_empty() {
        return;
    }

    log::info!(
        "[callback] Retrying {} pending doc_type checks from previous session",
        pending.len()
    );

    // First pass: try to resolve with locally available batches
    let mut still_pending = Vec::new();
    for doc_id in &pending {
        resolve_pending_doc_type(doc_id).await;

        // Check if it's still pending (couldn't resolve — batches not available)
        match crate::database::is_pending_doc_type_check(doc_id.clone()).await {
            Ok(true) => still_pending.push(doc_id.clone()),
            _ => {} // Resolved or error checking — either way, skip
        }
    }

    // Second pass: for documents whose batches are still missing, subscribe
    // to ALL their batches (no timestamp filter) so they arrive via on_insert.
    // The on_insert callback will call resolve_pending_doc_type for each batch.
    if !still_pending.is_empty() {
        log::info!(
            "[callback] {} pending doc_type checks still unresolved, subscribing to their batches",
            still_pending.len()
        );

        // Get active profile to find the connection
        let profile_id = match crate::database::get_active_profile().await {
            Ok(Some(p)) => p.id,
            _ => {
                log::warn!("[callback] No active profile, can't subscribe to pending doc batches");
                return;
            }
        };

        let conn_arc = match stdb::get_connection_for_profile(&profile_id).await {
            Ok(c) => c,
            Err(e) => {
                log::warn!(
                    "[callback] Can't get STDB connection for pending batch fetch: {}",
                    e
                );
                return;
            }
        };

        let conn = conn_arc.lock().await;
        if let Err(e) =
            crate::batch_handler::sync::subscribe_all_batches_for_docs(&conn, &still_pending)
        {
            log::warn!(
                "[callback] Failed to subscribe to pending doc batches: {}",
                e
            );
        }
    }
}

/// Called after a batch is synced for a document. If that document has a
/// pending doc_type check, resolves the type and updates per-user metadata.
///
/// There are three possible outcomes from `read_doc_type_from_blocks`:
///
/// - `Found("typst")` — explicit doc_type block found. Update metadata,
///   remove pending check.
/// - `Found("doc")` — explicit doc_type block found with value "doc".
///   Already correct, just remove pending check.
/// - `NotFound` — batches exist but the doc_type MetadataV1 block hasn't
///   been saved yet (race condition: only partial batches in local SQLite).
///   Keep the pending check so the next batch sync retries.
/// - `NoBatches` — no batches at all. Keep pending for later.
pub async fn resolve_pending_doc_type(doc_id: &str) {
    use crate::encryption::document::DecryptedMetadata;
    use crate::utils::vec_array::ByteArrayConversion;

    // Check if this doc_id is in the pending set (persisted in SQLite)
    let is_pending = crate::database::is_pending_doc_type_check(doc_id.to_string())
        .await
        .unwrap_or(false);
    if !is_pending {
        return;
    }

    // Try to read the doc_type now that batches may be available
    let doc_type = match read_doc_type_from_blocks(doc_id).await {
        DocTypeFromBlocks::Found(dt) => dt,
        DocTypeFromBlocks::NoBatches => {
            // Still no batches at all — will retry on next batch sync
            log::debug!(
                "[callback] resolve_pending_doc_type({}): still no batches, keeping pending",
                doc_id
            );
            return;
        }
        DocTypeFromBlocks::NotFound => {
            // Batches exist but the doc_type MetadataV1 block wasn't found.
            // This is likely a race condition — only some batches have been
            // saved to local SQLite, and the doc_type block is in another
            // batch that hasn't been saved yet. Keep the pending check.
            log::debug!(
                "[callback] resolve_pending_doc_type({}): batches exist but doc_type block not found yet, keeping pending",
                doc_id
            );
            return;
        }
    };

    // We found an explicit doc_type. Remove from pending set.
    if let Err(e) = crate::database::remove_pending_doc_type_check(doc_id.to_string()).await {
        log::warn!(
            "[callback] Failed to remove pending doc_type check for {}: {}",
            doc_id,
            e
        );
    }

    log::info!(
        "[callback] resolve_pending_doc_type({}): resolved to \"{}\"",
        doc_id,
        doc_type
    );

    // If it resolved to "doc" (the default we already set), nothing to update
    if doc_type == crate::encryption::document::DOC_TYPE_DOC {
        return;
    }

    // Need to update the metadata with the real doc_type and correct extension
    let identity = match stdb::active_profile().get_identity().await {
        Ok(Some(id)) => id,
        _ => return,
    };

    let current_meta =
        match crate::database::get_metadata_for_doc(identity, doc_id.to_string()).await {
            Ok(Some(m)) => m,
            _ => return,
        };

    // If metadata already has the correct doc_type, no update needed
    if current_meta.metadata.doc_type == doc_type {
        log::debug!(
            "[callback] resolve_pending_doc_type({}): metadata already has correct doc_type \"{}\"",
            doc_id,
            doc_type
        );
        return;
    }

    // Update path extension from .doc to the correct one
    let extension = match doc_type.as_str() {
        crate::encryption::document::DOC_TYPE_TYPST => ".typst",
        _ => return, // Unknown type, leave as-is
    };
    let new_path = if current_meta.metadata.path.ends_with(".doc") {
        format!(
            "{}{}",
            &current_meta.metadata.path[..current_meta.metadata.path.len() - 4],
            extension
        )
    } else {
        current_meta.metadata.path.clone()
    };

    let updated_metadata = DecryptedMetadata {
        version: current_meta.metadata.version,
        path: new_path.clone(),
        tags: current_meta.metadata.tags.clone(),
        doc_type: doc_type.clone(),
    };

    // Encrypt and save locally + push to server
    let user_keys = match crate::database::get_active_user_keys().await {
        Ok(Some(keys)) => keys,
        _ => return,
    };
    let Ok(private_key) = user_keys.private_encryption_key.as_array() else {
        return;
    };
    let Ok(public_key) = user_keys.public_encryption_key.as_array() else {
        return;
    };

    let updated = crate::encryption::document::DecryptedDocumentMetadata {
        meta_id: current_meta.meta_id,
        user_id: current_meta.user_id,
        doc_id: doc_id.to_string(),
        metadata: updated_metadata.clone(),
    };

    if let Err(e) = crate::database::save_metadata(identity, updated).await {
        log::warn!("[callback] Failed to update metadata for deferred doc_type: {e}");
        return;
    }

    emit_document_metadata_updated(
        doc_id.to_string(),
        new_path.clone(),
        updated_metadata.tags.clone(),
        updated_metadata.doc_type.clone(),
    );

    // Push to server
    if let Ok(encrypted_blob) = updated_metadata.encrypt(private_key, public_key, public_key) {
        if let Err(e) = stdb::active_profile()
            .update_document_metadata(doc_id.to_string(), encrypted_blob)
            .await
        {
            log::warn!("[callback] Failed to push deferred doc_type to server: {e}");
        }
    }

    log::info!(
        "[callback] Resolved deferred doc_type for document {} → \"{}\" (path: {})",
        doc_id,
        doc_type,
        new_path,
    );
}
