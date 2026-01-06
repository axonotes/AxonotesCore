use crate::tables::{
    private_document_permission, private_pending_share, private_share_request, private_user,
    PendingShare, Role, ShareRequest,
};
use crate::utils::auth::verify_message;
use crate::utils::share_code::generate_share_code;
use crate::utils::uuid::generate_uuid;
use spacetimedb::{ReducerContext, Table, Timestamp};

/// Share code expiration time in milliseconds (5 minutes)
const SHARE_EXPIRATION_MS: u128 = 5 * 60 * 1000;

/// Helper to get current timestamp in milliseconds
fn get_timestamp_ms(ctx: &ReducerContext) -> u128 {
    ctx.timestamp
        .duration_since(Timestamp::UNIX_EPOCH)
        .expect("timestamp should exist")
        .as_millis()
}

/// Helper to check if a share is expired
fn is_share_expired(share: &PendingShare, now: u128) -> bool {
    now > share.created_at + SHARE_EXPIRATION_MS
}

/// Helper to delete a share and all related requests
fn cleanup_share(ctx: &ReducerContext, share_code: &str) {
    // Delete all share requests for this code
    for request in ctx
        .db
        .private_share_request()
        .by_share_code()
        .filter(share_code)
    {
        ctx.db
            .private_share_request()
            .request_id()
            .delete(&request.request_id);
    }

    // Delete the pending share
    ctx.db
        .private_pending_share()
        .share_code()
        .delete(&share_code.to_string());
}

/// Create a pending share for a document
/// Only Owner or Editor can create shares
#[spacetimedb::reducer]
pub fn create_pending_share(
    ctx: &ReducerContext,
    doc_id: String,
    signature: Vec<u8>,
) -> Result<(), String> {
    // Get caller's permission
    let caller_perm = ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .find(|p| p.user_id == ctx.sender)
        .ok_or("No permission for this document")?;

    // Check if caller can share (Owner or Editor)
    match caller_perm.role {
        Role::Owner | Role::Editor => {}
        Role::Reader => {
            return Err("Readers cannot share documents".to_string());
        }
    }

    // Verify signature
    let message = [b"create_pending_share", doc_id.as_bytes()].concat();
    let sig_array: &[u8; 64] = signature
        .as_slice()
        .try_into()
        .map_err(|_| "Signature must be 64 bytes")?;

    match verify_message(ctx, &message, sig_array) {
        Ok(true) => {}
        Ok(false) => return Err("Invalid signature".to_string()),
        Err(e) => return Err(format!("Signature verification failed: {}", e)),
    }

    // Generate share code
    let share_code = generate_share_code(ctx);

    // Check for collision (extremely unlikely but let's be safe)
    if ctx
        .db
        .private_pending_share()
        .share_code()
        .find(&share_code)
        .is_some()
    {
        return Err("Share code collision, please try again".to_string());
    }

    let now = get_timestamp_ms(ctx);

    // Insert pending share
    ctx.db.private_pending_share().insert(PendingShare {
        share_code,
        doc_id,
        creator_id: ctx.sender,
        created_at: now,
    });

    Ok(())
}

/// Join a pending share
/// Fetches joiner's public_encryption_key from User table
#[spacetimedb::reducer]
pub fn join_pending_share(ctx: &ReducerContext, share_code: String) -> Result<(), String> {
    // Auto-capitalize the share code
    let share_code = share_code.to_uppercase();

    // Look up the pending share
    let share = ctx
        .db
        .private_pending_share()
        .share_code()
        .find(&share_code);

    let now = get_timestamp_ms(ctx);

    // Check if share exists and not expired
    let share = match share {
        Some(s) => {
            if is_share_expired(&s, now) {
                // Lazy cleanup of expired share
                cleanup_share(ctx, &share_code);
                return Err("Invalid or expired share code".to_string());
            }
            s
        }
        None => {
            return Err("Invalid or expired share code".to_string());
        }
    };

    // Cannot join your own share
    if share.creator_id == ctx.sender {
        return Err("Cannot join your own share".to_string());
    }

    // Check if user already joined this share
    let already_joined = ctx
        .db
        .private_share_request()
        .by_share_code()
        .filter(&share_code)
        .any(|r| r.user_id == ctx.sender);

    if already_joined {
        return Err("Already joined this share".to_string());
    }

    // Get joiner's public_encryption_key from User table
    let user = ctx
        .db
        .private_user()
        .identity()
        .find(ctx.sender)
        .ok_or("User not found")?;

    // Insert share request
    ctx.db.private_share_request().insert(ShareRequest {
        request_id: generate_uuid(ctx).to_string(),
        share_code,
        user_id: ctx.sender,
        public_encryption_key: user.public_encryption_key,
    });

    Ok(())
}

/// Close a pending share (cleanup after done or abandon)
/// Only the creator can close the share
#[spacetimedb::reducer]
pub fn close_pending_share(
    ctx: &ReducerContext,
    share_code: String,
    signature: Vec<u8>,
) -> Result<(), String> {
    // Auto-capitalize the share code
    let share_code = share_code.to_uppercase();

    // Look up the pending share
    let share = ctx
        .db
        .private_pending_share()
        .share_code()
        .find(&share_code)
        .ok_or("Share not found")?;

    // Only creator can close
    if share.creator_id != ctx.sender {
        return Err("Only the creator can close the share".to_string());
    }

    // Verify signature
    let message = [b"close_pending_share", share_code.as_bytes()].concat();
    let sig_array: &[u8; 64] = signature
        .as_slice()
        .try_into()
        .map_err(|_| "Signature must be 64 bytes")?;

    match verify_message(ctx, &message, sig_array) {
        Ok(true) => {}
        Ok(false) => return Err("Invalid signature".to_string()),
        Err(e) => return Err(format!("Signature verification failed: {}", e)),
    }

    // Cleanup share and all related requests
    cleanup_share(ctx, &share_code);

    Ok(())
}

/// Leave a pending share (joiner withdraws)
#[spacetimedb::reducer]
pub fn leave_pending_share(ctx: &ReducerContext, share_code: String) -> Result<(), String> {
    // Auto-capitalize the share code
    let share_code = share_code.to_uppercase();

    // Find the share request for this user
    let request = ctx
        .db
        .private_share_request()
        .by_share_code()
        .filter(&share_code)
        .find(|r| r.user_id == ctx.sender)
        .ok_or("No share request found")?;

    // Delete the request
    ctx.db
        .private_share_request()
        .request_id()
        .delete(&request.request_id);

    Ok(())
}
