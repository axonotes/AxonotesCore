use crate::crypto::chacha::generate_key;
use crate::crypto::ed25519::{generate_ed25519_keys, sign_message};
use crate::database::get_active_user_keys;
use crate::database::keys::Keys;
use crate::encryption::document::{DecryptedDocumentMetadata, DecryptedKeyData, DecryptedMetadata};
use crate::stdb;
use crate::utils::timestamp::timestamp;
use crate::utils::vec_array::ByteArrayConversion;
use uuid::Uuid;

/// Create a new document with optional title (default: "Untitled") returns document id
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
        let base_filename = format!("{}.doc", document_title);
        let mut path = format!("/{}", base_filename);
        let mut counter = 1;

        // Check if path already exists and increment counter if needed
        while document_metadata.iter().any(|m| m.metadata.path == path) {
            path = format!("/{} ({}).doc", document_title, counter);
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

        // TODO: Create metadata batch with document title

        Ok(doc_id)
    } else {
        Err("No user keys found".to_string())
    }
}

#[tauri::command]
pub async fn delete_document(doc_id: String) -> Result<(), String> {
    let user_keys: Option<Keys> = get_active_user_keys().await?;
    if let Some(user_keys) = user_keys {
        let private_signing_key = user_keys.private_signing_key.as_array()?;

        let message = [b"delete_document", doc_id.as_bytes()].concat();
        let signature = sign_message(private_signing_key, message.as_slice())
            .map_err(|e| format!("Error signing 'delete_document' message: {}", e))?;

        stdb::active_profile()
            .delete_document(doc_id, signature.to_vec())
            .await?;

        Ok(())
    } else {
        Err("No user keys found".to_string())
    }
}

#[tauri::command]
pub async fn get_document_meta(
    doc_id: String,
) -> Result<Option<DecryptedDocumentMetadata>, String> {
    let document_metadata: Vec<DecryptedDocumentMetadata> =
        stdb::active_profile().get_cached_metadata().await?;

    Ok(document_metadata.into_iter().find(|p| p.doc_id == doc_id))
}

#[tauri::command]
pub async fn list_documents() -> Result<Vec<DecryptedDocumentMetadata>, String> {
    stdb::active_profile().get_cached_metadata().await
}

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
            .update_document_metadata(doc_id, encrypted_blob)
            .await?;

        Ok(())
    } else {
        Err("No user keys found".to_string())
    }
}
