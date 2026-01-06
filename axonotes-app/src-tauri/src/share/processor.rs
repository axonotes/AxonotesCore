use super::key_rotation::{encrypt_key_for_user, rotate_keys_for_share};
use crate::crypto::ed25519::sign_message;
use crate::database::get_active_user_keys;
use crate::database::keys::Keys;
use crate::encryption::document::DecryptedDocumentKey;
use crate::events::emit_collaborator_added;
use crate::stdb;
use crate::stdb_bindings::{EncryptedKeyEntry, Role};
use crate::utils::vec_array::ByteArrayConversion;
use spacetimedb_sdk::Identity;

/// Process a joiner by encrypting keys and adding them to the document
///
/// This is the core logic that runs when a user joins a share session.
///
/// For `full_history=true`:
/// - Encrypts ALL existing document keys for the joiner
/// - Joiner can decrypt all historical content
///
/// For `full_history=false`:
/// - First rotates document keys (creates snapshots, invalidates old keys)
/// - Only gives the joiner the NEW key
/// - Joiner can only see content from the time they joined
pub async fn process_share_joiner(
    doc_id: String,
    role: Role,
    full_history: bool,
    joiner_id: Identity,
    joiner_public_key: Vec<u8>,
) -> Result<(), String> {
    // Get user keys
    let user_keys: Keys = get_active_user_keys().await?.ok_or("No active user keys")?;

    let private_encryption_key = user_keys.private_encryption_key.as_array()?;
    let public_encryption_key = user_keys.public_encryption_key.as_array()?;
    let private_signing_key = user_keys.private_signing_key.as_array()?;

    // Convert joiner's public key to array
    let joiner_public_key_array: &[u8; 32] = joiner_public_key
        .as_slice()
        .try_into()
        .map_err(|_| "Joiner public key must be 32 bytes")?;

    // Prepare encrypted keys for joiner (role determines if signing key is included)
    let encrypted_keys: Vec<EncryptedKeyEntry> = if full_history {
        // Full history: Encrypt ALL existing keys for the joiner
        process_full_history_share(
            &doc_id,
            role,
            private_encryption_key,
            public_encryption_key,
            joiner_public_key_array,
        )
        .await?
    } else {
        // No history: Rotate keys first, then give only the new key
        process_no_history_share(
            &doc_id,
            role,
            private_encryption_key,
            public_encryption_key,
            joiner_public_key_array,
        )
        .await?
    };

    // Convert role to byte for signature
    let role_byte: u8 = match role {
        Role::Owner => 0,
        Role::Editor => 1,
        Role::Reader => 2,
    };

    // Hash encrypted keys for signature (same as server)
    let mut keys_bytes = Vec::new();
    for key in &encrypted_keys {
        keys_bytes.extend_from_slice(&key.key_timestamp.to_le_bytes());
        keys_bytes.extend_from_slice(&key.encrypted_data);
    }
    let keys_hash = blake3::hash(&keys_bytes);

    // Sign the add_user message (must match server format)
    let message = [
        b"add_user".as_slice(),
        doc_id.as_bytes(),
        joiner_id.to_byte_array().as_slice(),
        &[role_byte],
        keys_hash.as_bytes(),
    ]
    .concat();

    let signature = sign_message(private_signing_key, &message)
        .map_err(|e| format!("Failed to sign message: {}", e))?;

    // Call add_user_to_document
    stdb::active_profile()
        .add_user_to_document(
            doc_id.clone(),
            joiner_id,
            role,
            encrypted_keys,
            signature.to_vec(),
        )
        .await?;

    // Emit collaborator added event
    let role_str = match role {
        Role::Owner => "owner",
        Role::Editor => "editor",
        Role::Reader => "reader",
    };
    emit_collaborator_added(doc_id, joiner_id.to_hex().to_string(), role_str.to_string());

    Ok(())
}

/// Process a share with full history access
/// Encrypts all existing document keys for the joiner
async fn process_full_history_share(
    doc_id: &str,
    role: Role,
    my_private_key: &[u8; 32],
    my_public_key: &[u8; 32],
    joiner_public_key: &[u8; 32],
) -> Result<Vec<EncryptedKeyEntry>, String> {
    // Get document keys
    let document_keys = stdb::active_profile().get_cached_document_keys().await?;

    // Filter keys for this document
    let doc_keys: Vec<&DecryptedDocumentKey> = document_keys
        .iter()
        .filter(|k| k.doc_id == doc_id)
        .collect();

    if doc_keys.is_empty() {
        return Err(format!("No keys found for document {}", doc_id));
    }

    // Encrypt ALL keys for the joiner (role determines if signing key is included)
    encrypt_keys_for_user(
        &doc_keys,
        role,
        my_private_key,
        my_public_key,
        joiner_public_key,
    )
}

/// Process a share without history access
/// Rotates keys first (creating snapshots), then gives only the new key
async fn process_no_history_share(
    doc_id: &str,
    role: Role,
    my_private_key: &[u8; 32],
    my_public_key: &[u8; 32],
    joiner_public_key: &[u8; 32],
) -> Result<Vec<EncryptedKeyEntry>, String> {
    // Rotate document keys (this creates snapshots and re-encrypts for existing users)
    let rotation_result = rotate_keys_for_share(doc_id).await?;

    // Encrypt only the NEW key for the joiner (role determines if signing key is included)
    let encrypted_key = encrypt_key_for_user(
        &rotation_result.new_key_data,
        rotation_result.new_key_timestamp,
        role,
        my_private_key,
        my_public_key,
        joiner_public_key,
    )?;

    Ok(vec![encrypted_key])
}

/// Encrypt document keys for a specific user based on their role
///
/// Takes a slice of document keys and encrypts them using X25519
/// key exchange so only the target user can decrypt them.
/// - Readers get empty signing_private_key (can only decrypt, not sign)
/// - Editors/Owners get full signing_private_key
fn encrypt_keys_for_user(
    keys: &[&DecryptedDocumentKey],
    role: Role,
    my_private_key: &[u8; 32],
    my_public_key: &[u8; 32],
    their_public_key: &[u8; 32],
) -> Result<Vec<EncryptedKeyEntry>, String> {
    keys.iter()
        .map(|key| {
            let encrypted_data = key.key_data.encrypt_for_role(
                role,
                my_private_key,
                my_public_key,
                their_public_key,
            )?;

            Ok(EncryptedKeyEntry {
                key_timestamp: key.key_timestamp,
                encrypted_data,
            })
        })
        .collect()
}
