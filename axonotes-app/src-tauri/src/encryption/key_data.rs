//! Document key data retrieval helpers.
//!
//! Provides functions to look up document encryption/signing keys
//! needed for blob operations.

use crate::encryption::document::DecryptedKeyData;
use crate::stdb;

/// Get the decrypted keys for a specific document.
///
/// Fetches document keys from the cached stdb data and returns
/// the decrypted key data for the specified document.
///
/// Returns `None` if the document is not found or user doesn't have access.
pub async fn get_document_keys(doc_id: &str) -> Result<Option<DecryptedKeyData>, String> {
    // Get all cached document keys
    let keys = stdb::active_profile().get_cached_document_keys().await?;

    // Find the key for the requested document
    // Note: There might be multiple keys for a document due to key rotation,
    // we want the most recent one (highest key_timestamp)
    let key = keys
        .into_iter()
        .filter(|k| k.doc_id == doc_id)
        .max_by_key(|k| k.key_timestamp);

    Ok(key.map(|k| k.key_data))
}

/// Get the signing key for a document (for blob upload signatures).
///
/// Returns the Ed25519 private key as a 32-byte array, or `None`
/// if the document is not found or user doesn't have signing access.
pub async fn get_document_signing_key(
    doc_id: &str,
) -> Result<Option<ed25519_dalek::SigningKey>, String> {
    let key_data = match get_document_keys(doc_id).await? {
        Some(k) => k,
        None => return Ok(None),
    };

    // Check if user has signing key (Readers don't)
    if key_data.signing_private_key.is_empty() {
        return Ok(None);
    }

    // Convert to SigningKey
    let key_bytes: [u8; 32] = key_data
        .signing_private_key
        .try_into()
        .map_err(|_| "Invalid signing key length")?;

    Ok(Some(ed25519_dalek::SigningKey::from_bytes(&key_bytes)))
}

/// Get the encryption key for a document (for blob encryption/decryption).
///
/// Returns the 32-byte ChaCha20 key, or `None` if the document
/// is not found or user doesn't have access.
pub async fn get_document_encryption_key(doc_id: &str) -> Result<Option<[u8; 32]>, String> {
    let key_data = match get_document_keys(doc_id).await? {
        Some(k) => k,
        None => return Ok(None),
    };

    let key_bytes: [u8; 32] = key_data
        .encryption_key
        .try_into()
        .map_err(|_| "Invalid encryption key length")?;

    Ok(Some(key_bytes))
}

/// Get the public signing key for a document.
///
/// This is derived from the private signing key and used for
/// storage API document registration.
pub async fn get_document_public_signing_key(
    doc_id: &str,
) -> Result<Option<ed25519_dalek::VerifyingKey>, String> {
    let signing_key = match get_document_signing_key(doc_id).await? {
        Some(k) => k,
        None => return Ok(None),
    };

    Ok(Some(signing_key.verifying_key()))
}
