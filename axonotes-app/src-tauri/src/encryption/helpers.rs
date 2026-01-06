use crate::encryption::document::DecryptedDocumentKey;

/// Find correct decryption key
/// 1. Filter keys for this document and where key_timestamp <= batch_timestamp
/// 2. Get the one with the highest timestamp
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
