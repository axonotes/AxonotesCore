use crate::tables::{
    private_document, private_document_batch, private_document_permission, DocumentBatch, Role,
};
use spacetimedb::{ReducerContext, Table};

/// Upload a batch of patches for a document
///
/// Batches are signed with the document's signing key (not user key)
/// Server verifies signature proves user has current keys
#[spacetimedb::reducer]
pub fn upload_batch(
    ctx: &ReducerContext,
    batch_id: String,
    doc_id: String,
    timestamp: u128,
    encrypted_data: Vec<u8>, // Contains block_id + patches (encrypted & signed)
    signature: Vec<u8>,      // Ed25519 signature (verified then discarded)
) -> Result<(), String> {
    // Check if batch already exists
    if ctx
        .db
        .private_document_batch()
        .batch_id()
        .find(&batch_id)
        .is_some()
    {
        return Err("Batch already exists".to_string());
    }

    // Get document
    let doc = ctx
        .db
        .private_document()
        .doc_id()
        .find(&doc_id)
        .ok_or("Document not found")?;

    // Check user permission
    let permission = ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == ctx.sender)
        .next()
        .ok_or("No permission for this document")?;

    // Readers cannot upload batches
    if matches!(permission.role, Role::Reader) {
        return Err("Readers cannot edit".to_string());
    }

    // Verify signature with document's current public signing key
    // This proves the user has the current document keys
    let sig_array: &[u8; 64] = signature
        .as_slice()
        .try_into()
        .map_err(|_| "Signature must be 64 bytes")?;

    let pub_key_array: &[u8; 32] = doc
        .current_public_signing_key
        .as_slice()
        .try_into()
        .map_err(|_| "Public signing key must be 32 bytes")?;

    match crate::crypto::ed25519::verify_signature(pub_key_array, &encrypted_data, sig_array) {
        Ok(true) => {}
        Ok(false) => return Err("Invalid batch signature - wrong keys or no access".to_string()),
        Err(e) => return Err(format!("Signature verification failed: {}", e)),
    }

    // Insert batch (signature NOT stored, only verified)
    ctx.db.private_document_batch().insert(DocumentBatch {
        batch_id,
        doc_id,
        timestamp,
        encrypted_data,
    });

    Ok(())
}
