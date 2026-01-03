use crate::tables::{private_document_metadata, private_document_permission, DocumentMetadata};
use spacetimedb::{ReducerContext, Table};

/// Create document metadata for the current user
/// Called when user gains access to a document
#[spacetimedb::reducer]
pub fn create_document_metadata(
    ctx: &ReducerContext,
    doc_id: String,
    encrypted_blob: Vec<u8>, // path, tags, etc.
) -> Result<(), String> {
    // Check if user has access to document
    let has_access = ctx
        .db
        .private_document_permission()
        .by_doc()
        .filter(&doc_id)
        .any(|p| p.user_id == ctx.sender);

    if !has_access {
        return Err("No access to this document".to_string());
    }

    // Check if metadata already exists
    let already_exists = ctx
        .db
        .private_document_metadata()
        .by_user()
        .filter(&ctx.sender)
        .any(|m| m.doc_id == doc_id);

    if already_exists {
        return Err("Metadata already exists for this document".to_string());
    }

    // Insert metadata
    ctx.db.private_document_metadata().insert(DocumentMetadata {
        meta_id: uuid::Uuid::new_v4().to_string(),
        user_id: ctx.sender,
        doc_id,
        encrypted_blob,
    });

    Ok(())
}

/// Update document metadata (path, tags)
#[spacetimedb::reducer]
pub fn update_document_metadata(
    ctx: &ReducerContext,
    doc_id: String,
    encrypted_blob: Vec<u8>,
) -> Result<(), String> {
    // Find user's metadata for this document
    let metadata = ctx
        .db
        .private_document_metadata()
        .by_user()
        .filter(&ctx.sender)
        .find(|m| m.doc_id == doc_id)
        .ok_or("Metadata not found")?;

    // Update metadata
    ctx.db
        .private_document_metadata()
        .meta_id()
        .update(DocumentMetadata {
            encrypted_blob,
            ..metadata
        });

    Ok(())
}

/// Delete document metadata
#[spacetimedb::reducer]
pub fn delete_document_metadata(ctx: &ReducerContext, doc_id: String) -> Result<(), String> {
    // Find user's metadata for this document
    let metadata = ctx
        .db
        .private_document_metadata()
        .by_user()
        .filter(&ctx.sender)
        .find(|m| m.doc_id == doc_id)
        .ok_or("Metadata not found")?;

    // Delete metadata
    ctx.db
        .private_document_metadata()
        .meta_id()
        .delete(&metadata.meta_id);

    Ok(())
}
