use crate::crypto::chacha::{decrypt, encrypt};
use crate::encryption::document::DecryptedDocumentKey;
use crate::encryption::helpers::find_correct_decryption_key;
use crate::stdb_bindings::DocumentBatch;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};

pub trait DecryptDocumentBatchVec {
    fn decrypt_all(
        self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<Vec<DecryptedBatch>, String>;
}

pub trait EncryptDocumentBatchVec {
    fn encrypt_all(
        self,
        latest_document_key: &DecryptedDocumentKey,
    ) -> Result<Vec<DocumentBatch>, String>;
}

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
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<DecryptedBatch, String> {
        let batch_timestamp = self.timestamp;

        let decryption_key: &DecryptedDocumentKey =
            find_correct_decryption_key(self.doc_id.to_string(), batch_timestamp, document_keys)?;

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

impl DecryptDocumentBatchVec for Vec<DocumentBatch> {
    fn decrypt_all(
        self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<Vec<DecryptedBatch>, String> {
        self.into_iter()
            .map(|batch| batch.decrypt(document_keys))
            .collect()
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

impl EncryptDocumentBatchVec for Vec<DecryptedBatch> {
    fn encrypt_all(
        self,
        latest_document_key: &DecryptedDocumentKey,
    ) -> Result<Vec<DocumentBatch>, String> {
        self.into_iter()
            .map(|batch| batch.encrypt(latest_document_key))
            .collect()
    }
}
