//! # Version Tag Commands
//!
//! Tauri commands for managing document version tags.
//!
//! ## Version Tags
//!
//! Version tags are named bookmarks at specific points in a document's history.
//! They allow users to:
//! - Mark significant milestones (e.g., "v1.0 Release")
//! - Create restore points before major changes
//! - Navigate document history semantically
//!
//! ## Tag Data
//!
//! Each tag stores:
//! - `tag_name`: User-provided name
//! - `timestamp`: Point in document history being tagged
//! - `created_by`: User who created the tag
//! - `created_at`: When the tag was created
//!
//! ## Encryption
//!
//! Tag data is encrypted with the document's encryption key to protect
//! tag names from unauthorized access.

use crate::crypto::ed25519::sign_message;
use crate::database::get_active_user_keys;
use crate::database::keys::Keys;
use crate::encryption::version_tag::{DecryptedVersionTag, DecryptedVersionTagData};
use crate::events::{emit_version_tag_created, emit_version_tag_deleted};
use crate::stdb;
use crate::utils::timestamp::timestamp;
use crate::utils::vec_array::ByteArrayConversion;
use serde::Serialize;
use spacetimedb_sdk::Identity;
use uuid::Uuid;

/// Version tag information returned to the frontend.
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
/// Works offline: saves locally first, uploads when connected.
/// If upload fails, tag is saved as pending for later upload.
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
    let document_keys = crate::database::get_document_keys_for_active_user().await?;
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
    let created_at = timestamp();

    // Create tag data
    let tag_data = DecryptedVersionTagData {
        tag_name: tag_name.clone(),
        timestamp: tag_timestamp,
        created_by: user_identity,
        created_at,
    };

    // Create the decrypted tag for local storage
    let decrypted_tag = DecryptedVersionTag {
        tag_id: tag_id.clone(),
        doc_id: doc_id.clone(),
        data: tag_data.clone(),
    };

    // Check if we're connected
    let is_connected = stdb::active_profile().is_connected().await.unwrap_or(false);

    if is_connected {
        // Try to upload to server
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

        match stdb::active_profile()
            .create_version_tag(
                tag_id.clone(),
                doc_id.clone(),
                encrypted_blob,
                signature.to_vec(),
            )
            .await
        {
            Ok(_) => {
                // Upload succeeded - on_insert callback will save with pending=0
                // But save locally too for immediate availability
                if let Err(e) =
                    crate::database::save_version_tag(user_identity, decrypted_tag).await
                {
                    log::warn!("Failed to save version tag locally after upload: {e}");
                }
            }
            Err(e) => {
                // Upload failed - save as pending for later upload
                log::warn!("Version tag upload failed, saving as pending: {e}");
                crate::database::save_pending_version_tag(user_identity, decrypted_tag).await?;
            }
        }
    } else {
        // Offline - save as pending for later upload
        crate::database::save_pending_version_tag(user_identity, decrypted_tag).await?;
    }

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
/// Reads from local SQLite for offline support.
///
/// # Arguments
/// * `doc_id` - The document ID
#[tauri::command]
pub async fn list_version_tags(doc_id: String) -> Result<Vec<VersionTagInfo>, String> {
    // Get identity for database query
    let identity = stdb::active_profile()
        .get_identity()
        .await?
        .ok_or("No identity available")?;

    // Get version tags from local SQLite (works offline)
    let mut tags = crate::database::get_version_tags_for_doc(identity, doc_id).await?;

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
