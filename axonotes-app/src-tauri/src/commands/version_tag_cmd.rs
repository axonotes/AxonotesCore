use crate::crypto::ed25519::sign_message;
use crate::database::get_active_user_keys;
use crate::database::keys::Keys;
use crate::encryption::version_tag::DecryptedVersionTagData;
use crate::events::{emit_version_tag_created, emit_version_tag_deleted};
use crate::stdb;
use crate::utils::timestamp::timestamp;
use crate::utils::vec_array::ByteArrayConversion;
use serde::Serialize;
use spacetimedb_sdk::Identity;
use uuid::Uuid;

/// Version tag info returned to frontend
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionTagInfo {
    pub tag_id: String,
    pub doc_id: String,
    pub tag_name: String,
    pub timestamp: u128,
    pub created_by: Identity,
    pub created_at: u128,
}

/// Create a version tag for a document
///
/// Creates a named bookmark at a specific timestamp in the document's history.
/// Only Owner or Editor can create tags.
///
/// # Arguments
/// * `doc_id` - The document ID
/// * `tag_name` - A descriptive name for the tag (e.g., "v1.0", "Before refactor")
/// * `tag_timestamp` - The timestamp to tag (must be a valid point in history)
#[tauri::command]
pub async fn create_version_tag(
    doc_id: String,
    tag_name: String,
    tag_timestamp: u128,
) -> Result<(), String> {
    // Get user keys
    let user_keys: Keys = get_active_user_keys().await?.ok_or("No active user keys")?;
    let private_signing_key = user_keys.private_signing_key.as_array()?;

    // Get document key for encryption
    let document_keys = stdb::active_profile().get_cached_document_keys().await?;
    let doc_key = document_keys
        .iter()
        .filter(|k| k.doc_id == doc_id)
        .max_by_key(|k| k.key_timestamp)
        .ok_or("No document key found")?;

    // Get user identity
    let user_identity = stdb::active_profile()
        .get_identity()
        .await?
        .ok_or("User identity not found")?;

    // Generate tag ID
    let tag_id = Uuid::new_v4().to_string();

    // Create tag data
    let tag_data = DecryptedVersionTagData {
        tag_name: tag_name.clone(),
        timestamp: tag_timestamp,
        created_by: user_identity,
        created_at: timestamp(),
    };

    // Encrypt tag data
    let encrypted_blob = tag_data
        .encrypt(&doc_key.key_data.encryption_key)
        .map_err(|e| format!("Failed to encrypt tag: {e}"))?;

    // Sign the create_version_tag message (must match server format)
    let blob_hash = blake3::hash(&encrypted_blob);
    let message = [
        b"create_version_tag".as_slice(),
        doc_id.as_bytes(),
        tag_id.as_bytes(),
        blob_hash.as_bytes(),
    ]
    .concat();

    let signature = sign_message(private_signing_key, &message)
        .map_err(|e| format!("Failed to sign message: {e}"))?;

    // Call the reducer
    stdb::active_profile()
        .create_version_tag(
            tag_id.clone(),
            doc_id.clone(),
            encrypted_blob,
            signature.to_vec(),
        )
        .await?;

    emit_version_tag_created(tag_id, doc_id, tag_name, tag_timestamp);

    Ok(())
}

/// Delete a version tag
///
/// Only the document Owner can delete version tags.
///
/// # Arguments
/// * `tag_id` - The ID of the tag to delete
/// * `doc_id` - The document ID (for event emission)
#[tauri::command]
pub async fn delete_version_tag(tag_id: String, doc_id: String) -> Result<(), String> {
    // Get user keys
    let user_keys: Keys = get_active_user_keys().await?.ok_or("No active user keys")?;
    let private_signing_key = user_keys.private_signing_key.as_array()?;

    // Sign the delete_version_tag message (must match server format)
    let message = [b"delete_version_tag".as_slice(), tag_id.as_bytes()].concat();

    let signature = sign_message(private_signing_key, &message)
        .map_err(|e| format!("Failed to sign message: {e}"))?;

    // Call the reducer
    stdb::active_profile()
        .delete_version_tag(tag_id.clone(), signature.to_vec())
        .await?;

    emit_version_tag_deleted(tag_id, doc_id);

    Ok(())
}

/// List all version tags for a document
///
/// Returns all version tags for the document, sorted by timestamp descending.
///
/// # Arguments
/// * `doc_id` - The document ID
#[tauri::command]
pub async fn list_version_tags(doc_id: String) -> Result<Vec<VersionTagInfo>, String> {
    // Get decrypted version tags
    let mut tags = stdb::active_profile()
        .get_cached_version_tags(&doc_id)
        .await?;

    // Sort by timestamp descending (most recent first)
    tags.sort_by(|a, b| b.data.timestamp.cmp(&a.data.timestamp));

    // Convert to frontend-friendly format
    let result = tags
        .into_iter()
        .map(|tag| VersionTagInfo {
            tag_id: tag.tag_id,
            doc_id: tag.doc_id,
            tag_name: tag.data.tag_name,
            timestamp: tag.data.timestamp,
            created_by: tag.data.created_by,
            created_at: tag.data.created_at,
        })
        .collect();

    Ok(result)
}
