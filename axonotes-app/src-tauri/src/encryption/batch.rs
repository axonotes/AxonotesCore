//! # Batch Encryption and Decryption
//!
//! Handles encryption and decryption of document batches (groups of patches).
//!
//! ## Batch Structure
//!
//! A batch contains patches for a specific block, encrypted with the document's
//! current encryption key. The key is selected based on the batch timestamp.
//!
//! ## Initial Batches
//!
//! When a batch's timestamp equals its key's timestamp, it's marked as "initial".
//! Initial batches represent the starting state after a key rotation.

use crate::crypto::chacha::{decrypt, encrypt};
use crate::encryption::document::DecryptedDocumentKey;
use crate::encryption::helpers::find_correct_decryption_key;
use crate::stdb_bindings::DocumentBatch;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};

/// Trait for decrypting a vector of document batches.
pub trait DecryptDocumentBatchVec {
    /// Decrypts all batches using the appropriate key from the provided key set.
    fn decrypt_all(
        self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<Vec<DecryptedBatch>, String>;
}

/// Trait for encrypting a vector of decrypted batches.
#[allow(dead_code)]
pub trait EncryptDocumentBatchVec {
    /// Encrypts all batches with the provided document key.
    fn encrypt_all(
        self,
        latest_document_key: &DecryptedDocumentKey,
    ) -> Result<Vec<DocumentBatch>, String>;
}

/// A single change operation within a batch.
///
/// Patches are applied sequentially to reconstruct block state.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Patch {
    /// The binary delta (change) data
    pub delta: Vec<u8>,
    /// Time offset in seconds from batch timestamp
    pub time_delta: u8,
}

/// The decrypted payload of a batch.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BatchData {
    /// The block this batch applies to
    pub block_id: u64,
    /// Ordered list of patches to apply
    pub patches: Vec<Patch>,
}

/// A decrypted document batch ready for processing.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecryptedBatch {
    /// True if this is an initial batch (snapshot) after key rotation
    pub is_initial: bool,
    /// Unique batch identifier
    pub batch_id: String,
    /// Document this batch belongs to
    pub doc_id: String,
    /// Timestamp for ordering (microseconds since epoch)
    pub timestamp: u128,
    /// The decrypted batch payload
    pub batch_data: BatchData,
}

impl DocumentBatch {
    pub fn decrypt(
        &self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<DecryptedBatch, String> {
        let batch_timestamp = self.timestamp;

        let decryption_key: &DecryptedDocumentKey =
            find_correct_decryption_key(&self.doc_id, batch_timestamp, document_keys)?;

        // Decrypt the batch data blob with this key
        let decrypted = decrypt(
            decryption_key.key_data.encryption_key.as_slice(),
            self.encrypted_data.as_slice(),
        )
        .map_err(|e| format!("Error while decrypting batch data: {e}"))?;

        let batch_data: BatchData = from_bytes(decrypted.as_slice())
            .map_err(|e| format!("Error deserializing batch data: {e}"))?;

        let is_initial = batch_timestamp == decryption_key.key_timestamp;

        Ok(DecryptedBatch {
            timestamp: self.timestamp,
            doc_id: self.doc_id.clone(),
            batch_id: self.batch_id.clone(),
            is_initial,
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
            .map_err(|e| format!("Error serializing batch data: {e}"))?;

        let encrypted_data = encrypt(
            latest_document_key.key_data.encryption_key.as_slice(),
            blob.as_slice(),
        )
        .map_err(|e| format!("Error when encrypting batch data: {e}"))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::document::{DecryptedDocumentKey, DecryptedKeyData};
    use spacetimedb_sdk::Identity;

    fn create_test_document_key(doc_id: &str, key_timestamp: u128) -> DecryptedDocumentKey {
        DecryptedDocumentKey {
            key_id: format!("key_{}_{}", doc_id, key_timestamp),
            doc_id: doc_id.to_string(),
            user_id: Identity::from_byte_array([0u8; 32]),
            key_timestamp,
            key_data: DecryptedKeyData {
                encryption_key: vec![0u8; 32], // ChaCha20 requires 32-byte key
                signing_private_key: vec![0u8; 32],
            },
        }
    }

    #[test]
    fn test_batch_encrypt_decrypt_roundtrip() {
        let doc_id = "doc123";
        let document_key = create_test_document_key(doc_id, 0);
        let document_keys = vec![document_key.clone()];

        let original = DecryptedBatch {
            is_initial: false,
            batch_id: "batch1".to_string(),
            doc_id: doc_id.to_string(),
            timestamp: 100,
            batch_data: BatchData {
                block_id: 42,
                patches: vec![
                    Patch {
                        delta: vec![1, 2, 3, 4, 5],
                        time_delta: 10,
                    },
                    Patch {
                        delta: vec![6, 7, 8],
                        time_delta: 20,
                    },
                ],
            },
        };

        let encrypted = original.encrypt(&document_key).expect("Encryption failed");

        // Verify unencrypted fields are preserved
        assert_eq!(encrypted.batch_id, "batch1");
        assert_eq!(encrypted.doc_id, doc_id);
        assert_eq!(encrypted.timestamp, 100);
        // encrypted_data should be non-empty and different from original
        assert!(!encrypted.encrypted_data.is_empty());

        let decrypted = encrypted
            .decrypt(&document_keys)
            .expect("Decryption failed");

        assert_eq!(decrypted.batch_id, original.batch_id);
        assert_eq!(decrypted.doc_id, original.doc_id);
        assert_eq!(decrypted.timestamp, original.timestamp);
        assert!(!decrypted.is_initial); // timestamp 100 != key_timestamp 0
        assert_eq!(decrypted.batch_data.block_id, 42);
        assert_eq!(decrypted.batch_data.patches.len(), 2);
        assert_eq!(decrypted.batch_data.patches[0].delta, vec![1, 2, 3, 4, 5]);
        assert_eq!(decrypted.batch_data.patches[0].time_delta, 10);
        assert_eq!(decrypted.batch_data.patches[1].delta, vec![6, 7, 8]);
        assert_eq!(decrypted.batch_data.patches[1].time_delta, 20);
    }

    #[test]
    fn test_batch_with_empty_patches() {
        let doc_id = "doc_empty";
        let document_key = create_test_document_key(doc_id, 0);
        let document_keys = vec![document_key.clone()];

        let original = DecryptedBatch {
            is_initial: false,
            batch_id: "batch_empty".to_string(),
            doc_id: doc_id.to_string(),
            timestamp: 50,
            batch_data: BatchData {
                block_id: 0,
                patches: vec![],
            },
        };

        let encrypted = original.encrypt(&document_key).expect("Encryption failed");
        let decrypted = encrypted
            .decrypt(&document_keys)
            .expect("Decryption failed");

        assert_eq!(decrypted.batch_data.block_id, 0);
        assert!(decrypted.batch_data.patches.is_empty());
        assert!(!decrypted.is_initial); // timestamp 50 != key_timestamp 0
    }

    #[test]
    fn test_vec_batch_encrypt_decrypt() {
        let doc_id = "doc456";
        let document_key = create_test_document_key(doc_id, 0);
        let document_keys = vec![document_key.clone()];

        let batches = vec![
            DecryptedBatch {
                is_initial: false,
                batch_id: "batch1".to_string(),
                doc_id: doc_id.to_string(),
                timestamp: 100,
                batch_data: BatchData {
                    block_id: 1,
                    patches: vec![],
                },
            },
            DecryptedBatch {
                is_initial: false,
                batch_id: "batch2".to_string(),
                doc_id: doc_id.to_string(),
                timestamp: 200,
                batch_data: BatchData {
                    block_id: 2,
                    patches: vec![Patch {
                        delta: vec![1, 2, 3],
                        time_delta: 5,
                    }],
                },
            },
        ];

        let encrypted = batches
            .encrypt_all(&document_key)
            .expect("Vec encryption failed");
        assert_eq!(encrypted.len(), 2);

        let decrypted = encrypted
            .decrypt_all(&document_keys)
            .expect("Vec decryption failed");
        assert_eq!(decrypted.len(), 2);
        assert_eq!(decrypted[0].batch_id, "batch1");
        assert_eq!(decrypted[1].batch_id, "batch2");
        assert!(!decrypted[0].is_initial);
        assert!(!decrypted[1].is_initial);
        assert!(decrypted[0].batch_data.patches.is_empty());
        assert_eq!(decrypted[1].batch_data.patches.len(), 1);
    }

    #[test]
    fn test_empty_vec_batch_encrypt_decrypt() {
        let document_key = create_test_document_key("doc", 0);
        let document_keys = vec![document_key.clone()];

        let empty: Vec<DecryptedBatch> = vec![];
        let encrypted = empty
            .encrypt_all(&document_key)
            .expect("Empty encrypt failed");
        assert!(encrypted.is_empty());

        let empty_encrypted: Vec<DocumentBatch> = vec![];
        let decrypted = empty_encrypted
            .decrypt_all(&document_keys)
            .expect("Empty decrypt failed");
        assert!(decrypted.is_empty());
    }

    #[test]
    fn test_decrypt_selects_correct_key_by_timestamp() {
        let doc_id = "doc_multikey";

        // Two keys for same document with different timestamps and different encryption keys
        let mut key_old = create_test_document_key(doc_id, 50);
        key_old.key_data.encryption_key = vec![1u8; 32];

        let mut key_new = create_test_document_key(doc_id, 150);
        key_new.key_data.encryption_key = vec![2u8; 32];

        let document_keys = vec![key_old.clone(), key_new.clone()];

        // Batch at timestamp 100 should decrypt with key_old (timestamp 50 <= 100)
        let batch_old = DecryptedBatch {
            is_initial: false,
            batch_id: "batch_old".to_string(),
            doc_id: doc_id.to_string(),
            timestamp: 100,
            batch_data: BatchData {
                block_id: 1,
                patches: vec![],
            },
        };
        let encrypted_old = batch_old.encrypt(&key_old).expect("Encryption failed");
        let decrypted_old = encrypted_old
            .decrypt(&document_keys)
            .expect("Decryption failed");
        assert_eq!(decrypted_old.batch_id, "batch_old");
        assert!(!decrypted_old.is_initial); // timestamp 100 != key_timestamp 50

        // Batch at timestamp 200 should decrypt with key_new (timestamp 150 <= 200)
        let batch_new = DecryptedBatch {
            is_initial: false,
            batch_id: "batch_new".to_string(),
            doc_id: doc_id.to_string(),
            timestamp: 200,
            batch_data: BatchData {
                block_id: 2,
                patches: vec![],
            },
        };
        let encrypted_new = batch_new.encrypt(&key_new).expect("Encryption failed");
        let decrypted_new = encrypted_new
            .decrypt(&document_keys)
            .expect("Decryption failed");
        assert_eq!(decrypted_new.batch_id, "batch_new");
        assert!(!decrypted_new.is_initial); // timestamp 200 != key_timestamp 150
    }

    #[test]
    fn test_decrypt_fails_with_wrong_key() {
        let document_key = create_test_document_key("doc1", 0);
        let wrong_keys = vec![create_test_document_key("doc2", 0)]; // Different doc_id

        let batch = DecryptedBatch {
            is_initial: false,
            batch_id: "batch1".to_string(),
            doc_id: "doc1".to_string(),
            timestamp: 100,
            batch_data: BatchData {
                block_id: 1,
                patches: vec![],
            },
        };

        let encrypted = batch.encrypt(&document_key).expect("Encryption failed");
        let result = encrypted.decrypt(wrong_keys.as_slice());

        assert!(result.is_err());
    }

    #[test]
    fn test_is_initial_true_when_timestamp_equals_key_timestamp() {
        let doc_id = "doc_initial";
        let key_timestamp = 100;
        let document_key = create_test_document_key(doc_id, key_timestamp);
        let document_keys = vec![document_key.clone()];

        // Initial batch: timestamp matches key_timestamp
        let initial_batch = DecryptedBatch {
            is_initial: true,
            batch_id: "initial".to_string(),
            doc_id: doc_id.to_string(),
            timestamp: key_timestamp, // Same as key!
            batch_data: BatchData {
                block_id: 0,
                patches: vec![],
            },
        };

        let encrypted = initial_batch
            .encrypt(&document_key)
            .expect("Encryption failed");
        let decrypted = encrypted
            .decrypt(&document_keys)
            .expect("Decryption failed");

        assert!(decrypted.is_initial); // timestamp 100 == key_timestamp 100
        assert_eq!(decrypted.batch_id, "initial");
    }

    #[test]
    fn test_is_initial_with_multiple_keys() {
        let doc_id = "doc_multi_initial";

        let mut key1 = create_test_document_key(doc_id, 100);
        key1.key_data.encryption_key = vec![1u8; 32];

        let mut key2 = create_test_document_key(doc_id, 200);
        key2.key_data.encryption_key = vec![2u8; 32];

        let document_keys = vec![key1.clone(), key2.clone()];

        // Initial batch for key1
        let initial1 = DecryptedBatch {
            is_initial: true,
            batch_id: "initial1".to_string(),
            doc_id: doc_id.to_string(),
            timestamp: 100,
            batch_data: BatchData {
                block_id: 0,
                patches: vec![],
            },
        };
        let encrypted1 = initial1.encrypt(&key1).expect("Encryption failed");
        let decrypted1 = encrypted1
            .decrypt(&document_keys)
            .expect("Decryption failed");
        assert!(decrypted1.is_initial);

        // Non-initial batch for key1
        let regular1 = DecryptedBatch {
            is_initial: false,
            batch_id: "regular1".to_string(),
            doc_id: doc_id.to_string(),
            timestamp: 150,
            batch_data: BatchData {
                block_id: 1,
                patches: vec![],
            },
        };
        let encrypted_r1 = regular1.encrypt(&key1).expect("Encryption failed");
        let decrypted_r1 = encrypted_r1
            .decrypt(&document_keys)
            .expect("Decryption failed");
        assert!(!decrypted_r1.is_initial);

        // Initial batch for key2
        let initial2 = DecryptedBatch {
            is_initial: true,
            batch_id: "initial2".to_string(),
            doc_id: doc_id.to_string(),
            timestamp: 200,
            batch_data: BatchData {
                block_id: 0,
                patches: vec![],
            },
        };
        let encrypted2 = initial2.encrypt(&key2).expect("Encryption failed");
        let decrypted2 = encrypted2
            .decrypt(&document_keys)
            .expect("Decryption failed");
        assert!(decrypted2.is_initial);

        // Non-initial batch for key2
        let regular2 = DecryptedBatch {
            is_initial: false,
            batch_id: "regular2".to_string(),
            doc_id: doc_id.to_string(),
            timestamp: 250,
            batch_data: BatchData {
                block_id: 2,
                patches: vec![],
            },
        };
        let encrypted_r2 = regular2.encrypt(&key2).expect("Encryption failed");
        let decrypted_r2 = encrypted_r2
            .decrypt(&document_keys)
            .expect("Decryption failed");
        assert!(!decrypted_r2.is_initial);
    }
}
