//! # Open Document Handle
//!
//! Manages the lifecycle of a single open document, including:
//!
//! - Block state reconstruction and caching
//! - Frontend mirror tracking (what the frontend currently has)
//! - Event forwarding (live updates, sync completions, lock changes)
//! - Lock management (auto-acquire, auto-release, inactivity timeout)
//! - Time-travel (switching between live and history modes)
//!
//! ## Global State
//!
//! All open documents are tracked in the global `OPEN_DOCUMENTS` map.
//! The event forwarding system listens to existing Tauri events
//! (`live-block-updated`, `live-block-released`, `sync-completed`,
//! `block-lock-expired`) and transforms them into `block_update_{doc_id}`
//! events using the frontend mirror to compute diffs.
//!
//! ## Thread Safety
//!
//! Each `OpenDocumentHandle` is wrapped in a `tokio::sync::Mutex` to
//! serialize access. The outer `DashMap` allows concurrent access to
//! different documents.

use crate::app_handle;
use crate::batch_handler::block_getter::{self, BlockData, DocumentData};
use crate::batch_handler::block_setter;
use crate::batch_handler::block_types::Block;
use crate::database;
use crate::document_state::mirror::FrontendMirror;
use crate::document_state::types::{BlockUpdate, DocumentMode, LockInfo, LockState};
use crate::encryption::live_block::DecryptedLiveBlock;
use crate::stdb;
use crate::utils::timestamp::timestamp;
use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::LazyLock;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

// ==========================================
// Constants
// ==========================================

/// How long (ms) a lock is held without any edits before it is automatically
/// released. Matches the frontend's previous `LOCK_INACTIVITY_MS`.
const LOCK_INACTIVITY_MS: u64 = 60_000;

/// Throttle interval (ms) for live update broadcasts.
/// Matches the frontend's previous `LIVE_UPDATE_THROTTLE`.
const LIVE_UPDATE_THROTTLE_MS: u64 = 100;

/// Sliding window (ms) for release events.
/// SpacetimeDB models updates as DELETE+INSERT, so we wait briefly before
/// clearing lock info to avoid flickering.
const RELEASE_WINDOW_MS: u64 = 50;

// ==========================================
// Global State
// ==========================================

/// Global registry of all currently open documents.
///
/// Key: `doc_id`, Value: `Mutex<OpenDocumentHandle>`.
/// Using `DashMap` for the outer map allows concurrent access to different
/// documents, while the inner `Mutex` serializes operations on each document.
static OPEN_DOCUMENTS: LazyLock<DashMap<String, Mutex<OpenDocumentHandle>>> =
    LazyLock::new(DashMap::new);

// ==========================================
// Open Document Handle
// ==========================================

/// State for a single open document.
///
/// Manages block data, the frontend mirror, lock state, and live update throttling.
pub struct OpenDocumentHandle {
    doc_id: String,
    mode: DocumentMode,
    mirror: FrontendMirror,
    my_identity_hex: String,

    // Lock management
    locked_block_id: Option<u64>,
    lock_inactivity_handle: Option<JoinHandle<()>>,

    // Live update throttling (serialized: only one call in flight at a time)
    live_update_block_id: Option<u64>,
    live_update_timer: Option<JoinHandle<()>>,
    live_update_in_flight: bool,
    live_update_queued: bool,

    // Pending release sliding windows (block_id -> timer handle)
    pending_releases: std::collections::HashMap<u64, JoinHandle<()>>,
}

impl OpenDocumentHandle {
    fn new(doc_id: String, my_identity_hex: String) -> Self {
        Self {
            doc_id,
            mode: DocumentMode::Live,
            mirror: FrontendMirror::new(),
            my_identity_hex,
            locked_block_id: None,
            lock_inactivity_handle: None,
            live_update_block_id: None,
            live_update_timer: None,
            live_update_in_flight: false,
            live_update_queued: false,
            pending_releases: std::collections::HashMap::new(),
        }
    }
}

// ==========================================
// Public API
// ==========================================

/// Open a document: reconstruct state, register event forwarding,
/// push initial state to the frontend via the event channel.
///
/// If the document is already open, it is closed first (idempotent).
///
/// # Errors
///
/// Returns an error if the document doesn't exist, we lack access keys,
/// or block reconstruction fails.
pub async fn open_document(doc_id: String) -> Result<(), String> {
    // Close existing session if already open (idempotent)
    if OPEN_DOCUMENTS.contains_key(&doc_id) {
        close_document(doc_id.clone()).await?;
    }

    // Get current identity
    let identity_hex = stdb::active_profile()
        .get_identity()
        .await?
        .ok_or("No identity available")?
        .to_hex()
        .to_string();

    // Reconstruct all blocks at current time
    let all_block_ids = get_all_block_ids(&doc_id).await?;
    let document_data =
        block_getter::get_blocks(doc_id.clone(), all_block_ids, timestamp()).await?;

    // Fetch existing live locks
    let live_locks: Vec<DecryptedLiveBlock> =
        stdb::active_profile().get_cached_live_blocks().await?;
    let doc_locks: Vec<&DecryptedLiveBlock> =
        live_locks.iter().filter(|l| l.doc_id == doc_id).collect();

    // Release ghost locks from our own identity (leftover from crashed sessions)
    let my_ghost_locks: Vec<&DecryptedLiveBlock> = doc_locks
        .iter()
        .filter(|l| l.user_id.to_hex().to_string() == identity_hex)
        .copied()
        .collect();

    for ghost in &my_ghost_locks {
        if let Err(e) = stdb::active_profile()
            .unlock_block(doc_id.clone(), ghost.block_id, true)
            .await
        {
            log::warn!("[doc_state] Failed to release ghost lock: {e}");
        }
    }

    // Build initial block updates with lock info
    let mut initial_updates: Vec<BlockUpdate> = Vec::new();
    for bd in &document_data.blocks {
        if bd.block.deleted.unwrap_or(false) {
            continue;
        }

        // Find lock info for this block (skip our own ghost locks)
        let lock = doc_locks
            .iter()
            .find(|l| {
                l.block_id == bd.block_id
                    && l.user_id.to_hex().to_string() != identity_hex
            })
            .map(|l| LockInfo {
                user_id: l.user_id.to_hex().to_string(),
                username: l.username.clone(),
                state: if l.locked_at.is_some() {
                    LockState::Locked
                } else {
                    LockState::Focused
                },
            });

        initial_updates.push(BlockUpdate::Set {
            block_id: bd.block_id,
            block: bd.block.clone(),
            lock,
        });
    }

    // Create the handle
    let mut handle = OpenDocumentHandle::new(doc_id.clone(), identity_hex);

    // Apply initial state to mirror
    handle.mirror.apply_updates(&initial_updates);

    // Register handle BEFORE emitting events (so event forwarding can find it)
    OPEN_DOCUMENTS.insert(doc_id.clone(), Mutex::new(handle));

    // Emit initial state to frontend
    emit_block_updates(&doc_id, &initial_updates);

    log::info!(
        "[doc_state] Opened document {} ({} blocks)",
        doc_id,
        initial_updates.len()
    );

    Ok(())
}

/// Close a document: release locks, flush pending data, tear down event forwarding.
///
/// No-op if the document is not currently open.
pub async fn close_document(doc_id: String) -> Result<(), String> {
    // Remove from global map (this also prevents further event forwarding)
    let entry = OPEN_DOCUMENTS.remove(&doc_id);

    if let Some((_, handle_mutex)) = entry {
        let mut handle = handle_mutex.lock().await;

        // Release any lock we hold
        if let Some(block_id) = handle.locked_block_id.take() {
            // Cancel live update timers
            cancel_live_update_timers(&mut handle);

            if let Err(e) = stdb::active_profile()
                .unlock_block(doc_id.clone(), block_id, true)
                .await
            {
                log::debug!("[doc_state] Lock release on close failed (may be already released): {e}");
            }
        }

        // Cancel inactivity timer
        if let Some(timer) = handle.lock_inactivity_handle.take() {
            timer.abort();
        }

        // Cancel all pending release timers
        for (_, timer) in handle.pending_releases.drain() {
            timer.abort();
        }

        // Clear mirror
        handle.mirror.clear();

        log::info!("[doc_state] Closed document {}", doc_id);
    }

    Ok(())
}

/// Close all open documents. Called during window close / app shutdown.
pub async fn close_all_documents() -> Result<(), String> {
    let doc_ids: Vec<String> = OPEN_DOCUMENTS
        .iter()
        .map(|e| e.key().clone())
        .collect();

    for doc_id in doc_ids {
        if let Err(e) = close_document(doc_id.clone()).await {
            log::warn!("[doc_state] Failed to close document {doc_id}: {e}");
        }
    }

    Ok(())
}

/// Set the document time: switch between live mode and history mode.
///
/// Reconstructs the document at the given timestamp, diffs against the
/// frontend mirror, and emits only the changes. Uses `u64::MAX` to
/// return to live mode.
///
/// # History Mode
///
/// When viewing a historical timestamp, live updates and sync events are
/// ignored (the document is a frozen snapshot). Editing is not allowed.
///
/// # Errors
///
/// Returns an error if the document is not open or reconstruction fails.
pub async fn set_time(doc_id: String, ts: u64) -> Result<(), String> {
    let handle_ref = OPEN_DOCUMENTS
        .get(&doc_id)
        .ok_or("Document not open")?;
    let mut handle = handle_ref.lock().await;

    let is_live = ts == u64::MAX;

    // If we're leaving live mode, release any lock we hold
    if !is_live {
        if let Some(block_id) = handle.locked_block_id.take() {
            cancel_live_update_timers(&mut handle);
            if let Some(timer) = handle.lock_inactivity_handle.take() {
                timer.abort();
            }
            let doc_id_clone = doc_id.clone();
            tokio::spawn(async move {
                let _ = stdb::active_profile()
                    .unlock_block(doc_id_clone, block_id, true)
                    .await;
            });
        }
    }

    // Reconstruct blocks at the target timestamp
    let target_ts = if is_live { timestamp() } else { ts as u128 };
    let all_block_ids = get_all_block_ids(&doc_id).await?;
    let document_data =
        block_getter::get_blocks(doc_id.clone(), all_block_ids, target_ts).await?;

    // Build desired state (no locks in history mode)
    let desired: Vec<(u64, Block, Option<LockInfo>)> = document_data
        .blocks
        .iter()
        .filter(|bd| !bd.block.deleted.unwrap_or(false))
        .map(|bd| (bd.block_id, bd.block.clone(), None))
        .collect();

    // If returning to live mode, also fetch current locks
    let desired = if is_live {
        let live_locks = stdb::active_profile()
            .get_cached_live_blocks()
            .await
            .unwrap_or_default();
        desired
            .into_iter()
            .map(|(id, block, _)| {
                let lock = live_locks
                    .iter()
                    .find(|l| {
                        l.block_id == id
                            && l.doc_id == doc_id
                            && l.user_id.to_hex().to_string() != handle.my_identity_hex
                    })
                    .map(|l| LockInfo {
                        user_id: l.user_id.to_hex().to_string(),
                        username: l.username.clone(),
                        state: if l.locked_at.is_some() {
                            LockState::Locked
                        } else {
                            LockState::Focused
                        },
                    });
                (id, block, lock)
            })
            .collect()
    } else {
        desired
    };

    // Diff against mirror
    let updates = handle.mirror.diff(&desired);

    if !updates.is_empty() {
        // Apply to mirror and emit
        handle.mirror.apply_updates(&updates);
        emit_block_updates(&doc_id, &updates);
    }

    // Update mode
    handle.mode = if is_live {
        DocumentMode::Live
    } else {
        DocumentMode::History {
            timestamp: target_ts,
        }
    };

    log::info!(
        "[doc_state] set_time for {} -> {} ({} updates)",
        doc_id,
        if is_live {
            "live".to_string()
        } else {
            format!("t={target_ts}")
        },
        updates.len()
    );

    Ok(())
}

/// Update a block's content. The backend handles:
///
/// 1. Lock validation and auto-acquisition
/// 2. Persistence via the throttled batch system
/// 3. Live broadcast to collaborators
/// 4. Frontend mirror update (no echo event for own edits)
///
/// # Lock Lifecycle
///
/// - Lock is acquired on the first edit if not already held.
/// - If a different block is currently locked, the old lock is released first.
/// - An inactivity timer auto-releases the lock after 60s of silence.
///
/// # Errors
///
/// - Document not open
/// - Document in history mode (read-only)
/// - Block locked by another user
pub async fn update_block(doc_id: String, block_id: u64, content: Block) -> Result<(), String> {
    let handle_ref = OPEN_DOCUMENTS
        .get(&doc_id)
        .ok_or("Document not open")?;
    let mut handle = handle_ref.lock().await;

    // Verify live mode
    if matches!(handle.mode, DocumentMode::History { .. }) {
        return Err("Document is in read-only history mode".to_string());
    }

    // Lock management: auto-acquire or validate
    let currently_locked = handle.locked_block_id;

    if currently_locked != Some(block_id) {
        // Release old lock if switching blocks
        if let Some(old_block_id) = currently_locked {
            cancel_live_update_timers(&mut handle);
            handle.locked_block_id = None;

            let doc_id_clone = doc_id.clone();
            tokio::spawn(async move {
                let _ = stdb::active_profile()
                    .unlock_block(doc_id_clone, old_block_id, true)
                    .await;
            });
        }

        // Acquire lock on new block
        stdb::active_profile()
            .try_lock_block(
                doc_id.clone(),
                block_id,
                &content,
                "User".to_string(), // TODO: get actual username from profile
            )
            .await?;

        handle.locked_block_id = Some(block_id);
    }

    // Persist via the throttled batch system
    block_setter::update_block(doc_id.clone(), Some(block_id), content.clone());

    // Schedule live broadcast to collaborators
    schedule_live_update(&mut handle, doc_id.clone(), block_id, &content);

    // Reset inactivity timer
    reset_inactivity_timer(&mut handle, doc_id.clone());

    // Update mirror directly (no event emitted for own edits)
    handle.mirror.set(block_id, content, None);

    Ok(())
}

/// Create a new block in the document. Returns the real block ID immediately.
///
/// Unlike the old frontend flow, there are no temp IDs. The backend generates
/// the block ID and persists synchronously.
///
/// ## Immediate Flush
///
/// After creating the block, the batch is flushed immediately. A brand new
/// block is a single initial patch with nothing to coalesce — holding it in
/// the 500ms throttle window just delays visibility for collaborators. By
/// flushing immediately the batch contains exactly one initial patch and
/// gets uploaded/synced right away so the new line appears on the other side
/// without delay.
///
/// # Errors
///
/// - Document not open
/// - Document in history mode (read-only)
pub async fn create_block(doc_id: String, block: Block) -> Result<u64, String> {
    let handle_ref = OPEN_DOCUMENTS
        .get(&doc_id)
        .ok_or("Document not open")?;
    let mut handle = handle_ref.lock().await;

    // Verify live mode
    if matches!(handle.mode, DocumentMode::History { .. }) {
        return Err("Document is in read-only history mode".to_string());
    }

    // Generate real block ID and persist
    let block_id = block_setter::update_block(doc_id.clone(), None, block.clone());

    // Immediately flush — a new block is a single initial patch, nothing to
    // coalesce. Skipping the throttle window ensures collaborators see the
    // new line instantly.
    if let Err(e) = block_setter::flush_block(&doc_id, block_id).await {
        log::warn!("[doc_state] Failed to flush new block {block_id}: {e}");
    }

    // Update mirror with the new block
    handle.mirror.set(block_id, block, None);

    Ok(block_id)
}

/// Soft-delete a block (marks it as deleted, preserving history).
///
/// If we hold a lock on this block, it is released.
///
/// # Errors
///
/// - Document not open
/// - Document in history mode (read-only)
/// - Block doesn't exist
pub async fn delete_block(doc_id: String, block_id: u64) -> Result<(), String> {
    let handle_ref = OPEN_DOCUMENTS
        .get(&doc_id)
        .ok_or("Document not open")?;
    let mut handle = handle_ref.lock().await;

    // Verify live mode
    if matches!(handle.mode, DocumentMode::History { .. }) {
        return Err("Document is in read-only history mode".to_string());
    }

    // Release lock if we hold it for this block
    if handle.locked_block_id == Some(block_id) {
        cancel_live_update_timers(&mut handle);
        handle.locked_block_id = None;

        if let Some(timer) = handle.lock_inactivity_handle.take() {
            timer.abort();
        }

        let doc_id_clone = doc_id.clone();
        tokio::spawn(async move {
            let _ = stdb::active_profile()
                .unlock_block(doc_id_clone, block_id, true)
                .await;
        });
    }

    // Fetch current block state for the soft-delete
    let doc_data: DocumentData =
        block_getter::get_blocks(doc_id.clone(), vec![block_id], timestamp()).await?;
    if doc_data.blocks.is_empty() {
        return Err("Can't delete block that doesn't exist.".to_string());
    }
    let block_data: BlockData = doc_data.blocks.into_iter().next().unwrap();
    block_setter::update_block(
        doc_id.clone(),
        Some(block_id),
        Block {
            deleted: Some(true),
            ..block_data.block
        },
    );

    // Remove from mirror
    handle.mirror.remove(block_id);

    // Emit removal to frontend
    let updates = vec![BlockUpdate::Remove { block_id }];
    emit_block_updates(&doc_id, &updates);

    Ok(())
}

// ==========================================
// Event Forwarding
// ==========================================

/// Called by the STDB callback system when a live block update arrives.
///
/// If the affected document is open and in live mode, this transforms the
/// update into a `BlockUpdate::Set` and emits it to the frontend.
/// Own updates are ignored (the frontend already has them).
pub async fn on_live_block_updated(decrypted: DecryptedLiveBlock) {
    let doc_id = &decrypted.doc_id;

    let handle_ref = match OPEN_DOCUMENTS.get(doc_id) {
        Some(h) => h,
        None => return, // Document not open, ignore
    };
    let mut handle = handle_ref.lock().await;

    // Ignore if in history mode
    if matches!(handle.mode, DocumentMode::History { .. }) {
        return;
    }

    // Ignore our own updates
    if decrypted.user_id.to_hex().to_string() == handle.my_identity_hex {
        return;
    }

    // Cancel any pending release for this block (sliding window)
    if let Some(timer) = handle.pending_releases.remove(&decrypted.block_id) {
        timer.abort();
    }

    let lock = Some(LockInfo {
        user_id: decrypted.user_id.to_hex().to_string(),
        username: decrypted.username.clone(),
        state: if decrypted.locked_at.is_some() {
            LockState::Locked
        } else {
            LockState::Focused
        },
    });

    let block_id = decrypted.block_id;
    let block = decrypted.content;

    // Only emit if this actually changes the mirror
    let needs_update = match handle.mirror.get(block_id) {
        None => true,
        Some(mirrored) => {
            let content_changed = serde_json::to_vec(&mirrored.block).ok()
                != serde_json::to_vec(&block).ok();
            let lock_changed = mirrored.lock != lock;
            content_changed || lock_changed
        }
    };

    if needs_update {
        let updates = vec![BlockUpdate::Set {
            block_id,
            block: block.clone(),
            lock: lock.clone(),
        }];
        handle.mirror.apply_updates(&updates);
        emit_block_updates(doc_id, &updates);
    }
}

/// Called by the STDB callback system when a live block is released.
///
/// Uses a sliding window to avoid flickering from SpacetimeDB's DELETE+INSERT
/// update model. After the window, the lock is cleared from the block.
pub async fn on_live_block_released(doc_id: String, block_id: u64) {
    let handle_ref = match OPEN_DOCUMENTS.get(&doc_id) {
        Some(h) => h,
        None => return,
    };
    let mut handle = handle_ref.lock().await;

    // Ignore if in history mode
    if matches!(handle.mode, DocumentMode::History { .. }) {
        return;
    }

    // Cancel any existing pending release for this block
    if let Some(timer) = handle.pending_releases.remove(&block_id) {
        timer.abort();
    }

    // Schedule release after sliding window
    let doc_id_clone = doc_id.clone();
    let timer = tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(RELEASE_WINDOW_MS)).await;
        apply_block_release(doc_id_clone, block_id).await;
    });

    handle.pending_releases.insert(block_id, timer);
}

/// Apply a block release after the sliding window has elapsed.
async fn apply_block_release(doc_id: String, block_id: u64) {
    let handle_ref = match OPEN_DOCUMENTS.get(&doc_id) {
        Some(h) => h,
        None => return,
    };
    let mut handle = handle_ref.lock().await;

    handle.pending_releases.remove(&block_id);

    // Update mirror: clear lock but keep block content
    if handle.mirror.update_lock(block_id, None) {
        // Re-read the block from mirror to emit full update
        if let Some(mirrored) = handle.mirror.get(block_id) {
            let updates = vec![BlockUpdate::Set {
                block_id,
                block: mirrored.block.clone(),
                lock: None,
            }];
            emit_block_updates(&doc_id, &updates);
        }
    }
}

/// Called by the STDB callback system when a lock expires (60s timeout).
pub async fn on_lock_expired(doc_id: String, block_id: u64) {
    // Treat exactly like a release
    apply_block_release(doc_id, block_id).await;
}

/// Called when a sync completes for a document (new batches arrived from server).
///
/// Re-fetches all blocks, diffs against the mirror, and emits only changes.
/// Preserves our own locked block's content (we hold the authoritative text).
pub async fn on_sync_completed(doc_id: String, batch_count: u32) {
    if batch_count == 0 {
        return;
    }

    let handle_ref = match OPEN_DOCUMENTS.get(&doc_id) {
        Some(h) => h,
        None => return,
    };
    let mut handle = handle_ref.lock().await;

    // Ignore if in history mode
    if matches!(handle.mode, DocumentMode::History { .. }) {
        return;
    }

    // Re-fetch all blocks at current time
    let all_block_ids = match get_all_block_ids(&doc_id).await {
        Ok(ids) => ids,
        Err(e) => {
            log::error!("[doc_state] Failed to get block IDs during sync: {e}");
            return;
        }
    };
    let document_data =
        match block_getter::get_blocks(doc_id.clone(), all_block_ids, timestamp()).await {
            Ok(data) => data,
            Err(e) => {
                log::error!("[doc_state] Failed to reconstruct blocks during sync: {e}");
                return;
            }
        };

    // Fetch current live locks
    let live_locks = stdb::active_profile()
        .get_cached_live_blocks()
        .await
        .unwrap_or_default();

    // Build desired state
    let desired: Vec<(u64, Block, Option<LockInfo>)> = document_data
        .blocks
        .iter()
        .filter(|bd| !bd.block.deleted.unwrap_or(false))
        .map(|bd| {
            // For our own locked block, preserve mirror content (we are authoritative)
            let block = if handle.locked_block_id == Some(bd.block_id) {
                handle
                    .mirror
                    .get(bd.block_id)
                    .map(|m| m.block.clone())
                    .unwrap_or_else(|| bd.block.clone())
            } else {
                bd.block.clone()
            };

            let lock = live_locks
                .iter()
                .find(|l| {
                    l.block_id == bd.block_id
                        && l.doc_id == doc_id
                        && l.user_id.to_hex().to_string() != handle.my_identity_hex
                })
                .map(|l| LockInfo {
                    user_id: l.user_id.to_hex().to_string(),
                    username: l.username.clone(),
                    state: if l.locked_at.is_some() {
                        LockState::Locked
                    } else {
                        LockState::Focused
                    },
                });

            (bd.block_id, block, lock)
        })
        .collect();

    // Diff and emit
    let updates = handle.mirror.diff(&desired);

    if !updates.is_empty() {
        handle.mirror.apply_updates(&updates);
        emit_block_updates(&doc_id, &updates);

        log::debug!(
            "[doc_state] Sync applied for {}: {} updates",
            doc_id,
            updates.len()
        );
    }
}

// ==========================================
// Lock Management Internals
// ==========================================

/// Reset the inactivity timer. Called on every edit to keep the lock alive.
fn reset_inactivity_timer(handle: &mut OpenDocumentHandle, doc_id: String) {
    // Cancel existing timer
    if let Some(timer) = handle.lock_inactivity_handle.take() {
        timer.abort();
    }

    let block_id = match handle.locked_block_id {
        Some(id) => id,
        None => return,
    };

    // Start new timer
    handle.lock_inactivity_handle = Some(tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(LOCK_INACTIVITY_MS)).await;
        release_lock_by_inactivity(doc_id, block_id).await;
    }));
}

/// Release a lock that timed out due to inactivity.
async fn release_lock_by_inactivity(doc_id: String, block_id: u64) {
    let handle_ref = match OPEN_DOCUMENTS.get(&doc_id) {
        Some(h) => h,
        None => return,
    };
    let mut handle = handle_ref.lock().await;

    // Only release if this is still the active lock
    if handle.locked_block_id != Some(block_id) {
        return;
    }

    cancel_live_update_timers(&mut handle);
    handle.locked_block_id = None;
    handle.lock_inactivity_handle = None;

    // Release on STDB
    let doc_id_clone = doc_id.clone();
    tokio::spawn(async move {
        let _ = stdb::active_profile()
            .unlock_block(doc_id_clone, block_id, true)
            .await;
    });

    log::debug!("[doc_state] Lock released by inactivity for block {block_id}");
}

// ==========================================
// Live Update Throttling
// ==========================================

/// Schedule a live update broadcast for the given block.
///
/// Throttled at `LIVE_UPDATE_THROTTLE_MS`. If a call is already in flight,
/// the update is queued and sent as a follow-up when the current call completes.
fn schedule_live_update(
    handle: &mut OpenDocumentHandle,
    doc_id: String,
    block_id: u64,
    _content: &Block,
) {
    // If we switched blocks, cancel any pending timer for the old block
    if let Some(timer) = handle.live_update_timer.take() {
        if handle.live_update_block_id != Some(block_id) {
            timer.abort();
        }
    }

    // If a call is currently in flight, just flag that we need a follow-up
    if handle.live_update_in_flight {
        handle.live_update_queued = true;
        handle.live_update_block_id = Some(block_id);
        return;
    }

    handle.live_update_block_id = Some(block_id);

    let doc_id_clone = doc_id.clone();
    handle.live_update_timer = Some(tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_millis(LIVE_UPDATE_THROTTLE_MS)).await;
        send_live_update(doc_id_clone, block_id).await;
    }));
}

/// Actually send the live update. Serialized: only one call at a time.
async fn send_live_update(doc_id: String, block_id: u64) {
    let handle_ref = match OPEN_DOCUMENTS.get(&doc_id) {
        Some(h) => h,
        None => return,
    };

    // Get block content and mark as in-flight
    let content = {
        let mut handle = handle_ref.lock().await;
        handle.live_update_timer = None;
        handle.live_update_block_id = None;

        if handle.locked_block_id != Some(block_id) {
            return;
        }

        let content = match handle.mirror.get(block_id) {
            Some(m) => m.block.clone(),
            None => return,
        };

        handle.live_update_in_flight = true;
        content
    };

    // Send (outside the lock to avoid holding it during the network call)
    let result = stdb::active_profile()
        .update_live_block(doc_id.clone(), block_id, &content)
        .await;

    if let Err(e) = result {
        log::error!("[doc_state] Failed to update live block: {e}");
    }

    // Check for queued follow-up
    let handle_ref = match OPEN_DOCUMENTS.get(&doc_id) {
        Some(h) => h,
        None => return,
    };
    let should_follow_up = {
        let mut handle = handle_ref.lock().await;
        handle.live_update_in_flight = false;

        if handle.live_update_queued {
            handle.live_update_queued = false;
            handle.locked_block_id == Some(block_id)
        } else {
            false
        }
    };

    if should_follow_up {
        // No throttle delay — the in-flight wait already served as a natural rate limit
        Box::pin(send_live_update(doc_id, block_id)).await;
    }
}

/// Cancel all live update timers (called when releasing a lock).
fn cancel_live_update_timers(handle: &mut OpenDocumentHandle) {
    if let Some(timer) = handle.live_update_timer.take() {
        timer.abort();
    }
    handle.live_update_block_id = None;
    handle.live_update_queued = false;
}

// ==========================================
// Event Emission
// ==========================================

/// Emit block updates to the frontend via the `block_update_{doc_id}` event.
fn emit_block_updates(doc_id: &str, updates: &[BlockUpdate]) {
    if updates.is_empty() {
        return;
    }

    let event_name = format!("block_update_{doc_id}");
    if let Err(e) = app_handle::emit(&event_name, updates) {
        log::error!("[doc_state] Failed to emit {event_name}: {e}");
    }
}

// ==========================================
// Helpers
// ==========================================

/// Get all unique block IDs for a document from the local database.
async fn get_all_block_ids(doc_id: &str) -> Result<Vec<u64>, String> {
    let batches = database::get_batches_by_doc(doc_id.to_string()).await?;
    let block_ids: HashSet<u64> = batches.iter().map(|b| b.batch_data.block_id).collect();
    Ok(block_ids.into_iter().collect())
}

/// Check if a document is currently open.
pub fn is_document_open(doc_id: &str) -> bool {
    OPEN_DOCUMENTS.contains_key(doc_id)
}
