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
use crate::encryption::document::{DecryptedDocumentMetadata, DecryptedKeyData, DecryptedMetadata};
use crate::events::{emit_document_created, emit_document_deleted, emit_document_metadata_updated};
use crate::stdb;
use crate::storage;
use crate::utils::timestamp::timestamp;
use crate::utils::vec_array::ByteArrayConversion;
use spacetimedb_sdk::Identity;
use uuid::Uuid;

/// Creates a new document with encryption keys and metadata.
///
/// # Arguments
///
/// * `title` - Optional document title (defaults to "Default")
///
/// # Returns
///
/// The new document's UUID.
///
/// # Process
///
/// 1. Generates unique path (deduplicating if needed)
/// 2. Creates Ed25519 signing keypair for the document
/// 3. Creates ChaCha20 encryption key
/// 4. Encrypts keys for the creating user
/// 5. Creates initial metadata block with title
#[tauri::command]
pub async fn create_document(title: Option<String>) -> Result<String, String> {
    let user_keys: Option<Keys> = get_active_user_keys().await?;
    if let Some(user_keys) = user_keys {
        let private_encryption_key = user_keys.private_encryption_key.as_array()?;
        let public_encryption_key = user_keys.public_encryption_key.as_array()?;

        let document_metadata: Vec<DecryptedDocumentMetadata> =
            stdb::active_profile().get_cached_metadata().await?;

        let doc_id = Uuid::new_v4().to_string();
        let document_title = if let Some(title) = title {
            title
        } else {
            "Default".to_string()
        };

        // Generate unique path
        let base_filename = format!("{document_title}.doc");
        let mut path = format!("/{base_filename}");
        let mut counter = 1;

        // Check if path already exists and increment counter if needed
        while document_metadata.iter().any(|m| m.metadata.path == path) {
            path = format!("/{document_title} ({counter}).doc");
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

        stdb::active_profile()
            .create_document(
                doc_id.to_string(),
                public_doc_signing_key.to_vec(),
                key_timestamp,
                encrypted_key_data,
                encrypted_meta_blob,
            )
            .await?;

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

        let user_identity: Option<Identity> = stdb::active_profile().get_identity().await?;
        let user_identity: Identity = user_identity.expect("User identity should be set by now");
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

        emit_document_deleted(doc_id);

        Ok(())
    } else {
        Err("No user keys found".to_string())
    }
}

/// Gets metadata for a specific document.
#[tauri::command]
pub async fn get_document_meta(
    doc_id: String,
) -> Result<Option<DecryptedDocumentMetadata>, String> {
    let document_metadata: Vec<DecryptedDocumentMetadata> =
        stdb::active_profile().get_cached_metadata().await?;

    Ok(document_metadata.into_iter().find(|p| p.doc_id == doc_id))
}

/// Lists all documents accessible to the current user.
#[tauri::command]
pub async fn list_documents() -> Result<Vec<DecryptedDocumentMetadata>, String> {
    stdb::active_profile().get_cached_metadata().await
}

/// Updates document metadata (path and tags).
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

        let encrypted_blob = metadata.encrypt(
            private_encryption_key,
            public_encryption_key,
            public_encryption_key,
        )?;

        stdb::active_profile()
            .update_document_metadata(doc_id.clone(), encrypted_blob)
            .await?;

        emit_document_metadata_updated(doc_id, metadata.path, metadata.tags);

        Ok(())
    } else {
        Err("No user keys found".to_string())
    }
}
