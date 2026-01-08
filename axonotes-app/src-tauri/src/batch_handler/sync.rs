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
//! - Local pending batches are deleted
//! - Server batch wins (last-write-wins)
//! - Conflict is emitted to frontend for user notification
//!
//! ## Lock Checking
//!
//! Before uploading, checks if the target block is locked by another user.
//! Locked blocks cannot be modified until the lock expires (60 seconds).

use crate::encryption::batch::{DecryptDocumentBatchVec, DecryptedBatch};
use crate::encryption::document::DecryptedDocumentKey;
use crate::encryption::live_block::DecryptedLiveBlock;
use crate::events::{emit_sync_completed, emit_sync_error, emit_sync_progress, emit_sync_started};
use crate::stdb_bindings::{AccessibleBatchesTableAccess, DbConnection, DocumentBatch};
use crate::utils::timestamp::timestamp;
use crate::utils::vec_array::ByteArrayConversion;
use crate::{app_handle, database, stdb};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use spacetimedb_sdk::{DbContext, Identity, Table};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Global timestamp of the last successful sync operation.
/// Used to filter out already-processed batches on reconnect.
static LAST_SYNC: OnceCell<Arc<Mutex<u128>>> = OnceCell::new();

/// Returns a thread-safe reference to the last sync timestamp.
fn get_last_sync() -> Arc<Mutex<u128>> {
    LAST_SYNC.get_or_init(|| Arc::new(Mutex::new(0))).clone()
}

/// Information about a sync conflict between local and server batches.
///
/// Emitted to the frontend when a server batch overwrites local pending changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Document where conflict occurred
    pub doc_id: String,
    /// Block that had conflicting edits
    pub block_id: u64,
    /// The winning server batch
    pub server_batch: DecryptedBatch,
    /// Local batches that were discarded
    pub local_pending_batches: Vec<DecryptedBatch>,
}

// ==========================================
// Setup
// ==========================================

/// Sets up the SpacetimeDB subscription for batch synchronization.
///
/// Subscribes to all batches newer than `start_time` and triggers sync
/// when new batches arrive.
///
/// # Arguments
///
/// * `conn` - Active SpacetimeDB connection
/// * `start_time` - Only sync batches created after this timestamp
pub fn setup_batch_sync(conn: &DbConnection, start_time: u128) -> Result<(), String> {
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

/// Processes incoming batches from SpacetimeDB subscription.
///
/// Filters batches newer than last sync, decrypts them, detects conflicts,
/// saves to local database, and uploads any pending local batches.
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

        let mut conflicts = Vec::new();
        #[allow(clippy::cast_possible_truncation)] // Batch count bounded in practice
        let total_count = decrypted_batches.len() as u32;
        let mut synced_count: u32 = 0;

        for server_batch in decrypted_batches {
            let batch_doc_id = server_batch.doc_id.clone();

            if let Some(conflict) = sync_single_batch(server_batch).await? {
                conflicts.push(conflict);
            }

            synced_count += 1;
            emit_sync_progress(batch_doc_id, synced_count, total_count);
        }

        if !conflicts.is_empty() {
            emit_conflicts(&conflicts);
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

/// Syncs a single batch, detecting and handling conflicts.
///
/// Returns `Some(ConflictInfo)` if local pending batches were overwritten.
async fn sync_single_batch(server_batch: DecryptedBatch) -> Result<Option<ConflictInfo>, String> {
    let doc_id = &server_batch.doc_id;
    let block_id = server_batch.batch_data.block_id;

    let pending_conflicts =
        database::get_pending_batches_by_doc_and_block(doc_id.clone(), block_id).await?;

    let conflict_info = if !pending_conflicts.is_empty() {
        let conflict = ConflictInfo {
            doc_id: doc_id.clone(),
            block_id,
            server_batch: server_batch.clone(),
            local_pending_batches: pending_conflicts.clone(),
        };

        for pending in &pending_conflicts {
            database::delete_batch(pending).await?;
        }

        Some(conflict)
    } else {
        None
    };

    database::save_batch(server_batch).await?;

    Ok(conflict_info)
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

/// Emits conflict information to the frontend via Tauri events.
fn emit_conflicts(conflicts: &[ConflictInfo]) {
    if let Err(e) = app_handle::emit("batch-conflicts", &conflicts) {
        eprintln!("Failed to emit conflicts: {e}");
    }
}
