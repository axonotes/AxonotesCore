//! # Encryption Helpers
//!
//! Utility functions for encryption key selection and management.

use crate::encryption::document::DecryptedDocumentKey;

/// Finds the correct decryption key for a given timestamp.
///
/// When documents have multiple keys due to key rotation, this function
/// selects the appropriate key based on temporal ordering:
///
/// 1. Filters to keys for the specified document
/// 2. Filters to keys created before or at the given timestamp
/// 3. Returns the most recent key (highest `key_timestamp`)
///
/// ## Key Rotation Model
///
/// ```text
/// Timeline:     |---Key A---|---Key B---|---Key C---|-->
/// Key A valid:  [created_at_A, created_at_B)
/// Key B valid:  [created_at_B, created_at_C)
/// Key C valid:  [created_at_C, ∞)
/// ```
///
/// # Arguments
///
/// * `doc_id` - The document ID to find keys for
/// * `timestamp` - The batch/content timestamp to decrypt
/// * `document_keys` - Available decrypted keys for the user
///
/// # Errors
///
/// Returns an error if no valid key exists (e.g., user lost access before `timestamp`).
pub fn find_correct_decryption_key<'a>(
    doc_id: &str,
    timestamp: u128,
    document_keys: &'a [DecryptedDocumentKey],
) -> Result<&'a DecryptedDocumentKey, String> {
    document_keys
        .iter()
        .filter(|key| key.doc_id == doc_id && key.key_timestamp <= timestamp)
        .max_by_key(|key| key.key_timestamp)
        .ok_or_else(|| {
            format!("No valid encryption key found for doc_id: {doc_id} at timestamp: {timestamp}")
        })
}
