use crate::tables::{
    private_document, private_document_key, private_document_metadata, private_document_permission,
    Document, DocumentKey, DocumentMetadata, DocumentPermission, Role,
};
use crate::utils::auth::verify_message;
use crate::utils::uuid::generate_uuid;
use crate::{private_document_batch, private_document_version_tag, private_live_block};
use spacetimedb::{ReducerContext, Table};

/// Creates a new document owned by the caller
///
/// Client generates:
/// - doc_id (UUID)
/// - document encryption key (ChaCha20)
/// - document signing keypair (Ed25519)
/// - encrypts keys with their own public_encryption_key
/// - default metadata with path "/Untitled.doc"
#[spacetimedb::reducer]
pub fn create_document(
    ctx: &ReducerContext,
    doc_id: String,
    new_public_signing_key: Vec<u8>,
    key_timestamp: u128,
    encrypted_key_data: Vec<u8>, // DocumentKeyData encrypted for owner
    encrypted_metadata_blob: Vec<u8>, // Default path "/Untitled.doc"
) -> Result<(), String> {
    // Check if document already exists
    if ctx.db.private_document().doc_id().find(&doc_id).is_some() {
        return Err("Document already exists".to_string());
    }

    // Insert document
    ctx.db.private_document().insert(Document {
        doc_id: doc_id.clone(),
        owner_id: ctx.sender,
        current_public_signing_key: new_public_signing_key,
        key_timestamp,
    });

    // Insert owner's document keys
    ctx.db.private_document_key().insert(DocumentKey {
        key_id: generate_uuid(ctx).to_string(),
        doc_id: doc_id.clone(),
        user_id: ctx.sender,
        key_timestamp,
        encrypted_data: encrypted_key_data,
    });

    // Insert owner permission
    ctx.db
        .private_document_permission()
        .insert(DocumentPermission {
            permission_id: generate_uuid(ctx).to_string(),
            doc_id: doc_id.clone(),
            user_id: ctx.sender,
            role: Role::Owner,
        });

    // Insert default metadata
    ctx.db.private_document_metadata().insert(DocumentMetadata {
        meta_id: generate_uuid(ctx).to_string(),
        user_id: ctx.sender,
        doc_id,
        encrypted_blob: encrypted_metadata_blob,
    });

    Ok(())
}

/// Deletes a document and all related data
/// Only the owner can delete a document
#[spacetimedb::reducer]
pub fn delete_document(
    ctx: &ReducerContext,
    doc_id: String,
    signature: Vec<u8>,
) -> Result<(), String> {
    // Get document
    let doc = ctx
        .db
        .private_document()
        .doc_id()
        .find(&doc_id)
        .ok_or("Document not found")?;

    // Check if caller is owner
    if doc.owner_id != ctx.sender {
        return Err("Only owner can delete document".to_string());
    }

    // Verify signature (high-order operation)
    let message = [b"delete_document", doc_id.as_bytes()].concat();
    let sig_array: &[u8; 64] = signature
        .as_slice()
        .try_into()
        .map_err(|_| "Signature must be 64 bytes")?;

    match verify_message(ctx, message.as_slice(), sig_array) {
        Ok(true) => {}
        Ok(false) => return Err("Invalid signature".to_string()),
        Err(e) => return Err(format!("Signature verification failed: {}", e)),
    }

    // Delete document (this will cascade to batches via manual deletion)
    ctx.db.private_document().doc_id().delete(&doc_id);

    // Delete all permissions
    for perm in ctx
        .db
        .private_document_permission()
        .by_doc()
        .filter(&doc_id)
    {
        ctx.db
            .private_document_permission()
            .permission_id()
            .delete(&perm.permission_id);
    }

    // Delete all keys
    for key in ctx
        .db
        .private_document_key()
        .by_doc_and_user()
        .filter(&doc_id)
    {
        ctx.db.private_document_key().key_id().delete(&key.key_id);
    }

    // Delete all metadata
    for meta in ctx
        .db
        .private_document_metadata()
        .by_user()
        .filter(&ctx.sender)
    {
        if meta.doc_id == doc_id {
            ctx.db
                .private_document_metadata()
                .meta_id()
                .delete(&meta.meta_id);
        }
    }

    // Delete all version tags
    for tag in ctx
        .db
        .private_document_version_tag()
        .by_doc()
        .filter(&doc_id)
    {
        ctx.db
            .private_document_version_tag()
            .tag_id()
            .delete(&tag.tag_id);
    }

    // Delete all batches
    for batch in ctx
        .db
        .private_document_batch()
        .by_doc_and_timestamp()
        .filter(&doc_id)
    {
        ctx.db
            .private_document_batch()
            .batch_id()
            .delete(&batch.batch_id);
    }

    // Delete all live blocks
    for live_block in ctx.db.private_live_block().by_doc().filter(&doc_id) {
        ctx.db
            .private_live_block()
            .live_block_id()
            .delete(&live_block.live_block_id);
    }

    Ok(())
}
