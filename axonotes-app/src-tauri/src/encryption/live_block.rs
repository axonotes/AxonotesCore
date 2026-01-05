use crate::crypto::chacha::{decrypt, encrypt};
use crate::encryption::document::DecryptedDocumentKey;
use crate::encryption::helpers::find_correct_decryption_key;
use crate::stdb_bindings::LiveBlock;
use crate::utils::timestamp::timestamp;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use spacetimedb_sdk::Identity;

pub trait DecryptLiveBlockVec {
    fn decrypt_all(
        self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<Vec<DecryptedLiveBlock>, String>;
}

pub trait EncryptLiveBlockVec {
    fn encrypt_all(
        self,
        latest_document_key: &DecryptedDocumentKey,
    ) -> Result<Vec<LiveBlock>, String>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecryptedLiveBlock {
    pub live_block_id: String,
    pub doc_id: String,
    pub block_id: u64,
    pub user_id: Identity,
    pub content: Vec<u8>,
    pub username: String,
    pub locked_at: Option<u128>,
}

impl LiveBlock {
    pub fn decrypt(
        &self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<DecryptedLiveBlock, String> {
        let timestamp = if let Some(locked_at) = self.locked_at {
            locked_at
        } else {
            timestamp()
        };

        // Find correct decryption key
        // 1. Filter keys for this document and where key_timestamp <= batch_timestamp
        // 2. Get the one with the highest timestamp
        let decryption_key: &DecryptedDocumentKey =
            find_correct_decryption_key(self.doc_id.to_string(), timestamp, document_keys)?;

        let content = decrypt(
            decryption_key.key_data.encryption_key.as_slice(),
            self.encrypted_content.as_slice(),
        )
        .map_err(|e| format!("Error while decrypting live block content: {}", e))?;

        let decrypted_username = decrypt(
            decryption_key.key_data.encryption_key.as_slice(),
            self.encrypted_username.as_slice(),
        )
        .map_err(|e| format!("Error while decrypting live block username: {}", e))?;

        let username: String = from_bytes(decrypted_username.as_slice())
            .map_err(|e| format!("Error deserializing username: {}", e))?;

        Ok(DecryptedLiveBlock {
            doc_id: self.doc_id.to_string(),
            locked_at: self.locked_at,
            user_id: self.user_id,
            block_id: self.block_id,
            live_block_id: self.live_block_id.to_string(),
            content,
            username,
        })
    }
}

impl DecryptLiveBlockVec for Vec<LiveBlock> {
    fn decrypt_all(
        self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<Vec<DecryptedLiveBlock>, String> {
        self.into_iter()
            .map(|block| block.decrypt(document_keys))
            .collect()
    }
}

impl DecryptedLiveBlock {
    pub fn encrypt(&self, latest_document_key: &DecryptedDocumentKey) -> Result<LiveBlock, String> {
        let username_blob = to_allocvec(&self.username)
            .map_err(|e| format!("Error serializing batch data: {}", e))?;

        let encrypted_content = encrypt(
            latest_document_key.key_data.encryption_key.as_slice(),
            self.content.as_slice(),
        )
        .map_err(|e| format!("Error when encrypting live block content: {}", e))?;

        let encrypted_username = encrypt(
            latest_document_key.key_data.encryption_key.as_slice(),
            username_blob.as_slice(),
        )
        .map_err(|e| format!("Error when encrypting live block username: {}", e))?;

        Ok(LiveBlock {
            doc_id: self.doc_id.to_string(),
            locked_at: self.locked_at,
            user_id: self.user_id,
            block_id: self.block_id,
            live_block_id: self.live_block_id.to_string(),
            encrypted_content,
            encrypted_username,
        })
    }
}

impl EncryptLiveBlockVec for Vec<DecryptedLiveBlock> {
    fn encrypt_all(
        self,
        latest_document_key: &DecryptedDocumentKey,
    ) -> Result<Vec<LiveBlock>, String> {
        self.into_iter()
            .map(|block| block.encrypt(latest_document_key))
            .collect()
    }
}
