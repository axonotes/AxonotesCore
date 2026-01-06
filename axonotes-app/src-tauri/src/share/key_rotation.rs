use crate::batch_handler::block_getter::{get_blocks, BlockData};
use crate::batch_handler::block_type_helpers::encode_initial_patch;
use crate::crypto::chacha::generate_key;
use crate::crypto::ed25519::{generate_ed25519_keys, sign_message};
use crate::database;
use crate::database::get_active_user_keys;
use crate::database::keys::Keys;
use crate::encryption::batch::{BatchData, DecryptedBatch};
use crate::encryption::document::{DecryptedDocumentKey, DecryptedKeyData};
use crate::stdb;
use crate::stdb_bindings::{
    DocumentPermission, EncryptedKeyEntry, PublicUserInfo, ReEncryptedTag, SnapshotBatch,
    UserKeyEntry,
};
use crate::utils::timestamp::timestamp;
use crate::utils::vec_array::ByteArrayConversion;
use spacetimedb_sdk::Identity;
use std::collections::HashSet;
use uuid::Uuid;

/// Result of key rotation containing all necessary data for add_user_to_document
pub struct KeyRotationResult {
    pub new_key_timestamp: u128,
    pub new_key_data: DecryptedKeyData,
    pub new_public_signing_key: Vec<u8>,
}

/// Rotate document keys and return the new key for the joiner
///
/// This function:
/// 1. Generates new document keys (ChaCha20 encryption + Ed25519 signing)
/// 2. Creates snapshot batches for ALL blocks with current state
/// 3. Gets all existing users and encrypts new key for them
/// 4. Re-encrypts version tags with new key
/// 5. Calls rotate_document_keys reducer
/// 6. Returns the new key data for the joiner
pub async fn rotate_keys_for_share(doc_id: &str) -> Result<KeyRotationResult, String> {
    // Get user keys
    let user_keys: Keys = get_active_user_keys().await?.ok_or("No active user keys")?;

    let private_encryption_key = user_keys.private_encryption_key.as_array()?;
    let public_encryption_key = user_keys.public_encryption_key.as_array()?;
    let private_signing_key: &[u8; 32] = user_keys.private_signing_key.as_array()?;

    // Generate new document keys
    let (new_private_signing_key, new_public_signing_key) = generate_ed25519_keys();
    let new_encryption_key = generate_key();
    let new_key_timestamp = timestamp();

    let new_key_data = DecryptedKeyData {
        encryption_key: new_encryption_key.to_vec(),
        signing_private_key: new_private_signing_key.to_vec(),
    };

    // Get all existing permissions for this document (existing users)
    let permissions = get_document_permissions(doc_id).await?;
    let public_keys = get_collaborator_public_keys().await?;

    // Create user key entries for all existing users
    let user_key_entries = create_user_key_entries(
        &permissions,
        &public_keys,
        &new_key_data,
        private_encryption_key,
        public_encryption_key,
    )?;

    // Create snapshot batches for all blocks
    let snapshot_batches =
        create_snapshot_batches(doc_id, &new_key_data, new_key_timestamp).await?;

    // Re-encrypt version tags (if any)
    let re_encrypted_tags = re_encrypt_version_tags(doc_id, &new_key_data).await?;

    // Sign the rotation message
    let signature = sign_rotation_message(
        doc_id,
        new_key_timestamp,
        &new_public_signing_key,
        &user_key_entries,
        &snapshot_batches,
        &re_encrypted_tags,
        private_signing_key,
    )?;

    // Call rotate_document_keys reducer
    stdb::active_profile()
        .rotate_document_keys(
            doc_id.to_string(),
            new_key_timestamp,
            new_public_signing_key.to_vec(),
            user_key_entries,
            snapshot_batches,
            re_encrypted_tags,
            signature.to_vec(),
        )
        .await?;

    Ok(KeyRotationResult {
        new_key_timestamp,
        new_key_data,
        new_public_signing_key: new_public_signing_key.to_vec(),
    })
}

/// Get document permissions from the manageable_permissions view
async fn get_document_permissions(doc_id: &str) -> Result<Vec<DocumentPermission>, String> {
    stdb::active_profile()
        .get_document_permissions(doc_id)
        .await
}

/// Get public keys of all collaborators
async fn get_collaborator_public_keys() -> Result<Vec<PublicUserInfo>, String> {
    stdb::active_profile().get_public_user_keys().await
}

/// Create UserKeyEntry for each existing user by encrypting the new key with their public key
fn create_user_key_entries(
    permissions: &[DocumentPermission],
    public_keys: &[PublicUserInfo],
    new_key_data: &DecryptedKeyData,
    my_private_key: &[u8; 32],
    my_public_key: &[u8; 32],
) -> Result<Vec<UserKeyEntry>, String> {
    let mut entries = Vec::new();

    for perm in permissions {
        // Find user's public encryption key
        let user_public_key = public_keys
            .iter()
            .find(|pk| pk.identity == perm.user_id)
            .map(|pk| &pk.public_encryption_key)
            .ok_or_else(|| format!("Public key not found for user {}", perm.user_id.to_hex()))?;

        let their_key_array: &[u8; 32] = user_public_key
            .as_slice()
            .try_into()
            .map_err(|_| "User public key must be 32 bytes")?;

        // Encrypt new key for this user
        let encrypted_data =
            new_key_data.encrypt(my_private_key, my_public_key, their_key_array)?;

        entries.push(UserKeyEntry {
            user_id: perm.user_id,
            encrypted_key_data: encrypted_data,
        });
    }

    Ok(entries)
}

/// Create snapshot batches for all blocks in the document
async fn create_snapshot_batches(
    doc_id: &str,
    new_key_data: &DecryptedKeyData,
    new_key_timestamp: u128,
) -> Result<Vec<SnapshotBatch>, String> {
    // Get all unique block IDs from local database
    let block_ids = get_all_block_ids(doc_id).await?;

    if block_ids.is_empty() {
        return Ok(vec![]);
    }

    // Get current state of all blocks
    let document_data = get_blocks(doc_id.to_string(), block_ids, u128::MAX).await?;

    let mut snapshots = Vec::new();

    for block_data in document_data.blocks {
        let snapshot =
            create_snapshot_for_block(doc_id, &block_data, new_key_data, new_key_timestamp)?;
        snapshots.push(snapshot);
    }

    Ok(snapshots)
}

/// Get all unique block IDs for a document from local database
async fn get_all_block_ids(doc_id: &str) -> Result<Vec<u64>, String> {
    let batches = database::get_batches_by_doc(doc_id.to_string()).await?;

    let block_ids: HashSet<u64> = batches.iter().map(|b| b.batch_data.block_id).collect();

    Ok(block_ids.into_iter().collect())
}

/// Create a snapshot batch for a single block
fn create_snapshot_for_block(
    doc_id: &str,
    block_data: &BlockData,
    new_key_data: &DecryptedKeyData,
    new_key_timestamp: u128,
) -> Result<SnapshotBatch, String> {
    // Encode current block state as initial patch
    let initial_patch = encode_initial_patch(0, 0, &block_data.block)?;

    // Create batch data
    let batch_data = BatchData {
        block_id: block_data.block_id,
        patches: vec![initial_patch],
    };

    // Generate UUIDv4 for batch_id
    let batch_id = Uuid::new_v4().to_string();

    // Create decrypted batch
    let decrypted_batch = DecryptedBatch {
        is_initial: true,
        batch_id: batch_id.clone(),
        doc_id: doc_id.to_string(),
        timestamp: new_key_timestamp,
        batch_data,
    };

    // Create temporary key for encryption
    let temp_key = DecryptedDocumentKey {
        key_id: "temp".to_string(),
        doc_id: doc_id.to_string(),
        user_id: Identity::from_byte_array([0u8; 32]),
        key_timestamp: new_key_timestamp,
        key_data: new_key_data.clone(),
    };

    // Encrypt with new key
    let encrypted_batch = decrypted_batch.encrypt(&temp_key)?;

    Ok(SnapshotBatch {
        batch_id: encrypted_batch.batch_id,
        encrypted_data: encrypted_batch.encrypted_data,
    })
}

/// Re-encrypt version tags with new key
async fn re_encrypt_version_tags(
    doc_id: &str,
    new_key_data: &DecryptedKeyData,
) -> Result<Vec<ReEncryptedTag>, String> {
    // Get already-decrypted version tags for this document
    let decrypted_tags = stdb::active_profile()
        .get_cached_version_tags(doc_id)
        .await?;

    let mut re_encrypted = Vec::new();

    for tag in decrypted_tags {
        // Re-encrypt with new key using the encryption module
        let encrypted_tag = tag
            .encrypt(&new_key_data.encryption_key)
            .map_err(|e| format!("Failed to re-encrypt tag {}: {}", tag.tag_id, e))?;

        re_encrypted.push(ReEncryptedTag {
            tag_id: encrypted_tag.tag_id,
            encrypted_blob: encrypted_tag.encrypted_blob,
        });
    }

    Ok(re_encrypted)
}

/// Sign the rotation message
fn sign_rotation_message(
    doc_id: &str,
    new_key_timestamp: u128,
    new_public_signing_key: &[u8],
    user_keys: &[UserKeyEntry],
    snapshot_batches: &[SnapshotBatch],
    re_encrypted_tags: &[ReEncryptedTag],
    private_signing_key: &[u8; 32],
) -> Result<[u8; 64], String> {
    // Build bytes for hashing
    let mut user_keys_bytes = Vec::new();
    for uk in user_keys {
        user_keys_bytes.extend_from_slice(uk.user_id.to_byte_array().as_slice());
        user_keys_bytes.extend_from_slice(&uk.encrypted_key_data);
    }

    let mut snapshots_bytes = Vec::new();
    for snap in snapshot_batches {
        snapshots_bytes.extend_from_slice(snap.batch_id.as_bytes());
        snapshots_bytes.extend_from_slice(&snap.encrypted_data);
    }

    let mut tags_bytes = Vec::new();
    for tag in re_encrypted_tags {
        tags_bytes.extend_from_slice(tag.tag_id.as_bytes());
        tags_bytes.extend_from_slice(&tag.encrypted_blob);
    }

    let message = [
        b"rotate_keys".as_slice(),
        doc_id.as_bytes(),
        &new_key_timestamp.to_le_bytes(),
        new_public_signing_key,
        blake3::hash(&user_keys_bytes).as_bytes(),
        blake3::hash(&snapshots_bytes).as_bytes(),
        blake3::hash(&tags_bytes).as_bytes(),
    ]
    .concat();

    sign_message(private_signing_key, &message)
        .map_err(|e| format!("Failed to sign rotation message: {}", e))
}

/// Encrypt a key for a specific user
pub fn encrypt_key_for_user(
    key_data: &DecryptedKeyData,
    key_timestamp: u128,
    my_private_key: &[u8; 32],
    my_public_key: &[u8; 32],
    their_public_key: &[u8; 32],
) -> Result<EncryptedKeyEntry, String> {
    let encrypted_data = key_data.encrypt(my_private_key, my_public_key, their_public_key)?;

    Ok(EncryptedKeyEntry {
        key_timestamp,
        encrypted_data,
    })
}
