use crate::crypto::chacha::{decrypt, encrypt};
use crate::encryption::document::DecryptedDocumentKey;
use crate::stdb_bindings::DocumentVersionTag;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use spacetimedb_sdk::Identity;

/// Trait for decrypting a Vec of version tags
pub trait DecryptVersionTagVec {
    fn decrypt_all(
        self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<Vec<DecryptedVersionTag>, String>;
}

/// Trait for encrypting a Vec of decrypted version tags
#[allow(dead_code)]
pub trait EncryptVersionTagVec {
    fn encrypt_all(self, encryption_key: &[u8]) -> Result<Vec<DocumentVersionTag>, String>;
}

/// The inner content of a version tag (encrypted with document key)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecryptedVersionTagData {
    pub tag_name: String,
    pub timestamp: u128,
    pub created_by: Identity,
    pub created_at: u128,
}

/// A fully decrypted version tag
#[derive(Clone, Debug)]
pub struct DecryptedVersionTag {
    pub tag_id: String,
    pub doc_id: String,
    pub data: DecryptedVersionTagData,
}

impl DecryptedVersionTagData {
    /// Encrypt the tag data with a document key
    pub fn encrypt(&self, encryption_key: &[u8]) -> Result<Vec<u8>, String> {
        let blob = to_allocvec(self).map_err(|e| format!("Error serializing version tag: {e}"))?;

        encrypt(encryption_key, &blob).map_err(|e| format!("Error encrypting version tag: {e}"))
    }

    /// Decrypt tag data from encrypted blob
    pub fn from_encrypted(encrypted_blob: &[u8], encryption_key: &[u8]) -> Result<Self, String> {
        let decrypted = decrypt(encryption_key, encrypted_blob)
            .map_err(|e| format!("Error decrypting version tag: {e}"))?;

        from_bytes(&decrypted).map_err(|e| format!("Error deserializing version tag: {e}"))
    }
}

impl DocumentVersionTag {
    /// Decrypt this version tag using document keys
    /// Tries each key until one works (needed because timestamp is inside encrypted blob)
    pub fn decrypt(
        &self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<DecryptedVersionTag, String> {
        // Filter keys for this document and sort by timestamp (most recent first)
        let mut doc_keys: Vec<&DecryptedDocumentKey> = document_keys
            .iter()
            .filter(|k| k.doc_id == self.doc_id)
            .collect();

        doc_keys.sort_by(|a, b| b.key_timestamp.cmp(&a.key_timestamp));

        if doc_keys.is_empty() {
            return Err(format!("No keys found for document {}", self.doc_id));
        }

        // Try each key until one works
        for key in doc_keys {
            if let Ok(data) = DecryptedVersionTagData::from_encrypted(
                &self.encrypted_blob,
                &key.key_data.encryption_key,
            ) {
                return Ok(DecryptedVersionTag {
                    tag_id: self.tag_id.clone(),
                    doc_id: self.doc_id.clone(),
                    data,
                });
            }
        }

        Err(format!(
            "Failed to decrypt version tag {} with any available key",
            self.tag_id
        ))
    }
}

impl DecryptVersionTagVec for Vec<DocumentVersionTag> {
    fn decrypt_all(
        self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<Vec<DecryptedVersionTag>, String> {
        self.into_iter()
            .map(|tag| tag.decrypt(document_keys))
            .collect()
    }
}

impl DecryptedVersionTag {
    /// Encrypt this version tag with a document key
    pub fn encrypt(&self, encryption_key: &[u8]) -> Result<DocumentVersionTag, String> {
        let encrypted_blob = self.data.encrypt(encryption_key)?;

        Ok(DocumentVersionTag {
            tag_id: self.tag_id.clone(),
            doc_id: self.doc_id.clone(),
            encrypted_blob,
        })
    }
}

impl EncryptVersionTagVec for Vec<DecryptedVersionTag> {
    fn encrypt_all(self, encryption_key: &[u8]) -> Result<Vec<DocumentVersionTag>, String> {
        self.into_iter()
            .map(|tag| tag.encrypt(encryption_key))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encryption::document::DecryptedKeyData;
    use spacetimedb_sdk::Identity;

    fn create_test_document_key(doc_id: &str, key_timestamp: u128) -> DecryptedDocumentKey {
        DecryptedDocumentKey {
            key_id: format!("key_{}_{}", doc_id, key_timestamp),
            doc_id: doc_id.to_string(),
            user_id: Identity::from_byte_array([0u8; 32]),
            key_timestamp,
            key_data: DecryptedKeyData {
                encryption_key: vec![0u8; 32], // ChaCha20 requires 32-byte key
                signing_private_key: vec![0u8; 64],
            },
        }
    }

    #[test]
    fn test_version_tag_data_encrypt_decrypt_roundtrip() {
        let encryption_key = vec![0u8; 32];

        let original = DecryptedVersionTagData {
            tag_name: "v1.0".to_string(),
            timestamp: 1234567890,
            created_by: Identity::from_byte_array([1u8; 32]),
            created_at: 1234567890,
        };

        let encrypted = original
            .encrypt(&encryption_key)
            .expect("Encryption failed");

        let decrypted = DecryptedVersionTagData::from_encrypted(&encrypted, &encryption_key)
            .expect("Decryption failed");

        assert_eq!(decrypted.tag_name, original.tag_name);
        assert_eq!(decrypted.timestamp, original.timestamp);
        assert_eq!(decrypted.created_by, original.created_by);
        assert_eq!(decrypted.created_at, original.created_at);
    }

    #[test]
    fn test_document_version_tag_encrypt_decrypt_roundtrip() {
        let doc_id = "doc123";
        let document_key = create_test_document_key(doc_id, 0);
        let document_keys = vec![document_key.clone()];

        let decrypted = DecryptedVersionTag {
            tag_id: "tag123".to_string(),
            doc_id: doc_id.to_string(),
            data: DecryptedVersionTagData {
                tag_name: "v1.0".to_string(),
                timestamp: 100,
                created_by: Identity::from_byte_array([1u8; 32]),
                created_at: 100,
            },
        };

        let encrypted = decrypted
            .encrypt(&document_key.key_data.encryption_key)
            .expect("Encryption failed");

        let decrypted_again = encrypted
            .decrypt(&document_keys)
            .expect("Decryption failed");

        assert_eq!(decrypted_again.tag_id, "tag123");
        assert_eq!(decrypted_again.doc_id, doc_id);
        assert_eq!(decrypted_again.data.tag_name, "v1.0");
        assert_eq!(decrypted_again.data.timestamp, 100);
    }

    #[test]
    fn test_vec_version_tag_encrypt_decrypt() {
        let doc_id = "doc456";
        let document_key = create_test_document_key(doc_id, 0);
        let document_keys = vec![document_key.clone()];

        let decrypted_vec = vec![
            DecryptedVersionTag {
                tag_id: "tag1".to_string(),
                doc_id: doc_id.to_string(),
                data: DecryptedVersionTagData {
                    tag_name: "v1.0".to_string(),
                    timestamp: 100,
                    created_by: Identity::from_byte_array([1u8; 32]),
                    created_at: 100,
                },
            },
            DecryptedVersionTag {
                tag_id: "tag2".to_string(),
                doc_id: doc_id.to_string(),
                data: DecryptedVersionTagData {
                    tag_name: "v2.0".to_string(),
                    timestamp: 200,
                    created_by: Identity::from_byte_array([2u8; 32]),
                    created_at: 200,
                },
            },
        ];

        let encrypted_vec = decrypted_vec
            .encrypt_all(&document_key.key_data.encryption_key)
            .expect("Vec encryption failed");

        assert_eq!(encrypted_vec.len(), 2);

        let decrypted_again = encrypted_vec
            .decrypt_all(&document_keys)
            .expect("Vec decryption failed");

        assert_eq!(decrypted_again.len(), 2);
        assert_eq!(decrypted_again[0].tag_id, "tag1");
        assert_eq!(decrypted_again[1].tag_id, "tag2");
        assert_eq!(decrypted_again[0].data.tag_name, "v1.0");
        assert_eq!(decrypted_again[1].data.tag_name, "v2.0");
    }

    #[test]
    fn test_version_tag_decrypt_with_multiple_keys() {
        let doc_id = "doc_multikey";

        // Two keys for same document with different timestamps
        let mut key_old = create_test_document_key(doc_id, 50);
        key_old.key_data.encryption_key = vec![1u8; 32];

        let mut key_new = create_test_document_key(doc_id, 150);
        key_new.key_data.encryption_key = vec![2u8; 32];

        let document_keys = vec![key_old.clone(), key_new.clone()];

        // Tag encrypted with old key should decrypt
        let tag_old = DecryptedVersionTag {
            tag_id: "tag_old".to_string(),
            doc_id: doc_id.to_string(),
            data: DecryptedVersionTagData {
                tag_name: "v1.0".to_string(),
                timestamp: 100,
                created_by: Identity::from_byte_array([1u8; 32]),
                created_at: 100,
            },
        };
        let encrypted_old = tag_old
            .encrypt(&key_old.key_data.encryption_key)
            .expect("Encryption failed");
        let decrypted_old = encrypted_old
            .decrypt(&document_keys)
            .expect("Decryption failed");
        assert_eq!(decrypted_old.data.tag_name, "v1.0");

        // Tag encrypted with new key should also decrypt
        let tag_new = DecryptedVersionTag {
            tag_id: "tag_new".to_string(),
            doc_id: doc_id.to_string(),
            data: DecryptedVersionTagData {
                tag_name: "v2.0".to_string(),
                timestamp: 200,
                created_by: Identity::from_byte_array([1u8; 32]),
                created_at: 200,
            },
        };
        let encrypted_new = tag_new
            .encrypt(&key_new.key_data.encryption_key)
            .expect("Encryption failed");
        let decrypted_new = encrypted_new
            .decrypt(&document_keys)
            .expect("Decryption failed");
        assert_eq!(decrypted_new.data.tag_name, "v2.0");
    }

    #[test]
    fn test_version_tag_decrypt_fails_with_wrong_key() {
        let document_key = create_test_document_key("doc1", 0);
        let wrong_keys = vec![create_test_document_key("doc2", 0)]; // Different doc_id

        let tag = DecryptedVersionTag {
            tag_id: "tag1".to_string(),
            doc_id: "doc1".to_string(),
            data: DecryptedVersionTagData {
                tag_name: "v1.0".to_string(),
                timestamp: 100,
                created_by: Identity::from_byte_array([1u8; 32]),
                created_at: 100,
            },
        };

        let encrypted = tag
            .encrypt(&document_key.key_data.encryption_key)
            .expect("Encryption failed");
        let result = encrypted.decrypt(&wrong_keys);

        assert!(result.is_err());
    }

    #[test]
    fn test_empty_vec_encrypt_decrypt() {
        let encryption_key = vec![0u8; 32];
        let document_keys: Vec<DecryptedDocumentKey> = vec![];

        let empty: Vec<DecryptedVersionTag> = vec![];
        let encrypted = empty
            .encrypt_all(&encryption_key)
            .expect("Empty vec encryption failed");
        assert!(encrypted.is_empty());

        let empty_encrypted: Vec<DocumentVersionTag> = vec![];
        let decrypted = empty_encrypted
            .decrypt_all(&document_keys)
            .expect("Empty vec decryption failed");
        assert!(decrypted.is_empty());
    }
}
