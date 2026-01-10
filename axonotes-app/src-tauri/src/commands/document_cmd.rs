//! # Document Commands
//!
//! Tauri commands for document lifecycle management.
//!
//! ## Operations
//!
//! - **Create**: Generates new document with encryption keys and metadata
//! - **Delete**: Removes document (owner only, requires signature)
//! - **List**: Returns all accessible documents with decrypted metadata
//! - **Update Metadata**: Changes document path/tags
//!
//! ## Security
//!
//! - Document deletion requires Ed25519 signature verification
//! - All metadata is encrypted per-user

use crate::batch_handler::block_setter::update_block;
use crate::batch_handler::block_types::BlockContent;
use crate::batch_handler::block_types::{Block, MetadataV1};
use crate::crypto::chacha::generate_key;
use crate::crypto::ed25519::{generate_ed25519_keys, sign_message};
use crate::database::get_active_user_keys;
use crate::database::keys::Keys;
use crate::encryption::document::{
    DecryptedDocumentKey, DecryptedDocumentMetadata, DecryptedKeyData, DecryptedMetadata,
};
use crate::events::{emit_document_created, emit_document_deleted, emit_document_metadata_updated};
use crate::stdb;
use crate::storage;
use crate::utils::timestamp::timestamp;
use crate::utils::vec_array::ByteArrayConversion;
use uuid::Uuid;

/// Creates a new document with encryption keys and metadata.
///
/// # Arguments
///
/// * `title` - Optional document title (defaults to "Default")
/// * `folder_path` - Optional folder path (defaults to "/")
///
/// # Returns
///
/// The new document's UUID.
///
/// # Process
///
/// 1. Generates unique path within folder (deduplicating if needed)
/// 2. Creates Ed25519 signing keypair for the document
/// 3. Creates ChaCha20 encryption key
/// 4. Encrypts keys for the creating user
/// 5. Creates initial metadata block with title
/// 6. Saves initial metadata to local SQLite for offline access
#[tauri::command]
pub async fn create_document(
    title: Option<String>,
    folder_path: Option<String>,
) -> Result<String, String> {
    let user_keys: Option<Keys> = get_active_user_keys().await?;
    if let Some(user_keys) = user_keys {
        let private_encryption_key = user_keys.private_encryption_key.as_array()?;
        let public_encryption_key = user_keys.public_encryption_key.as_array()?;

        // Get metadata from local DB for path deduplication
        let document_metadata: Vec<DecryptedDocumentMetadata> =
            crate::database::get_metadata_for_active_user()
                .await
                .unwrap_or_default();

        let doc_id = Uuid::new_v4().to_string();
        let document_title = if let Some(title) = title {
            title
        } else {
            "Default".to_string()
        };

        // Normalize folder path
        let folder = folder_path.unwrap_or_else(|| "/".to_string());
        let folder = if folder.is_empty() || folder == "/" {
            "".to_string()
        } else {
            // Ensure folder starts with / and doesn't end with /
            let f = if folder.starts_with('/') {
                folder.clone()
            } else {
                format!("/{folder}")
            };
            f.trim_end_matches('/').to_string()
        };

        // Generate unique path within the folder
        let base_filename = format!("{document_title}.doc");
        let mut path = format!("{folder}/{base_filename}");
        let mut counter = 1;

        // Check if path already exists in the same folder and increment counter if needed
        while document_metadata.iter().any(|m| m.metadata.path == path) {
            path = format!("{folder}/{document_title} ({counter}).doc");
            counter += 1;
        }

        let (private_doc_signing_key, public_doc_signing_key) = generate_ed25519_keys();
        let key_timestamp = timestamp();
        let encryption_key = generate_key();

        let key_data = DecryptedKeyData {
            encryption_key,
            signing_private_key: private_doc_signing_key.to_vec(),
        };
        let encrypted_key_data = key_data.encrypt(
            private_encryption_key,
            public_encryption_key,
            public_encryption_key,
        )?;

        let meta_data = DecryptedMetadata {
            version: 1,
            path,
            tags: vec![],
        };
        let encrypted_meta_blob = meta_data.encrypt(
            private_encryption_key,
            public_encryption_key,
            public_encryption_key,
        )?;

        // Get user identity first - we need it for saving both key and metadata
        let user_identity = stdb::active_profile()
            .get_identity()
            .await?
            .ok_or("No identity available")?;

        stdb::active_profile()
            .create_document(
                doc_id.to_string(),
                public_doc_signing_key.to_vec(),
                key_timestamp,
                encrypted_key_data,
                encrypted_meta_blob.clone(),
            )
            .await?;

        // Save document key to local SQLite immediately so batch uploads don't fail
        // The key will be overwritten later by the sync callback, but this ensures
        // the key is available locally before update_block creates a batch
        let local_key = DecryptedDocumentKey {
            key_id: format!("{}_{}", doc_id, key_timestamp),
            doc_id: doc_id.clone(),
            user_id: user_identity,
            key_timestamp,
            key_index: 0, // First key for this document
            key_data: key_data.clone(),
        };
        if let Err(e) = crate::database::save_document_key(user_identity, local_key).await {
            log::warn!("Warning: Failed to save document key locally: {e}");
        }

        // Save initial metadata to local SQLite for offline access
        let user_identity_for_meta = user_identity;
        let initial_metadata = DecryptedDocumentMetadata {
            // Generate temporary meta_id; will be overwritten by server sync
            meta_id: format!("{}_{}", user_identity_for_meta.to_hex(), doc_id),
            user_id: user_identity_for_meta,
            doc_id: doc_id.clone(),
            metadata: meta_data.clone(),
        };
        if let Err(e) =
            crate::database::save_metadata(user_identity_for_meta, initial_metadata).await
        {
            log::warn!("Warning: Failed to save initial metadata locally: {e}");
        }

        // Register document with storage API (if storage is initialized)
        // This is non-blocking - storage registration can fail without affecting document creation
        if storage::is_initialized().await {
            if let Err(e) = storage::active_profile()
                .create_storage_document(&doc_id)
                .await
            {
                eprintln!("Warning: Failed to register document with storage API: {e}");
            }
        }

        update_block(
            doc_id.clone(),
            None,
            Block {
                id: 0,
                timestamp: 0,
                deleted: None,
                content: BlockContent::MetadataV1(MetadataV1 {
                    author: user_identity,
                    group_id: "main".to_string(),
                    group_row: "main".to_string(),
                    field: "title".to_string(),
                    value: document_title.into(),
                }),
            },
        );

        emit_document_created(doc_id.clone());

        Ok(doc_id)
    } else {
        Err("No user keys found".to_string())
    }
}

/// Deletes a document (owner only).
///
/// # Security
///
/// Requires a valid Ed25519 signature over `"delete_document" + doc_id`.
/// The server verifies the signature against the user's registered public key.
#[tauri::command]
pub async fn delete_document(doc_id: String) -> Result<(), String> {
    let user_keys: Option<Keys> = get_active_user_keys().await?;
    if let Some(user_keys) = user_keys {
        let private_signing_key = user_keys.private_signing_key.as_array()?;

        let message = [b"delete_document", doc_id.as_bytes()].concat();
        let signature = sign_message(private_signing_key, message.as_slice())
            .map_err(|e| format!("Error signing 'delete_document' message: {e}"))?;

        stdb::active_profile()
            .delete_document(doc_id.clone(), signature.to_vec())
            .await?;

        // Unregister document from storage API and delete cached blobs (if storage is initialized)
        if storage::is_initialized().await {
            if let Err(e) = storage::active_profile()
                .delete_storage_document(&doc_id)
                .await
            {
                eprintln!("Warning: Failed to unregister document from storage API: {e}");
            }
        }

        // Delete local metadata
        let identity = stdb::active_profile()
            .get_identity()
            .await?
            .ok_or("No identity available")?;
        if let Err(e) = crate::database::delete_metadata_for_doc(identity, doc_id.clone()).await {
            log::warn!("Warning: Failed to delete local metadata: {e}");
        }

        emit_document_deleted(doc_id);

        Ok(())
    } else {
        Err("No user keys found".to_string())
    }
}

/// Gets metadata for a specific document.
/// Reads from local SQLite for offline support.
#[tauri::command]
pub async fn get_document_meta(
    doc_id: String,
) -> Result<Option<DecryptedDocumentMetadata>, String> {
    let identity = stdb::active_profile()
        .get_identity()
        .await?
        .ok_or("No identity available")?;

    crate::database::get_metadata_for_doc(identity, doc_id).await
}

/// Lists all documents accessible to the current user.
/// Reads from local SQLite for offline support.
#[tauri::command]
pub async fn list_documents() -> Result<Vec<DecryptedDocumentMetadata>, String> {
    crate::database::get_metadata_for_active_user().await
}

/// Updates document metadata (path and tags).
///
/// Updates local SQLite first for immediate UI response,
/// then attempts to push to server (fails silently if offline).
///
/// Metadata is encrypted to the user's own public key, allowing each
/// user to have their own organizational structure for shared documents.
#[tauri::command]
pub async fn update_document_metadata(
    doc_id: String,
    metadata: DecryptedMetadata,
) -> Result<(), String> {
    let user_keys: Option<Keys> = get_active_user_keys().await?;
    if let Some(user_keys) = user_keys {
        let private_encryption_key = user_keys.private_encryption_key.as_array()?;
        let public_encryption_key = user_keys.public_encryption_key.as_array()?;

        let identity = stdb::active_profile()
            .get_identity()
            .await?
            .ok_or("No identity available")?;

        // Get current metadata to preserve meta_id
        let current = crate::database::get_metadata_for_doc(identity, doc_id.clone())
            .await?
            .ok_or("Document metadata not found")?;

        // Create updated metadata record
        let updated = DecryptedDocumentMetadata {
            meta_id: current.meta_id,
            user_id: current.user_id,
            doc_id: doc_id.clone(),
            metadata: metadata.clone(),
        };

        // Update local SQLite immediately
        crate::database::save_metadata(identity, updated).await?;

        // Emit event for UI update
        emit_document_metadata_updated(
            doc_id.clone(),
            metadata.path.clone(),
            metadata.tags.clone(),
        );

        // Try to push to server (non-blocking, fails silently if offline)
        let encrypted_blob = metadata.encrypt(
            private_encryption_key,
            public_encryption_key,
            public_encryption_key,
        )?;

        // Spawn background task for server update
        let doc_id_for_server = doc_id.clone();
        tokio::spawn(async move {
            if let Err(e) = stdb::active_profile()
                .update_document_metadata(doc_id_for_server, encrypted_blob)
                .await
            {
                // Log but don't fail - local update succeeded
                log::warn!("[update_metadata] Server update failed (offline?): {e}");
            }
        });

        Ok(())
    } else {
        Err("No user keys found".to_string())
    }
}
