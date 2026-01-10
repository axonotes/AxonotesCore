use crate::tables::{
    private_document_permission, private_user_sync_conflict_history, Role, UserSyncConflictHistory,
};
use crate::utils::auth::verify_message;
use spacetimedb::{ReducerContext, Table};

/// Upload sync conflict history
/// Only Owner or Editor can upload (Readers cannot)
#[spacetimedb::reducer]
pub fn upload_conflict_history(
    ctx: &ReducerContext,
    history_id: String,
    doc_id: String,
    encrypted_blob: Vec<u8>,
    timestamp: u128,    // First lost batch timestamp (for ordering)
    key_index: Vec<u8>, // Varint-encoded key index (for decryption)
    signature: Vec<u8>,
) -> Result<(), String> {
    // Validate key_index is a valid varint
    crate::varint::decode(&key_index)?;
    // Check user permission
    let permission = ctx
        .db
        .private_document_permission()
        .by_doc()
        .filter(&doc_id)
        .find(|p| p.user_id == ctx.sender)
        .ok_or("No permission for this document")?;

    // Only Owner or Editor can upload conflict history
    if matches!(permission.role, Role::Reader) {
        return Err("Readers cannot upload conflict history".to_string());
    }

    // Check if history_id already exists
    if ctx
        .db
        .private_user_sync_conflict_history()
        .history_id()
        .find(&history_id)
        .is_some()
    {
        return Err("Conflict history ID already exists".to_string());
    }

    // Verify signature
    // Message format: "upload_conflict_history" | doc_id | history_id | blake3(encrypted_blob)
    let message = [
        b"upload_conflict_history",
        doc_id.as_bytes(),
        history_id.as_bytes(),
        blake3::hash(&encrypted_blob).as_bytes(),
    ]
    .concat();
    let sig_array: &[u8; 64] = signature
        .as_slice()
        .try_into()
        .map_err(|_| "Signature must be 64 bytes")?;

    match verify_message(ctx, &message, sig_array) {
        Ok(true) => {}
        Ok(false) => return Err("Invalid signature".to_string()),
        Err(e) => return Err(format!("Signature verification failed: {}", e)),
    }

    // Insert conflict history
    ctx.db
        .private_user_sync_conflict_history()
        .insert(UserSyncConflictHistory {
            history_id,
            user_id: ctx.sender,
            doc_id,
            encrypted_blob,
            timestamp,
            key_index,
        });

    Ok(())
}
