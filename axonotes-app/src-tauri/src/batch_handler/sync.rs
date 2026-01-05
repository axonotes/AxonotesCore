use crate::encryption::batch::{DecryptDocumentBatchVec, DecryptedBatch};
use crate::encryption::document::DecryptedDocumentKey;
use crate::stdb_bindings::{AccessibleBatchesTableAccess, DbConnection, DocumentBatch};
use crate::utils::timestamp::timestamp;
use crate::utils::vec_array::ByteArrayConversion;
use crate::{app_handle, database, stdb};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use spacetimedb_sdk::{DbContext, Table};
use std::sync::Arc;
use tokio::sync::Mutex;

static LAST_SYNC: OnceCell<Arc<Mutex<u128>>> = OnceCell::new();

fn get_last_sync() -> Arc<Mutex<u128>> {
    LAST_SYNC.get_or_init(|| Arc::new(Mutex::new(0))).clone()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    pub doc_id: String,
    pub block_id: u64,
    pub server_batch: DecryptedBatch,
    pub local_pending_batches: Vec<DecryptedBatch>,
}

// ==========================================
// Setup
// ==========================================

pub fn setup_batch_sync(conn: &DbConnection, start_time: u128) -> Result<(), String> {
    conn.subscription_builder()
        .on_applied(|ctx| {
            let batches: Vec<DocumentBatch> = ctx.db.accessible_batches().iter().collect();

            tokio::spawn(async move {
                if let Err(e) = sync_batches_with_data(batches).await {
                    eprintln!("Batch sync error: {}", e);
                }
            });
        })
        .on_error(|_error_ctx, error| {
            eprintln!("Batch subscription error: {:?}", error);
        })
        .subscribe([format!(
            "SELECT * FROM accessible_batches WHERE timestamp > {start_time}"
        )]);

    Ok(())
}

// ==========================================
// Public API
// ==========================================

/// Create and save a batch, uploading to server if connected
pub async fn create_batch(batch: DecryptedBatch) -> Result<(), String> {
    let last_sync_lock = get_last_sync();
    let mut last_sync = last_sync_lock.lock().await;

    let is_connected = stdb::active_profile().is_connected().await.unwrap_or(false);

    if is_connected {
        let document_keys = stdb::active_profile().get_cached_document_keys().await?;

        match upload_batch(&batch, &document_keys).await {
            Ok(_) => {
                database::save_batch(batch.clone()).await?;
                *last_sync = batch.timestamp;
                database::set_last_active_profile_sync_time(*last_sync).await?;
            }
            Err(e) => {
                eprintln!("Batch upload failed, saving as pending: {}", e);
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

async fn sync_batches_with_data(stdb_batches_unfiltered: Vec<DocumentBatch>) -> Result<(), String> {
    let last_sync_lock = get_last_sync();
    let mut last_sync = last_sync_lock.lock().await;

    let stdb_batches: Vec<DocumentBatch> = stdb_batches_unfiltered
        .into_iter()
        .filter(|b| b.timestamp > *last_sync)
        .collect();

    if !stdb_batches.is_empty() {
        let document_keys = stdb::active_profile().get_cached_document_keys().await?;
        let decrypted_batches = stdb_batches.decrypt_all(&document_keys)?;

        let mut conflicts = Vec::new();
        for server_batch in decrypted_batches {
            if let Some(conflict) = sync_single_batch(server_batch).await? {
                conflicts.push(conflict);
            }
        }

        if !conflicts.is_empty() {
            emit_conflicts(conflicts);
        }
    }

    upload_pending_batches().await?;

    *last_sync = timestamp();
    database::set_last_active_profile_sync_time(*last_sync).await?;

    Ok(())
}

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
            database::delete_batch(pending.batch_id.clone()).await?;
        }

        Some(conflict)
    } else {
        None
    };

    database::save_batch(server_batch).await?;

    Ok(conflict_info)
}

async fn upload_pending_batches() -> Result<(), String> {
    let document_keys = stdb::active_profile().get_cached_document_keys().await?;
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

async fn upload_batch(
    batch: &DecryptedBatch,
    document_keys: &[DecryptedDocumentKey],
) -> Result<(), String> {
    let document_key = find_document_key(document_keys, batch.doc_id.as_str(), batch.timestamp)?;
    let signing_key = document_key.key_data.signing_private_key.as_array()?;

    let encrypted = batch.encrypt(document_key)?;

    let signature = crate::crypto::ed25519::sign_message(signing_key, &encrypted.encrypted_data)
        .map_err(|e| format!("Failed to sign batch: {}", e))?;

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

fn find_document_key<'a>(
    keys: &'a [DecryptedDocumentKey],
    doc_id: &str,
    timestamp: u128,
) -> Result<&'a DecryptedDocumentKey, String> {
    keys.iter()
        .filter(|k| k.doc_id == doc_id && k.key_timestamp <= timestamp)
        .max_by_key(|k| k.key_timestamp)
        .ok_or_else(|| format!("No key found for doc {}", doc_id))
}

fn emit_conflicts(conflicts: Vec<ConflictInfo>) {
    if let Err(e) = app_handle::emit("batch-conflicts", &conflicts) {
        eprintln!("Failed to emit conflicts: {}", e);
    }
}
