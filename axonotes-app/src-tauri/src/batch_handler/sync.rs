//! # Batch Synchronization
//!
//! Handles bidirectional sync of document batches between local storage and SpacetimeDB.
//!
//! ## Sync Flow
//!
//! 1. **Inbound**: SpacetimeDB subscription delivers new batches
//! 2. **Decrypt**: Batches are decrypted using document keys
//! 3. **Conflict Detection**: Check for local pending batches on same block
//! 4. **Storage**: Save to local SQLite database
//! 5. **Outbound**: Upload any pending local batches
//!
//! ## Conflict Handling
//!
//! When a server batch conflicts with local pending batches:
//! 1. **Reconstruct user's block state** (includes local pending patches)
//! 2. **Archive lost batches** to `sync_conflict_history` (local + STDB)
//! 3. **Delete local pending batches**
//! 4. **Save server batch**
//! 5. **Reconstruct server's block state**
//! 6. **Store conflict** in `pending_sync_conflicts` (for UI resolution)
//! 7. **Emit batch-conflicts event** with both states
//!
//! ## Lock Checking
//!
//! Before uploading, checks if the target block is locked by another user.
//! Locked blocks cannot be modified until the lock expires (60 seconds).

use crate::batch_handler::block_getter::{get_blocks, invalidate_block_cache};
use crate::database::{PendingSyncConflict, SyncConflictHistory};
use crate::encryption::batch::{DecryptDocumentBatchVec, DecryptedBatch};
use crate::encryption::conflict_history::DecryptedConflictHistory;
use crate::encryption::document::DecryptedDocumentKey;
use crate::encryption::live_block::DecryptedLiveBlock;
use crate::events::{
    emit_batch_conflict, emit_sync_completed, emit_sync_error, emit_sync_progress,
    emit_sync_started, BatchConflictPayload,
};
use crate::stdb_bindings::{AccessibleBatchesTableAccess, DbConnection, DocumentBatch};
use crate::utils::timestamp::timestamp;
use crate::utils::vec_array::ByteArrayConversion;
use crate::{database, stdb};
use once_cell::sync::OnceCell;
use spacetimedb_sdk::{DbContext, Identity, Table};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Global timestamp of the last successful sync operation.
/// Used to filter out already-processed batches on reconnect.
static LAST_SYNC: OnceCell<Arc<Mutex<u128>>> = OnceCell::new();

/// Returns a thread-safe reference to the last sync timestamp.
fn get_last_sync() -> Arc<Mutex<u128>> {
    LAST_SYNC.get_or_init(|| Arc::new(Mutex::new(0))).clone()
}

// ==========================================
// Setup
// ==========================================

/// Sets up the SpacetimeDB subscription for batch synchronization.
///
/// Subscribes to all batches newer than `start_time` and triggers sync
/// when new batches arrive. Uses two callbacks:
/// - `on_applied`: Processes all existing batches on initial subscription
/// - `on_insert`: Processes new batches as they arrive in real-time
///
/// # Arguments
///
/// * `conn` - Active SpacetimeDB connection
/// * `start_time` - Only sync batches created after this timestamp
pub fn setup_batch_sync(conn: &DbConnection, start_time: u128) -> Result<(), String> {
    // Register on_insert callback for real-time batch updates
    conn.db.accessible_batches().on_insert(|_ctx, batch| {
        log::debug!(
            "[callback:realtime] accessible_batches on_insert - doc: {}, batch: {}, timestamp: {}",
            batch.doc_id,
            batch.batch_id,
            batch.timestamp
        );
        let batch = batch.clone();
        // This callback runs on SpacetimeDB's background thread, NOT the Tokio runtime.
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                let doc_id = batch.doc_id.clone();
                // Use realtime sync - no timestamp filtering to avoid race conditions
                if let Err(e) = sync_realtime_batch(batch).await {
                    eprintln!("Batch sync error (on_insert): {e}");
                }
                // Check if this document has a pending doc_type resolution
                crate::stdb::callbacks::resolve_pending_doc_type(&doc_id).await;
            });
        });
    });

    // Subscribe to batches and process initial batch load
    conn.subscription_builder()
        .on_applied(|ctx| {
            let batches: Vec<DocumentBatch> = ctx.db.accessible_batches().iter().collect();

            // This callback runs on SpacetimeDB's background thread, NOT the Tokio runtime.
            // Use std::thread::spawn with a blocking runtime to run async code.
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                    if let Err(e) = sync_batches_with_data(batches).await {
                        eprintln!("Batch sync error: {e}");
                    }
                });
            });
        })
        .on_error(|_error_ctx, error| {
            eprintln!("Batch subscription error: {error:?}");
        })
        .subscribe([format!(
            "SELECT * FROM accessible_batches WHERE timestamp > {start_time}"
        )]);

    Ok(())
}

// ==========================================
// Public API
// ==========================================

/// Creates and saves a batch, uploading to server if connected.
///
/// If online, encrypts and uploads the batch immediately. If offline or
/// upload fails, saves as a pending batch for later upload.
///
/// # Arguments
///
/// * `batch` - The decrypted batch to save and potentially upload
///
/// # Errors
///
/// Returns an error if local database operations fail.
pub async fn create_batch(batch: DecryptedBatch) -> Result<(), String> {
    let last_sync_lock = get_last_sync();
    let mut last_sync = last_sync_lock.lock().await;

    let is_connected = stdb::active_profile().is_connected().await.unwrap_or(false);

    if is_connected {
        let document_keys = crate::database::get_document_keys_for_active_user().await?;

        match upload_batch(&batch, &document_keys).await {
            Ok(_) => {
                database::save_batch(batch.clone()).await?;
                *last_sync = batch.timestamp;
                database::set_last_active_profile_sync_time(*last_sync).await?;
            }
            Err(e) => {
                eprintln!("Batch upload failed, saving as pending: {e}");
                database::save_pending_batch(batch).await?;
            }
        }
    } else {
        database::save_pending_batch(batch).await?;
    }

    Ok(())
}

// ==========================================
// Sync Logic
// ==========================================

/// Maximum retries when waiting for document keys during batch sync.
/// Keys may arrive slightly after batches due to callback ordering.
const KEY_WAIT_MAX_RETRIES: u32 = 10;
/// Delay between retries when waiting for keys (100ms)
const KEY_WAIT_DELAY_MS: u64 = 100;

/// Syncs a single batch received via real-time subscription (on_insert).
///
/// Unlike `sync_batches_with_data`, this does NOT filter by timestamp.
/// Each on_insert callback fires exactly once per batch, so filtering
/// would cause race conditions where out-of-order batches get skipped.
///
/// This function:
/// 1. Acquires the sync lock to serialize with other batch operations
/// 2. Waits for document keys if not yet available (handles join race condition)
/// 3. Decrypts and saves the batch (if not a duplicate)
/// 4. Invalidates block cache so next read gets fresh data
/// 5. Emits sync events for frontend notification
async fn sync_realtime_batch(stdb_batch: DocumentBatch) -> Result<(), String> {
    let last_sync_lock = get_last_sync();
    let _last_sync = last_sync_lock.lock().await;

    let doc_id = stdb_batch.doc_id.clone();
    let batch_id = stdb_batch.batch_id.clone();
    let key_index = crate::utils::varint::decode(&stdb_batch.key_index)?;

    // Get document keys for decryption, with retry for race condition
    // When joining a share, batches and keys arrive via separate callbacks.
    // Keys may not be synced to local DB yet when batch callback fires.
    let document_keys = match wait_for_document_key(&doc_id, key_index).await? {
        KeyWaitResult::Found(keys) => keys,
        KeyWaitResult::SkipNoHistory => {
            // User joined a no-history share and doesn't have the old key
            // Skip this batch silently - they'll get the snapshot batches instead
            return Ok(());
        }
    };

    // Decrypt the batch
    let decrypted = vec![stdb_batch]
        .decrypt_all(&document_keys)
        .map_err(|e| format!("Failed to decrypt batch: {e}"))?;

    let Some(batch) = decrypted.into_iter().next() else {
        return Ok(()); // No batch to process
    };

    let block_id = batch.batch_data.block_id;

    // Check if batch already exists (avoid duplicates from initial sync + realtime)
    let existing = database::get_batch_by_id(batch_id.clone()).await?;
    if existing.is_some() {
        log::debug!(
            "[sync:realtime] Batch {} already exists, skipping",
            batch_id
        );
        return Ok(());
    }

    // Save the batch
    database::save_batch(batch).await?;

    // Invalidate cache so next block read gets fresh data
    invalidate_block_cache(doc_id.clone(), block_id).await;

    // Emit sync completed event for frontend to refresh
    emit_sync_completed(doc_id, 1);

    Ok(())
}

/// Result of waiting for a document key
enum KeyWaitResult {
    /// Key found, proceed with decryption
    Found(Vec<DecryptedDocumentKey>),
    /// Key not found but user has a newer key - skip this batch (no-history share)
    SkipNoHistory,
}

/// Waits for a document key to become available in local storage.
///
/// When joining a share, document keys and batches arrive via separate SpacetimeDB
/// callbacks that run in parallel. The key sync may not have completed when the
/// batch callback fires. This function retries fetching keys until the required
/// key is available or max retries is exceeded.
///
/// If the user has a NEWER key but not the requested one, this indicates a
/// no-history share where the user shouldn't see old content. In this case,
/// returns `SkipNoHistory` so the batch can be silently skipped.
async fn wait_for_document_key(doc_id: &str, key_index: u32) -> Result<KeyWaitResult, String> {
    for attempt in 0..KEY_WAIT_MAX_RETRIES {
        let document_keys = crate::database::get_document_keys_for_active_user().await?;

        // Check if we have the required key
        let has_key = document_keys
            .iter()
            .any(|k| k.doc_id == doc_id && k.key_index == key_index);

        if has_key {
            return Ok(KeyWaitResult::Found(document_keys));
        }

        // Check if we have a NEWER key for this document (no-history share scenario)
        // If so, we should skip this batch rather than wait forever
        let has_newer_key = document_keys
            .iter()
            .any(|k| k.doc_id == doc_id && k.key_index > key_index);

        if has_newer_key {
            log::debug!(
                "[sync:realtime] Skipping batch for doc {} with key_index {} (no-history share: user has newer key)",
                doc_id,
                key_index
            );
            return Ok(KeyWaitResult::SkipNoHistory);
        }

        if attempt < KEY_WAIT_MAX_RETRIES - 1 {
            log::debug!(
                "[sync:realtime] Waiting for key (doc: {}, index: {}), attempt {}/{}",
                doc_id,
                key_index,
                attempt + 1,
                KEY_WAIT_MAX_RETRIES
            );
            tokio::time::sleep(tokio::time::Duration::from_millis(KEY_WAIT_DELAY_MS)).await;
        }
    }

    Err(format!(
        "No key found for doc_id: {} with key_index: {} after {} retries",
        doc_id, key_index, KEY_WAIT_MAX_RETRIES
    ))
}

/// Processes incoming batches from SpacetimeDB subscription.
///
/// Groups batches by (doc_id, block_id) to handle conflicts correctly,
/// then processes each group as a unit to capture complete user and server states.
async fn sync_batches_with_data(stdb_batches_unfiltered: Vec<DocumentBatch>) -> Result<(), String> {
    let last_sync_lock = get_last_sync();
    let mut last_sync = last_sync_lock.lock().await;

    let stdb_batches: Vec<DocumentBatch> = stdb_batches_unfiltered
        .into_iter()
        .filter(|b| b.timestamp > *last_sync)
        .collect();

    if !stdb_batches.is_empty() {
        // Get unique doc_ids for sync events
        let doc_ids: HashSet<String> = stdb_batches.iter().map(|b| b.doc_id.clone()).collect();
        #[allow(clippy::cast_possible_truncation)] // Batch count bounded in practice
        let batch_count = stdb_batches.len() as u32;

        // Emit sync started for each document
        for doc_id in &doc_ids {
            emit_sync_started(doc_id.clone(), batch_count);
        }

        let document_keys = match crate::database::get_document_keys_for_active_user().await {
            Ok(keys) => keys,
            Err(e) => {
                for doc_id in &doc_ids {
                    emit_sync_error(doc_id.clone(), e.clone());
                }
                return Err(e);
            }
        };

        let decrypted_batches = match stdb_batches.decrypt_all(&document_keys) {
            Ok(batches) => batches,
            Err(e) => {
                for doc_id in &doc_ids {
                    emit_sync_error(doc_id.clone(), e.clone());
                }
                return Err(e);
            }
        };

        // Group batches by (doc_id, block_id) to process all batches for a block together
        let mut batches_by_block: HashMap<(String, u64), Vec<DecryptedBatch>> = HashMap::new();
        for batch in decrypted_batches {
            let key = (batch.doc_id.clone(), batch.batch_data.block_id);
            batches_by_block.entry(key).or_default().push(batch);
        }

        // Sort batches within each group by timestamp (oldest first)
        for batches in batches_by_block.values_mut() {
            batches.sort_by_key(|b| b.timestamp);
        }

        let mut conflict_payloads = Vec::new();
        #[allow(clippy::cast_possible_truncation)] // Batch count bounded in practice
        let total_count = batch_count;
        let mut synced_count: u32 = 0;

        // Process each (doc_id, block_id) group as a single sync operation
        for ((doc_id, block_id), server_batches) in batches_by_block {
            #[allow(clippy::cast_possible_truncation)]
            let group_batch_count = server_batches.len() as u32;

            if let Some(conflict_payload) =
                sync_block_batches(&doc_id, block_id, server_batches, &document_keys).await?
            {
                conflict_payloads.push(conflict_payload);
            }

            synced_count += group_batch_count;
            emit_sync_progress(doc_id, synced_count, total_count);
        }

        // Emit conflict events for each conflict
        for payload in &conflict_payloads {
            emit_batch_conflict(payload);
        }

        // Emit sync completed for each document
        for doc_id in doc_ids {
            emit_sync_completed(doc_id, batch_count);
        }
    }

    upload_pending_batches().await?;

    *last_sync = timestamp();
    database::set_last_active_profile_sync_time(*last_sync).await?;

    Ok(())
}

/// Syncs all batches for a single block, detecting and handling conflicts.
///
/// This function processes ALL server batches for a (doc_id, block_id) together
/// to ensure complete user and server states are captured during conflict detection.
///
/// Returns `Some(BatchConflictPayload)` if local pending batches were overwritten.
/// The conflict is saved locally for UI resolution and uploaded to STDB for backup.
async fn sync_block_batches(
    doc_id: &str,
    block_id: u64,
    server_batches: Vec<DecryptedBatch>,
    document_keys: &[DecryptedDocumentKey],
) -> Result<Option<BatchConflictPayload>, String> {
    let pending_conflicts =
        database::get_pending_batches_by_doc_and_block(doc_id.to_string(), block_id).await?;

    if pending_conflicts.is_empty() {
        // No conflict - just save all batches
        for batch in server_batches {
            database::save_batch(batch).await?;
        }
        return Ok(None);
    }

    // CONFLICT DETECTED!

    // 1. Reconstruct user's block state BEFORE any changes (includes pending patches)
    let user_block_data = get_blocks(doc_id.to_string(), vec![block_id], u128::MAX).await?;
    let user_state = user_block_data
        .blocks
        .first()
        .map(|bd| serde_json::to_string(&bd.block).unwrap_or_default())
        .unwrap_or_default();

    // 2. Get first lost batch timestamp (for ordering)
    let first_lost_timestamp = pending_conflicts
        .iter()
        .map(|b| b.timestamp)
        .min()
        .unwrap_or_else(timestamp);

    // 3. Archive lost batches to sync_conflict_history (local)
    let history_id = Uuid::new_v4().to_string();
    let conflict_history = SyncConflictHistory {
        history_id: history_id.clone(),
        doc_id: doc_id.to_string(),
        block_id,
        lost_batches: pending_conflicts.clone(),
        timestamp: first_lost_timestamp,
    };
    database::save_conflict_history(conflict_history).await?;

    // 4. Delete local pending batches
    for pending in &pending_conflicts {
        database::delete_batch(pending).await?;
    }

    // 5. Invalidate block cache
    invalidate_block_cache(doc_id.to_string(), block_id).await;

    // 6. Save ALL server batches
    for batch in &server_batches {
        database::save_batch(batch.clone()).await?;
    }

    // 7. Reconstruct server's block state AFTER ALL batches saved
    let server_block_data = get_blocks(doc_id.to_string(), vec![block_id], u128::MAX).await?;
    let server_state = server_block_data
        .blocks
        .first()
        .map(|bd| serde_json::to_string(&bd.block).unwrap_or_default())
        .unwrap_or_default();

    // 8. Delete old pending conflict for this block (if any - new conflict supersedes)
    database::delete_pending_conflict_by_block(doc_id.to_string(), block_id).await?;

    // 9. Generate conflict ID and save to pending_sync_conflicts
    let conflict_id = Uuid::new_v4().to_string();
    let conflict_timestamp = timestamp();
    let pending_conflict = PendingSyncConflict {
        conflict_id: conflict_id.clone(),
        doc_id: doc_id.to_string(),
        block_id,
        user_state: user_state.clone(),
        server_state: server_state.clone(),
        timestamp: conflict_timestamp,
    };
    database::save_pending_conflict(pending_conflict).await?;

    // 10. Upload to STDB (async, best-effort)
    let doc_id_for_upload = doc_id.to_string();
    let lost_batches_for_upload = pending_conflicts;
    let document_keys_for_upload = document_keys.to_vec();
    tokio::spawn(async move {
        if let Err(e) = upload_conflict_history_to_stdb(
            history_id,
            doc_id_for_upload,
            block_id,
            lost_batches_for_upload,
            first_lost_timestamp,
            &document_keys_for_upload,
        )
        .await
        {
            eprintln!("Failed to upload conflict history to STDB (non-fatal): {e}");
        }
    });

    // 11. Create payload for event emission
    let payload = BatchConflictPayload {
        conflict_id,
        doc_id: doc_id.to_string(),
        block_id,
        user_state,
        server_state,
        timestamp: conflict_timestamp,
    };

    Ok(Some(payload))
}

/// Uploads conflict history to STDB (best-effort, async)
async fn upload_conflict_history_to_stdb(
    history_id: String,
    doc_id: String,
    block_id: u64,
    lost_batches: Vec<DecryptedBatch>,
    timestamp: u128,
    document_keys: &[DecryptedDocumentKey],
) -> Result<(), String> {
    // Get user identity
    let user_identity = stdb::active_profile()
        .get_identity()
        .await?
        .ok_or("User identity not found")?;

    // Find the latest document key for encryption
    let latest_key = document_keys
        .iter()
        .filter(|k| k.doc_id == doc_id)
        .max_by_key(|k| k.key_timestamp)
        .ok_or("No document key found")?;

    // Create decrypted conflict history
    let decrypted_history = DecryptedConflictHistory::new(
        history_id.clone(),
        user_identity,
        doc_id.clone(),
        block_id,
        lost_batches,
        timestamp,
    );

    // Encrypt for upload
    let encrypted_history = decrypted_history.encrypt(latest_key)?;

    // Sign the message
    let user_keys = database::get_active_user_keys()
        .await?
        .ok_or("No active user keys")?;
    let private_signing_key = user_keys.private_signing_key.as_array()?;
    let signature = sign_conflict_history(
        &doc_id,
        &history_id,
        &encrypted_history.encrypted_blob,
        private_signing_key,
    )?;

    // Upload to STDB
    stdb::active_profile()
        .upload_conflict_history(
            encrypted_history.history_id,
            encrypted_history.doc_id,
            encrypted_history.encrypted_blob,
            encrypted_history.timestamp,
            encrypted_history.key_index,
            signature,
        )
        .await
}

/// Signs conflict history for STDB upload
fn sign_conflict_history(
    doc_id: &str,
    history_id: &str,
    encrypted_blob: &[u8],
    signing_key: &[u8; 32],
) -> Result<Vec<u8>, String> {
    let blob_hash = blake3::hash(encrypted_blob);
    let message = [
        b"upload_conflict_history".as_slice(),
        doc_id.as_bytes(),
        history_id.as_bytes(),
        blob_hash.as_bytes(),
    ]
    .concat();

    crate::crypto::ed25519::sign_message(signing_key, &message)
        .map(|sig| sig.to_vec())
        .map_err(|e| format!("Failed to sign conflict history: {e}"))
}

/// Uploads all pending local batches to the server.
///
/// Called after processing incoming batches to sync local changes.
async fn upload_pending_batches() -> Result<(), String> {
    let document_keys = crate::database::get_document_keys_for_active_user().await?;
    let pending = database::get_all_pending_batches().await?;

    for batch in pending {
        match upload_batch(&batch, &document_keys).await {
            Ok(_) => {
                database::mark_batch_synced(batch.batch_id).await?;
            }
            Err(e) => {
                eprintln!("Failed to upload pending batch {}: {}", batch.batch_id, e);
            }
        }
    }

    Ok(())
}

// ==========================================
// Helpers
// ==========================================

/// Encrypts, signs, and uploads a batch to SpacetimeDB.
///
/// Checks block locks before uploading to prevent overwriting
/// blocks being edited by other users.
async fn upload_batch(
    batch: &DecryptedBatch,
    document_keys: &[DecryptedDocumentKey],
) -> Result<(), String> {
    let live_blocks: Vec<DecryptedLiveBlock> =
        stdb::active_profile().get_cached_live_blocks().await?;
    let user_identity: Option<Identity> = stdb::active_profile().get_identity().await?;

    // Check if batch would write to a block locked by someone else
    let now = timestamp();
    const LOCK_TIMEOUT_MS: u128 = 60_000; // 60 seconds

    let is_block_locked_by_other = live_blocks.iter().any(|lb| {
        lb.doc_id == batch.doc_id
            && lb.block_id == batch.batch_data.block_id
            && lb.locked_at.is_some()
            && Some(lb.user_id) != user_identity
            && lb
                .locked_at
                .is_some_and(|locked_time| now - locked_time < LOCK_TIMEOUT_MS)
    });

    if is_block_locked_by_other {
        return Err(format!(
            "Cannot upload batch: block {} in document {} is locked by another user",
            batch.batch_data.block_id, batch.doc_id
        ));
    }

    let document_key = find_document_key(document_keys, batch.doc_id.as_str(), batch.timestamp)?;
    let signing_key = document_key.key_data.signing_private_key.as_array()?;

    let encrypted = batch.encrypt(document_key)?;

    let signature = crate::crypto::ed25519::sign_message(signing_key, &encrypted.encrypted_data)
        .map_err(|e| format!("Failed to sign batch: {e}"))?;

    stdb::active_profile()
        .upload_batch(
            encrypted.batch_id,
            encrypted.doc_id,
            encrypted.timestamp,
            encrypted.key_index,
            encrypted.encrypted_data,
            signature.to_vec(),
        )
        .await
}

/// Finds the correct document key for a given timestamp.
///
/// Returns the key with the highest timestamp that is still <= the batch timestamp.
fn find_document_key<'a>(
    keys: &'a [DecryptedDocumentKey],
    doc_id: &str,
    timestamp: u128,
) -> Result<&'a DecryptedDocumentKey, String> {
    keys.iter()
        .filter(|k| k.doc_id == doc_id && k.key_timestamp <= timestamp)
        .max_by_key(|k| k.key_timestamp)
        .ok_or_else(|| format!("No key found for doc {doc_id}"))
}
