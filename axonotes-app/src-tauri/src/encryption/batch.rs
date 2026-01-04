use crate::crypto::chacha::{decrypt, encrypt};
use crate::encryption::document::DecryptedDocumentKey;
use crate::stdb_bindings::DocumentBatch;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Patch {
    pub delta: Vec<u8>,
    pub time_delta: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BatchData {
    pub block_id: u64,
    pub patches: Vec<Patch>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecryptedBatch {
    pub batch_id: String,
    pub doc_id: String,
    pub timestamp: u128,
    pub batch_data: BatchData,
}

impl DocumentBatch {
    pub fn decrypt(
        &self,
        document_keys: &Vec<DecryptedDocumentKey>,
    ) -> Result<DecryptedBatch, String> {
        let batch_timestamp = self.timestamp;

        // Find correct decryption key
        // 1. Filter keys for this document and where key_timestamp <= batch_timestamp
        // 2. Get the one with the highest timestamp
        let decryption_key: &DecryptedDocumentKey = document_keys
            .iter()
            .filter(|key| key.doc_id == self.doc_id && key.key_timestamp <= batch_timestamp)
            .max_by_key(|key| key.key_timestamp)
            .ok_or_else(|| {
                format!(
                    "No valid encryption key found for doc_id: {} at timestamp: {}",
                    self.doc_id, batch_timestamp
                )
            })?;

        // Decrypt the batch data blob with this key
        let decrypted = decrypt(
            decryption_key.key_data.encryption_key.as_slice(),
            self.encrypted_data.as_slice(),
        )
        .map_err(|e| format!("Error while decrypting batch data: {}", e))?;

        let batch_data: BatchData = from_bytes(decrypted.as_slice())
            .map_err(|e| format!("Error deserializing batch data: {}", e))?;

        Ok(DecryptedBatch {
            timestamp: self.timestamp,
            doc_id: self.doc_id.clone(),
            batch_id: self.batch_id.clone(),
            batch_data,
        })
    }
}

impl DecryptedBatch {
    pub fn encrypt(
        &self,
        latest_document_key: &DecryptedDocumentKey,
    ) -> Result<DocumentBatch, String> {
        let blob = to_allocvec(&self.batch_data)
            .map_err(|e| format!("Error serializing batch data: {}", e))?;

        let encrypted_data = encrypt(
            latest_document_key.key_data.encryption_key.as_slice(),
            blob.as_slice(),
        )
        .map_err(|e| format!("Error when encrypting batch data: {}", e))?;

        Ok(DocumentBatch {
            timestamp: self.timestamp,
            doc_id: self.doc_id.clone(),
            batch_id: self.batch_id.clone(),
            encrypted_data,
        })
    }
}
