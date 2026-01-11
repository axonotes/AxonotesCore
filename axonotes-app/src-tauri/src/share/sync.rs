//! # Share Synchronization
//!
//! Manages active share sessions and processes incoming join requests.
//!
//! ## Share Flow
//!
//! 1. **Create Share**: `register_share_session` stores session locally
//! 2. **Receive Code**: Subscription receives share code from server
//! 3. **Code Ready**: `emit_share_code_ready` notifies frontend
//! 4. **Joiner Arrives**: Subscription receives join request
//! 5. **Auto-Accept**: `process_share_joiner` encrypts keys and adds user
//!
//! ## State Management
//!
//! - Sessions keyed by `doc_id` (share code unknown until server generates it)
//! - Tracks processed requests to prevent duplicate processing
//! - Share code stored when received from server subscription

use super::processor::process_share_joiner;
use super::types::ActiveShareSession;
use crate::events::{emit_share_code_ready, emit_share_error, emit_share_user_added};
use crate::stdb_bindings::{
    DbConnection, MyPendingSharesTableAccess, PendingShare, PendingShareRequestsTableAccess, Role,
    ShareRequest,
};
use once_cell::sync::OnceCell;
use spacetimedb_sdk::{DbContext, Table};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

// ==========================================
// State Management
// ==========================================

/// Map of active share sessions, keyed by document ID.
/// We use doc_id as the key because share_code is unknown until the server generates it.
type ShareSessionMap = Arc<Mutex<HashMap<String, ActiveShareSession>>>;

/// Global storage for active share sessions.
static ACTIVE_SHARE_SESSIONS: OnceCell<ShareSessionMap> = OnceCell::new();

/// Returns a thread-safe reference to the active sessions map.
fn get_active_sessions() -> ShareSessionMap {
    ACTIVE_SHARE_SESSIONS
        .get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
        .clone()
}

// ==========================================
// Public API
// ==========================================

/// Registers a new share session before calling create_pending_share.
///
/// The share code will be discovered via subscription when the server generates it.
///
/// # Arguments
///
/// * `doc_id` - Document being shared
/// * `role` - Role to grant to joiners
/// * `full_history` - Whether joiners get full history access
pub async fn register_share_session(doc_id: String, role: Role, full_history: bool) {
    let sessions = get_active_sessions();
    let mut sessions = sessions.lock().await;
    sessions.insert(
        doc_id.clone(),
        ActiveShareSession::new(doc_id, role, full_history),
    );
}

/// Unregisters a share session when sharing is closed.
pub async fn unregister_share_session(doc_id: &str) {
    let sessions = get_active_sessions();
    let mut sessions = sessions.lock().await;
    sessions.remove(doc_id);
}

/// Checks if a share session exists for a document.
#[allow(dead_code)]
pub async fn has_active_session_for_doc(doc_id: &str) -> bool {
    let sessions = get_active_sessions();
    let sessions = sessions.lock().await;
    sessions.contains_key(doc_id)
}

/// Gets the share code for a document if one has been received from server.
pub async fn get_share_code_for_doc(doc_id: &str) -> Option<String> {
    let sessions = get_active_sessions();
    let sessions = sessions.lock().await;
    sessions.get(doc_id).and_then(|s| s.share_code.clone())
}

// ==========================================
// Subscription Setup
// ==========================================

/// Sets up SpacetimeDB subscriptions for share synchronization.
///
/// Subscribes to:
/// - `my_pending_shares`: Shares created by this user (to get share codes)
/// - `pending_share_requests`: Join requests to auto-accept
///
/// Uses `on_insert` callbacks for real-time updates in addition to `on_applied`
/// for initial data load.
///
/// This should be called after the main connection is established.
pub fn setup_share_sync(conn: &DbConnection) -> Result<(), String> {
    // Register on_insert for my_pending_shares (real-time share code discovery)
    conn.db.my_pending_shares().on_insert(|_ctx, share| {
        let share = share.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                if let Err(e) = process_share_updates(vec![share], vec![]).await {
                    eprintln!("Share sync error (on_insert pending_share): {e}");
                }
            });
        });
    });

    // Register on_insert for pending_share_requests (real-time join request processing)
    conn.db.pending_share_requests().on_insert(|ctx, request| {
        // Get current pending shares to map share_code -> doc_id
        let pending_shares: Vec<PendingShare> = ctx.db.my_pending_shares().iter().collect();
        let request = request.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                if let Err(e) = process_share_updates(pending_shares, vec![request]).await {
                    eprintln!("Share sync error (on_insert share_request): {e}");
                }
            });
        });
    });

    // Subscribe and process initial data load
    conn.subscription_builder()
        .on_applied(|ctx| {
            // Get all pending shares and requests
            let pending_shares: Vec<PendingShare> = ctx.db.my_pending_shares().iter().collect();
            let share_requests: Vec<ShareRequest> =
                ctx.db.pending_share_requests().iter().collect();

            // This callback runs on SpacetimeDB's background thread, NOT the Tokio runtime.
            // Use std::thread::spawn with a blocking runtime to run async code.
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                    if let Err(e) = process_share_updates(pending_shares, share_requests).await {
                        eprintln!("Share sync error: {e}");
                    }
                });
            });
        })
        .on_error(|_ctx, error| {
            eprintln!("Share subscription error: {error:?}");
        })
        .subscribe([
            "SELECT * FROM my_pending_shares",
            "SELECT * FROM pending_share_requests",
        ]);

    Ok(())
}

// ==========================================
// Processing Logic
// ==========================================

/// Processes incoming share updates from SpacetimeDB subscription.
///
/// - Discovers share codes for pending shares
/// - Auto-accepts join requests by encrypting keys for joiners
async fn process_share_updates(
    pending_shares: Vec<PendingShare>,
    share_requests: Vec<ShareRequest>,
) -> Result<(), String> {
    let sessions = get_active_sessions();
    let mut sessions = sessions.lock().await;

    // Process pending shares - discover share codes and emit events
    for share in &pending_shares {
        if let Some(session) = sessions.get_mut(&share.doc_id) {
            // Store the share code if we don't have it yet
            let is_new_code = session.share_code.is_none();
            if is_new_code {
                session.share_code = Some(share.share_code.clone());
                // Emit event that share code is ready
                emit_share_code_ready(session.doc_id.clone(), share.share_code.clone());
            }
        }
    }

    // Build a lookup from share_code -> doc_id for processing requests
    let share_code_to_doc: HashMap<String, String> = pending_shares
        .iter()
        .map(|s| (s.share_code.clone(), s.doc_id.clone()))
        .collect();

    // Drop the sessions lock before async processing to avoid holding it
    // across await points
    let requests_to_process: Vec<_> = share_requests
        .into_iter()
        .filter_map(|request| {
            let doc_id = share_code_to_doc.get(&request.share_code)?;
            let session = sessions.get_mut(doc_id)?;

            // Skip if already processed
            if session.processed_requests.contains(&request.request_id) {
                return None;
            }

            // Mark as processing
            session
                .processed_requests
                .insert(request.request_id.clone());

            Some((
                session.doc_id.clone(),
                session.role,
                session.full_history,
                request.user_id,
                request.public_encryption_key.clone(),
            ))
        })
        .collect();

    // Drop the lock before processing
    drop(sessions);

    // Process share requests (auto-accept joiners) - await directly instead of spawning
    for (doc_id, role, full_history, joiner_id, joiner_public_key) in requests_to_process {
        match process_share_joiner(
            doc_id.clone(),
            role,
            full_history,
            joiner_id,
            joiner_public_key,
        )
        .await
        {
            Ok(_) => {
                emit_share_user_added(doc_id, joiner_id.to_hex().to_string());
            }
            Err(e) => {
                eprintln!("Failed to process share joiner: {e}");
                emit_share_error(doc_id, e);
            }
        }
    }

    Ok(())
}
