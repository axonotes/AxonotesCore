use crate::tables::{
    private_document_permission, private_document_version_tag, DocumentVersionTag, Role,
};
use crate::utils::auth::verify_message;
use spacetimedb::{ReducerContext, Table};

/// Create a version tag for a document
/// Only Owner or Editor can create tags
#[spacetimedb::reducer]
pub fn create_version_tag(
    ctx: &ReducerContext,
    tag_id: String,
    doc_id: String,
    encrypted_blob: Vec<u8>, // tag_name, timestamp, created_by, created_at
    signature: Vec<u8>,
) -> Result<(), String> {
    // Check user permission
    let permission = ctx
        .db
        .private_document_permission()
        .by_doc()
        .filter(&doc_id)
        .find(|p| p.user_id == ctx.sender)
        .ok_or("No permission for this document")?;

    // Only Owner or Editor can create tags
    if matches!(permission.role, Role::Reader) {
        return Err("Readers cannot create version tags".to_string());
    }

    // Check if tag already exists
    if ctx
        .db
        .private_document_version_tag()
        .tag_id()
        .find(&tag_id)
        .is_some()
    {
        return Err("Tag ID already exists".to_string());
    }

    // Verify signature (high-order operation)
    let message = [
        b"create_version_tag",
        doc_id.as_bytes(),
        tag_id.as_bytes(),
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

    // Insert version tag
    ctx.db
        .private_document_version_tag()
        .insert(DocumentVersionTag {
            tag_id,
            doc_id,
            encrypted_blob,
        });

    Ok(())
}

/// Delete a version tag
/// Only Owner can delete tags
#[spacetimedb::reducer]
pub fn delete_version_tag(
    ctx: &ReducerContext,
    tag_id: String,
    signature: Vec<u8>,
) -> Result<(), String> {
    // Get the tag
    let tag = ctx
        .db
        .private_document_version_tag()
        .tag_id()
        .find(&tag_id)
        .ok_or("Version tag not found")?;

    // Check if user is owner of the document
    let permission = ctx
        .db
        .private_document_permission()
        .by_doc()
        .filter(&tag.doc_id)
        .find(|p| p.user_id == ctx.sender)
        .ok_or("No permission for this document")?;

    // Only Owner can delete tags
    if !matches!(permission.role, Role::Owner) {
        return Err("Only owner can delete version tags".to_string());
    }

    // Verify signature (high-order operation)
    let message = [b"delete_version_tag", tag_id.as_bytes()].concat();
    let sig_array: &[u8; 64] = signature
        .as_slice()
        .try_into()
        .map_err(|_| "Signature must be 64 bytes")?;

    match verify_message(ctx, &message, sig_array) {
        Ok(true) => {}
        Ok(false) => return Err("Invalid signature".to_string()),
        Err(e) => return Err(format!("Signature verification failed: {}", e)),
    }

    // Delete tag
    ctx.db
        .private_document_version_tag()
        .tag_id()
        .delete(&tag_id);

    Ok(())
}
