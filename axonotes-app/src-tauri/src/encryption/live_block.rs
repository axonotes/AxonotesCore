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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::document::{DecryptedDocumentKey, DecryptedKeyData};
    use spacetimedb_sdk::Identity;

    fn create_test_identity() -> Identity {
        Identity::from_byte_array([0u8; 32])
    }

    fn create_test_document_key(doc_id: &str, key_timestamp: u128) -> DecryptedDocumentKey {
        DecryptedDocumentKey {
            key_id: format!("key_{}_{}", doc_id, key_timestamp),
            doc_id: doc_id.to_string(),
            user_id: create_test_identity(),
            key_timestamp,
            key_data: DecryptedKeyData {
                encryption_key: vec![0u8; 32],
                signing_private_key: vec![0u8; 32],
            },
        }
    }

    #[test]
    fn test_live_block_encrypt_decrypt_roundtrip() {
        let doc_id = "doc123";
        let document_key = create_test_document_key(doc_id, 0);
        let document_keys = vec![document_key.clone()];
        let identity = create_test_identity();

        let original = DecryptedLiveBlock {
            live_block_id: "lb1".to_string(),
            doc_id: doc_id.to_string(),
            block_id: 42,
            user_id: identity,
            content: vec![1, 2, 3, 4, 5],
            username: "alice".to_string(),
            locked_at: Some(100),
        };

        let encrypted = original.encrypt(&document_key).expect("Encryption failed");

        // Verify unencrypted fields preserved
        assert_eq!(encrypted.live_block_id, "lb1");
        assert_eq!(encrypted.doc_id, doc_id);
        assert_eq!(encrypted.block_id, 42);
        assert_eq!(encrypted.locked_at, Some(100));
        // Encrypted fields should be non-empty
        assert!(!encrypted.encrypted_content.is_empty());
        assert!(!encrypted.encrypted_username.is_empty());

        let decrypted = encrypted
            .decrypt(&document_keys)
            .expect("Decryption failed");

        assert_eq!(decrypted.live_block_id, original.live_block_id);
        assert_eq!(decrypted.doc_id, original.doc_id);
        assert_eq!(decrypted.block_id, original.block_id);
        assert_eq!(decrypted.content, vec![1, 2, 3, 4, 5]);
        assert_eq!(decrypted.username, "alice");
        assert_eq!(decrypted.locked_at, Some(100));
    }

    #[test]
    fn test_live_block_with_empty_content() {
        let doc_id = "doc_empty";
        let document_key = create_test_document_key(doc_id, 0);
        let document_keys = vec![document_key.clone()];

        let original = DecryptedLiveBlock {
            live_block_id: "lb_empty".to_string(),
            doc_id: doc_id.to_string(),
            block_id: 0,
            user_id: create_test_identity(),
            content: vec![],
            username: "bob".to_string(),
            locked_at: Some(50),
        };

        let encrypted = original.encrypt(&document_key).expect("Encryption failed");
        let decrypted = encrypted
            .decrypt(&document_keys)
            .expect("Decryption failed");

        assert!(decrypted.content.is_empty());
        assert_eq!(decrypted.username, "bob");
    }

    #[test]
    fn test_live_block_with_unicode_username() {
        let doc_id = "doc_unicode";
        let document_key = create_test_document_key(doc_id, 0);
        let document_keys = vec![document_key.clone()];

        let original = DecryptedLiveBlock {
            live_block_id: "lb_unicode".to_string(),
            doc_id: doc_id.to_string(),
            block_id: 1,
            user_id: create_test_identity(),
            content: vec![255, 254, 253],
            username: "用户🚀".to_string(),
            locked_at: Some(100),
        };

        let encrypted = original.encrypt(&document_key).expect("Encryption failed");
        let decrypted = encrypted
            .decrypt(&document_keys)
            .expect("Decryption failed");

        assert_eq!(decrypted.username, "用户🚀");
    }

    #[test]
    fn test_vec_live_block_encrypt_decrypt() {
        let doc_id = "doc456";
        let document_key = create_test_document_key(doc_id, 0);
        let document_keys = vec![document_key.clone()];
        let identity = create_test_identity();

        let blocks = vec![
            DecryptedLiveBlock {
                live_block_id: "lb1".to_string(),
                doc_id: doc_id.to_string(),
                block_id: 1,
                user_id: identity,
                content: vec![1, 2, 3],
                username: "user1".to_string(),
                locked_at: Some(100),
            },
            DecryptedLiveBlock {
                live_block_id: "lb2".to_string(),
                doc_id: doc_id.to_string(),
                block_id: 2,
                user_id: identity,
                content: vec![4, 5, 6],
                username: "user2".to_string(),
                locked_at: Some(200),
            },
        ];

        let encrypted = blocks
            .encrypt_all(&document_key)
            .expect("Vec encryption failed");
        assert_eq!(encrypted.len(), 2);

        let decrypted = encrypted
            .decrypt_all(&document_keys)
            .expect("Vec decryption failed");
        assert_eq!(decrypted.len(), 2);
        assert_eq!(decrypted[0].live_block_id, "lb1");
        assert_eq!(decrypted[1].live_block_id, "lb2");
        assert_eq!(decrypted[0].username, "user1");
        assert_eq!(decrypted[1].username, "user2");
    }

    #[test]
    fn test_empty_vec_live_block_encrypt_decrypt() {
        let document_key = create_test_document_key("doc", 0);
        let document_keys = vec![document_key.clone()];

        let empty: Vec<DecryptedLiveBlock> = vec![];
        let encrypted = empty
            .encrypt_all(&document_key)
            .expect("Empty encrypt failed");
        assert!(encrypted.is_empty());

        let empty_encrypted: Vec<LiveBlock> = vec![];
        let decrypted = empty_encrypted
            .decrypt_all(&document_keys)
            .expect("Empty decrypt failed");
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_decrypt_selects_correct_key_by_locked_at() {
        let doc_id = "doc_multikey";

        let mut key_old = create_test_document_key(doc_id, 50);
        key_old.key_data.encryption_key = vec![1u8; 32];

        let mut key_new = create_test_document_key(doc_id, 150);
        key_new.key_data.encryption_key = vec![2u8; 32];

        let document_keys = vec![key_old.clone(), key_new.clone()];
        let identity = create_test_identity();

        // Block with locked_at=100 should use key_old (50 <= 100 < 150)
        let block_old = DecryptedLiveBlock {
            live_block_id: "lb_old".to_string(),
            doc_id: doc_id.to_string(),
            block_id: 1,
            user_id: identity,
            content: vec![1],
            username: "old".to_string(),
            locked_at: Some(100),
        };
        let encrypted_old = block_old.encrypt(&key_old).expect("Encryption failed");
        let decrypted_old = encrypted_old
            .decrypt(&document_keys)
            .expect("Decryption failed");
        assert_eq!(decrypted_old.username, "old");

        // Block with locked_at=200 should use key_new (150 <= 200)
        let block_new = DecryptedLiveBlock {
            live_block_id: "lb_new".to_string(),
            doc_id: doc_id.to_string(),
            block_id: 2,
            user_id: identity,
            content: vec![2],
            username: "new".to_string(),
            locked_at: Some(200),
        };
        let encrypted_new = block_new.encrypt(&key_new).expect("Encryption failed");
        let decrypted_new = encrypted_new
            .decrypt(&document_keys)
            .expect("Decryption failed");
        assert_eq!(decrypted_new.username, "new");
    }

    #[test]
    fn test_decrypt_fails_with_wrong_doc_key() {
        let document_key = create_test_document_key("doc1", 0);
        let wrong_keys = vec![create_test_document_key("doc2", 0)];

        let block = DecryptedLiveBlock {
            live_block_id: "lb1".to_string(),
            doc_id: "doc1".to_string(),
            block_id: 1,
            user_id: create_test_identity(),
            content: vec![1, 2, 3],
            username: "user".to_string(),
            locked_at: Some(100),
        };

        let encrypted = block.encrypt(&document_key).expect("Encryption failed");
        let result = encrypted.decrypt(wrong_keys.as_slice());

        assert!(result.is_err());
    }
}
