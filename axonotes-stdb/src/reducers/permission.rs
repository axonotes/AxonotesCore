use crate::tables::{
    private_document, private_document_key, private_document_permission, DocumentKey,
    DocumentPermission, Role,
};
use crate::utils::auth::verify_message;
use crate::utils::uuid::generate_uuid;
use crate::{private_document_metadata, Document};
use spacetimedb::{Identity, ReducerContext, SpacetimeType, Table};

#[derive(SpacetimeType)]
pub struct EncryptedKeyEntry {
    pub key_timestamp: u128,
    pub encrypted_data: Vec<u8>,
}

/// Add a user to a document
/// Owner or Editor can add users
/// Editors cannot assign Owner role
#[spacetimedb::reducer]
pub fn add_user_to_document(
    ctx: &ReducerContext,
    doc_id: String,
    new_user_id: Identity,
    role: Role,
    encrypted_keys: Vec<EncryptedKeyEntry>,
    signature: Vec<u8>,
) -> Result<(), String> {
    // Get caller's permission
    let caller_perm = ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == ctx.sender)
        .next()
        .ok_or("No permission for this document")?;

    // Check if caller can add users
    match caller_perm.role {
        Role::Owner => {
            // Owner can do anything
        }
        Role::Editor => {
            // Editors cannot assign Owner role
            if matches!(role, Role::Owner) {
                return Err("Only owner can assign owner role".to_string());
            }
        }
        Role::Reader => {
            return Err("Readers cannot add users".to_string());
        }
    }

    // Check if user already has permission
    if ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == new_user_id)
        .next()
        .is_some()
    {
        return Err("User already has access".to_string());
    }

    // Verify signature (high-order operation)
    let role_byte = match role {
        Role::Owner => 0u8,
        Role::Editor => 1u8,
        Role::Reader => 2u8,
    };

    let mut keys_bytes = Vec::new();
    for key in &encrypted_keys {
        keys_bytes.extend_from_slice(&key.key_timestamp.to_le_bytes());
        keys_bytes.extend_from_slice(&key.encrypted_data);
    }
    let keys_hash = blake3::hash(&keys_bytes);

    let message = [
        b"add_user",
        doc_id.as_bytes(),
        new_user_id.to_byte_array().as_slice(),
        &[role_byte],
        keys_hash.as_bytes(),
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

    // Insert permission
    ctx.db
        .private_document_permission()
        .insert(DocumentPermission {
            permission_id: generate_uuid(ctx).to_string(),
            doc_id: doc_id.clone(),
            user_id: new_user_id,
            role,
        });

    // Insert all historical keys for the new user
    for key in encrypted_keys {
        ctx.db.private_document_key().insert(DocumentKey {
            key_id: generate_uuid(ctx).to_string(),
            doc_id: doc_id.clone(),
            user_id: new_user_id,
            key_timestamp: key.key_timestamp,
            encrypted_data: key.encrypted_data,
        });
    }

    Ok(())
}

/// Remove a user from a document
/// ALWAYS triggers key rotation
/// Owner/Editor can remove users (except Owner)
#[spacetimedb::reducer]
pub fn remove_user_from_document(
    ctx: &ReducerContext,
    doc_id: String,
    removed_user_id: Identity,
    signature: Vec<u8>,
) -> Result<(), String> {
    // Get caller's permission
    let caller_perm = ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == ctx.sender)
        .next()
        .ok_or("No permission for this document")?;

    // Get target's permission
    let target_perm = ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == removed_user_id)
        .next()
        .ok_or("User not found in document")?;

    // Check permissions
    match caller_perm.role {
        Role::Owner => {
            // Owner cannot remove themselves
            if ctx.sender == removed_user_id {
                return Err("Owner cannot remove themselves, use transfer_ownership".to_string());
            }
        }
        Role::Editor => {
            // Editor cannot remove owner
            if matches!(target_perm.role, Role::Owner) {
                return Err("Editor cannot remove owner".to_string());
            }
        }
        Role::Reader => {
            return Err("Readers cannot remove users".to_string());
        }
    }

    // Verify signature
    let message = [
        b"remove_user",
        doc_id.as_bytes(),
        removed_user_id.to_byte_array().as_slice(),
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

    // Delete permission
    ctx.db
        .private_document_permission()
        .permission_id()
        .delete(&target_perm.permission_id);

    // Delete ALL keys for removed user (all timestamps)
    for key in ctx
        .db
        .private_document_key()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == removed_user_id)
    {
        ctx.db.private_document_key().key_id().delete(&key.key_id);
    }

    // Delete their metadata
    for meta in ctx
        .db
        .private_document_metadata()
        .by_user()
        .filter(&removed_user_id)
    {
        if meta.doc_id == doc_id {
            ctx.db
                .private_document_metadata()
                .meta_id()
                .delete(&meta.meta_id);
        }
    }

    // NOTE: Key rotation happens AFTER this via rotate_document_keys reducer
    // Client must call rotate_document_keys with new keys + snapshot batches

    Ok(())
}

/// Change a user's role (Editor ↔ Reader)
/// NO key rotation needed
#[spacetimedb::reducer]
pub fn change_user_role(
    ctx: &ReducerContext,
    doc_id: String,
    target_user_id: Identity,
    new_role: Role,
    signature: Vec<u8>,
) -> Result<(), String> {
    // Get caller's permission
    let caller_perm = ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == ctx.sender)
        .next()
        .ok_or("No permission for this document")?;

    // Get target's permission
    let target_perm = ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == target_user_id)
        .next()
        .ok_or("User not found in document")?;

    // Cannot change owner role (use transfer_ownership)
    if matches!(target_perm.role, Role::Owner) {
        return Err("Cannot change owner role, use transfer_ownership".to_string());
    }

    // Cannot promote to owner
    if matches!(new_role, Role::Owner) {
        return Err("Cannot promote to owner, use transfer_ownership".to_string());
    }

    // Check permissions
    match caller_perm.role {
        Role::Owner => {
            // Owner can change anyone
        }
        Role::Editor => {
            // Editor can only change Editor ↔ Reader
        }
        Role::Reader => {
            return Err("Readers cannot change roles".to_string());
        }
    }

    // Verify signature
    let role_byte = match new_role {
        Role::Owner => 0u8,
        Role::Editor => 1u8,
        Role::Reader => 2u8,
    };

    let message = [
        b"change_role",
        doc_id.as_bytes(),
        target_user_id.to_byte_array().as_slice(),
        &[role_byte],
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

    // Update role
    ctx.db
        .private_document_permission()
        .permission_id()
        .update(DocumentPermission {
            role: new_role,
            ..target_perm
        });

    Ok(())
}

/// Transfer ownership to another user
/// Old owner becomes Editor
#[spacetimedb::reducer]
pub fn transfer_ownership(
    ctx: &ReducerContext,
    doc_id: String,
    new_owner_id: Identity,
    signature: Vec<u8>,
) -> Result<(), String> {
    // Get document
    let doc = ctx
        .db
        .private_document()
        .doc_id()
        .find(&doc_id)
        .ok_or("Document not found")?;

    // Only current owner can transfer
    if doc.owner_id != ctx.sender {
        return Err("Only owner can transfer ownership".to_string());
    }

    // Get new owner's permission
    let new_owner_perm = ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == new_owner_id)
        .next()
        .ok_or("New owner must have access to document first")?;

    // Get old owner's permission
    let old_owner_perm = ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == ctx.sender)
        .next()
        .ok_or("Old owner permission not found")?;

    // Verify signature
    let message = [
        b"transfer_ownership",
        doc_id.as_bytes(),
        new_owner_id.to_byte_array().as_slice(),
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

    // Update document owner
    ctx.db.private_document().doc_id().update(Document {
        owner_id: new_owner_id,
        ..doc
    });

    // Promote new owner
    ctx.db
        .private_document_permission()
        .permission_id()
        .update(DocumentPermission {
            role: Role::Owner,
            ..new_owner_perm
        });

    // Demote old owner to Editor
    ctx.db
        .private_document_permission()
        .permission_id()
        .update(DocumentPermission {
            role: Role::Editor,
            ..old_owner_perm
        });

    Ok(())
}
