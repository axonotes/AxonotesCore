use crate::tables::{
    private_document, private_document_batch, private_document_key, private_document_permission,
    private_document_version_tag, Document, DocumentBatch, DocumentKey, DocumentVersionTag,
};
use crate::utils::auth::verify_message;
use crate::utils::uuid::generate_uuid;
use crate::Role;
use spacetimedb::{Identity, ReducerContext, SpacetimeType, Table};

/// Snapshot batch (delta from empty)
#[derive(SpacetimeType, Clone)]
pub struct SnapshotBatch {
    pub batch_id: String,
    pub timestamp: u128,
    pub encrypted_data: Vec<u8>,
}

/// Re-encrypted version tag
#[derive(SpacetimeType, Clone)]
pub struct ReEncryptedTag {
    pub tag_id: String,
    pub encrypted_blob: Vec<u8>,
}

/// User with their encrypted keys (for key rotation)
#[derive(SpacetimeType, Clone)]
pub struct UserKeyEntry {
    pub user_id: Identity,
    pub encrypted_key_data: Vec<u8>,
}

/// Rotate document keys (called after user removal)
///
/// This is a CRITICAL atomic operation that:
/// 1. Updates document's current signing key
/// 2. Deletes old keys for revoked users (already done in remove_user)
/// 3. Inserts new keys for all remaining users
/// 4. Stores snapshot batches for ALL blocks
/// 5. Re-encrypts all version tags with new key
#[spacetimedb::reducer]
pub fn rotate_document_keys(
    ctx: &ReducerContext,
    doc_id: String,
    new_key_timestamp: u128,
    new_public_signing_key: Vec<u8>,
    user_keys: Vec<UserKeyEntry>,
    snapshot_batches: Vec<SnapshotBatch>,
    re_encrypted_tags: Vec<ReEncryptedTag>,
    signature: Vec<u8>,
) -> Result<(), String> {
    // Get document
    let doc = ctx
        .db
        .private_document()
        .doc_id()
        .find(&doc_id)
        .ok_or("Document not found")?;

    // Get caller's permission
    let caller_perm = ctx
        .db
        .private_document_permission()
        .by_doc_and_user()
        .filter(&doc_id)
        .filter(|p| p.user_id == ctx.sender)
        .next()
        .ok_or("No permission for this document")?;

    // Check permissions
    match caller_perm.role {
        Role::Owner => {
            // Owner can rotate
        }
        Role::Editor => {
            // Editor can rotate
        }
        Role::Reader => {
            return Err("Readers cannot rotate keys".to_string());
        }
    }

    // Verify signature
    let mut user_keys_bytes = Vec::new();
    for uk in &user_keys {
        user_keys_bytes.extend_from_slice(uk.user_id.to_byte_array().as_slice());
        user_keys_bytes.extend_from_slice(&uk.encrypted_key_data);
    }

    let mut snapshots_bytes = Vec::new();
    for snap in &snapshot_batches {
        snapshots_bytes.extend_from_slice(snap.batch_id.as_bytes());
        snapshots_bytes.extend_from_slice(&snap.timestamp.to_le_bytes());
        snapshots_bytes.extend_from_slice(&snap.encrypted_data);
    }

    let mut tags_bytes = Vec::new();
    for tag in &re_encrypted_tags {
        tags_bytes.extend_from_slice(tag.tag_id.as_bytes());
        tags_bytes.extend_from_slice(&tag.encrypted_blob);
    }

    let message = [
        b"rotate_keys",
        doc_id.as_bytes(),
        &new_key_timestamp.to_le_bytes(),
        &new_public_signing_key,
        blake3::hash(&user_keys_bytes).as_bytes(),
        blake3::hash(&snapshots_bytes).as_bytes(),
        blake3::hash(&tags_bytes).as_bytes(),
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

    // Update document's current key
    ctx.db.private_document().doc_id().update(Document {
        current_public_signing_key: new_public_signing_key,
        key_timestamp: new_key_timestamp,
        ..doc
    });

    // Insert new keys for all authorized users
    for user_key in user_keys {
        ctx.db.private_document_key().insert(DocumentKey {
            key_id: generate_uuid(ctx).to_string(),
            doc_id: doc_id.clone(),
            user_id: user_key.user_id,
            key_timestamp: new_key_timestamp,
            encrypted_data: user_key.encrypted_key_data,
        });
    }

    // Store ALL snapshot batches
    for snapshot in snapshot_batches {
        ctx.db.private_document_batch().insert(DocumentBatch {
            batch_id: snapshot.batch_id,
            doc_id: doc_id.clone(),
            timestamp: snapshot.timestamp,
            encrypted_data: snapshot.encrypted_data,
        });
    }

    // Re-encrypt all version tags with new key
    for tag_update in re_encrypted_tags {
        if let Some(tag) = ctx
            .db
            .private_document_version_tag()
            .tag_id()
            .find(&tag_update.tag_id)
        {
            ctx.db
                .private_document_version_tag()
                .tag_id()
                .update(DocumentVersionTag {
                    encrypted_blob: tag_update.encrypted_blob,
                    ..tag
                });
        }
    }

    Ok(())
}
