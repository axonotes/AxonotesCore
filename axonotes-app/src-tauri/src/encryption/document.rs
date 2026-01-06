use crate::crypto::x25519::{decrypt_from_anyone, encrypt_for_recipient};
use crate::stdb_bindings::{DocumentKey, DocumentMetadata, Role};
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use spacetimedb_sdk::Identity;

pub trait DecryptDocumentMetaAndKeyVec {
    type Output;
    fn decrypt_all(self, private_key: &[u8; 32]) -> Result<Self::Output, String>;
}

#[allow(dead_code)]
pub trait EncryptDocumentMetadataVec {
    type Output;
    fn encrypt_all(
        self,
        my_private_key: &[u8; 32],
        my_public_key: &[u8; 32],
    ) -> Result<Self::Output, String>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecryptedMetadata {
    pub version: u16,
    pub path: String,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecryptedDocumentMetadata {
    pub meta_id: String,
    pub user_id: Identity,
    pub doc_id: String,
    pub metadata: DecryptedMetadata,
}

impl DecryptedMetadata {
    pub fn encrypt(
        &self,
        my_private_key: &[u8; 32],
        my_public_key: &[u8; 32],
        their_public_key: &[u8; 32],
    ) -> Result<Vec<u8>, String> {
        let blob = to_allocvec(self).map_err(|e| format!("Error serializing metadata: {e}"))?;

        encrypt_for_recipient(
            my_private_key,
            my_public_key,
            their_public_key,
            blob.as_slice(),
        )
        .map_err(|e| format!("Error when encrypting metadata: {e}"))
    }

    pub fn from_encrypted(encrypted_blob: &[u8], private_key: &[u8; 32]) -> Result<Self, String> {
        let (decrypted_blob, _): (Vec<u8>, [u8; 32]) =
            decrypt_from_anyone(private_key, encrypted_blob)
                .map_err(|e| format!("Error when decrypting metadata: {e}"))?;

        from_bytes(decrypted_blob.as_slice())
            .map_err(|e| format!("Error deserializing metadata: {e}"))
    }
}

impl DocumentMetadata {
    pub fn decrypt(self, private_key: &[u8; 32]) -> Result<DecryptedDocumentMetadata, String> {
        let metadata =
            DecryptedMetadata::from_encrypted(self.encrypted_blob.as_slice(), private_key)?;

        Ok(DecryptedDocumentMetadata {
            doc_id: self.doc_id,
            user_id: self.user_id,
            meta_id: self.meta_id,
            metadata,
        })
    }
}

impl DecryptDocumentMetaAndKeyVec for Vec<DocumentMetadata> {
    type Output = Vec<DecryptedDocumentMetadata>;

    fn decrypt_all(self, private_key: &[u8; 32]) -> Result<Self::Output, String> {
        self.into_iter()
            .map(|meta| meta.decrypt(private_key))
            .collect()
    }
}

impl DecryptedDocumentMetadata {
    #[allow(dead_code)]
    pub fn encrypt(
        self,
        private_key: &[u8; 32],
        public_key: &[u8; 32],
    ) -> Result<DocumentMetadata, String> {
        let encrypted_blob = self.metadata.encrypt(private_key, public_key, public_key)?;

        Ok(DocumentMetadata {
            doc_id: self.doc_id,
            user_id: self.user_id,
            meta_id: self.meta_id,
            encrypted_blob,
        })
    }
}

impl EncryptDocumentMetadataVec for Vec<DecryptedDocumentMetadata> {
    type Output = Vec<DocumentMetadata>;

    fn encrypt_all(
        self,
        my_private_key: &[u8; 32],
        my_public_key: &[u8; 32],
    ) -> Result<Self::Output, String> {
        self.into_iter()
            .map(|meta| meta.encrypt(my_private_key, my_public_key))
            .collect()
    }
}

#[allow(dead_code)]
pub trait EncryptDocumentKeyVec {
    fn encrypt_all(
        self,
        my_private_key: &[u8; 32],
        my_public_key: &[u8; 32],
        their_public_key: &[u8; 32],
    ) -> Result<Vec<DocumentKey>, String>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecryptedKeyData {
    pub encryption_key: Vec<u8>,
    pub signing_private_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DecryptedDocumentKey {
    pub key_id: String,
    pub doc_id: String,
    pub user_id: Identity,
    pub key_timestamp: u128,
    pub key_data: DecryptedKeyData,
}

impl DecryptedKeyData {
    pub fn encrypt(
        &self,
        my_private_key: &[u8; 32],
        my_public_key: &[u8; 32],
        their_public_key: &[u8; 32],
    ) -> Result<Vec<u8>, String> {
        let blob = to_allocvec(self).map_err(|e| format!("Error serializing key data: {e}"))?;

        encrypt_for_recipient(
            my_private_key,
            my_public_key,
            their_public_key,
            blob.as_slice(),
        )
        .map_err(|e| format!("Error when encrypting key data: {e}"))
    }

    /// Encrypt key data for a specific role
    /// - Readers get empty signing_private_key (can only decrypt, not sign)
    /// - Editors/Owners get full signing_private_key
    pub fn encrypt_for_role(
        &self,
        role: Role,
        my_private_key: &[u8; 32],
        my_public_key: &[u8; 32],
        their_public_key: &[u8; 32],
    ) -> Result<Vec<u8>, String> {
        let key_data = match role {
            Role::Reader => DecryptedKeyData {
                encryption_key: self.encryption_key.clone(),
                signing_private_key: vec![], // Readers don't get signing key
            },
            Role::Editor | Role::Owner => self.clone(),
        };

        key_data.encrypt(my_private_key, my_public_key, their_public_key)
    }

    pub fn from_encrypted(encrypted_blob: &[u8], private_key: &[u8; 32]) -> Result<Self, String> {
        let (decrypted_blob, _): (Vec<u8>, [u8; 32]) =
            decrypt_from_anyone(private_key, encrypted_blob)
                .map_err(|e| format!("Error when decrypting key data: {e}"))?;

        from_bytes(decrypted_blob.as_slice())
            .map_err(|e| format!("Error deserializing key data: {e}"))
    }
}

impl DocumentKey {
    pub fn decrypt(self, private_key: &[u8; 32]) -> Result<DecryptedDocumentKey, String> {
        let key_data =
            DecryptedKeyData::from_encrypted(self.encrypted_data.as_slice(), private_key)?;

        Ok(DecryptedDocumentKey {
            key_id: self.key_id,
            doc_id: self.doc_id,
            user_id: self.user_id,
            key_timestamp: self.key_timestamp,
            key_data,
        })
    }
}

impl DecryptDocumentMetaAndKeyVec for Vec<DocumentKey> {
    type Output = Vec<DecryptedDocumentKey>;

    fn decrypt_all(self, private_key: &[u8; 32]) -> Result<Self::Output, String> {
        self.into_iter()
            .map(|key| key.decrypt(private_key))
            .collect()
    }
}

impl DecryptedDocumentKey {
    #[allow(dead_code)]
    pub fn encrypt(
        self,
        my_private_key: &[u8; 32],
        my_public_key: &[u8; 32],
        their_public_key: &[u8; 32],
    ) -> Result<DocumentKey, String> {
        let encrypted_data =
            self.key_data
                .encrypt(my_private_key, my_public_key, their_public_key)?;

        Ok(DocumentKey {
            key_id: self.key_id,
            doc_id: self.doc_id,
            user_id: self.user_id,
            key_timestamp: self.key_timestamp,
            encrypted_data,
        })
    }
}

impl EncryptDocumentKeyVec for Vec<DecryptedDocumentKey> {
    fn encrypt_all(
        self,
        my_private_key: &[u8; 32],
        my_public_key: &[u8; 32],
        their_public_key: &[u8; 32],
    ) -> Result<Vec<DocumentKey>, String> {
        self.into_iter()
            .map(|key| key.encrypt(my_private_key, my_public_key, their_public_key))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::ed25519::generate_ed25519_keys;
    use spacetimedb_sdk::Identity;

    fn create_test_keys() -> ([u8; 32], [u8; 32]) {
        generate_ed25519_keys()
    }

    fn create_test_identity() -> Identity {
        Identity::from_byte_array([0u8; 32])
    }

    #[test]
    fn test_metadata_encrypt_decrypt_roundtrip() {
        let (private_key, public_key) = create_test_keys();

        let original = DecryptedMetadata {
            version: 1,
            path: "/test/document.doc".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };

        let encrypted = original
            .encrypt(&private_key, &public_key, &public_key)
            .expect("Encryption failed");

        let decrypted = DecryptedMetadata::from_encrypted(encrypted.as_slice(), &private_key)
            .expect("Decryption failed");

        assert_eq!(decrypted.version, original.version);
        assert_eq!(decrypted.path, original.path);
        assert_eq!(decrypted.tags, original.tags);
    }

    #[test]
    fn test_document_metadata_encrypt_decrypt_roundtrip() {
        let (private_key, public_key) = create_test_keys();
        let identity = create_test_identity();

        let decrypted = DecryptedDocumentMetadata {
            meta_id: "meta123".to_string(),
            user_id: identity,
            doc_id: "doc456".to_string(),
            metadata: DecryptedMetadata {
                version: 1,
                path: "/test.doc".to_string(),
                tags: vec!["important".to_string()],
            },
        };

        let encrypted = decrypted
            .encrypt(&private_key, &public_key)
            .expect("Encryption failed");

        let decrypted_again = encrypted.decrypt(&private_key).expect("Decryption failed");

        assert_eq!(decrypted_again.meta_id, "meta123");
        assert_eq!(decrypted_again.doc_id, "doc456");
        assert_eq!(decrypted_again.metadata.version, 1);
        assert_eq!(decrypted_again.metadata.path, "/test.doc");
        assert_eq!(decrypted_again.metadata.tags.len(), 1);
    }

    #[test]
    fn test_vec_document_metadata_encrypt_decrypt() {
        let (private_key, public_key) = create_test_keys();
        let identity = create_test_identity();

        let decrypted_vec = vec![
            DecryptedDocumentMetadata {
                meta_id: "meta1".to_string(),
                user_id: identity,
                doc_id: "doc1".to_string(),
                metadata: DecryptedMetadata {
                    version: 1,
                    path: "/doc1.doc".to_string(),
                    tags: vec![],
                },
            },
            DecryptedDocumentMetadata {
                meta_id: "meta2".to_string(),
                user_id: identity,
                doc_id: "doc2".to_string(),
                metadata: DecryptedMetadata {
                    version: 1,
                    path: "/doc2.doc".to_string(),
                    tags: vec!["tag1".to_string()],
                },
            },
        ];

        let encrypted_vec = decrypted_vec
            .encrypt_all(&private_key, &public_key)
            .expect("Vec encryption failed");

        assert_eq!(encrypted_vec.len(), 2);

        let decrypted_again = encrypted_vec
            .decrypt_all(&private_key)
            .expect("Vec decryption failed");

        assert_eq!(decrypted_again.len(), 2);
        assert_eq!(decrypted_again[0].doc_id, "doc1");
        assert_eq!(decrypted_again[1].doc_id, "doc2");
        assert_eq!(decrypted_again[1].metadata.tags.len(), 1);
    }

    #[test]
    fn test_key_data_encrypt_decrypt_roundtrip() {
        let (private_key, public_key) = create_test_keys();

        let original = DecryptedKeyData {
            encryption_key: vec![1, 2, 3, 4, 5],
            signing_private_key: vec![6, 7, 8, 9, 10],
        };

        let encrypted = original
            .encrypt(&private_key, &public_key, &public_key)
            .expect("Encryption failed");

        let decrypted = DecryptedKeyData::from_encrypted(encrypted.as_slice(), &private_key)
            .expect("Decryption failed");

        assert_eq!(decrypted.encryption_key, original.encryption_key);
        assert_eq!(decrypted.signing_private_key, original.signing_private_key);
    }

    #[test]
    fn test_document_key_encrypt_decrypt_roundtrip() {
        let (private_key, public_key) = create_test_keys();
        let identity = create_test_identity();

        let decrypted = DecryptedDocumentKey {
            key_id: "key123".to_string(),
            doc_id: "doc456".to_string(),
            user_id: identity,
            key_timestamp: 1234567890,
            key_data: DecryptedKeyData {
                encryption_key: vec![1, 2, 3],
                signing_private_key: vec![4, 5, 6],
            },
        };

        let encrypted = decrypted
            .encrypt(&private_key, &public_key, &public_key)
            .expect("Encryption failed");

        let decrypted_again = encrypted.decrypt(&private_key).expect("Decryption failed");

        assert_eq!(decrypted_again.key_id, "key123");
        assert_eq!(decrypted_again.doc_id, "doc456");
        assert_eq!(decrypted_again.key_timestamp, 1234567890);
        assert_eq!(decrypted_again.key_data.encryption_key, vec![1, 2, 3]);
    }

    #[test]
    fn test_vec_document_key_encrypt_decrypt() {
        let (private_key, public_key) = create_test_keys();
        let identity = create_test_identity();

        let decrypted_vec = vec![
            DecryptedDocumentKey {
                key_id: "key1".to_string(),
                doc_id: "doc1".to_string(),
                user_id: identity,
                key_timestamp: 100,
                key_data: DecryptedKeyData {
                    encryption_key: vec![1],
                    signing_private_key: vec![2],
                },
            },
            DecryptedDocumentKey {
                key_id: "key2".to_string(),
                doc_id: "doc2".to_string(),
                user_id: identity,
                key_timestamp: 200,
                key_data: DecryptedKeyData {
                    encryption_key: vec![3],
                    signing_private_key: vec![4],
                },
            },
        ];

        let encrypted_vec = decrypted_vec
            .encrypt_all(&private_key, &public_key, &public_key)
            .expect("Vec encryption failed");

        assert_eq!(encrypted_vec.len(), 2);

        let decrypted_again = encrypted_vec
            .decrypt_all(&private_key)
            .expect("Vec decryption failed");

        assert_eq!(decrypted_again.len(), 2);
        assert_eq!(decrypted_again[0].key_id, "key1");
        assert_eq!(decrypted_again[1].key_id, "key2");
        assert_eq!(decrypted_again[0].key_timestamp, 100);
        assert_eq!(decrypted_again[1].key_timestamp, 200);
    }

    #[test]
    fn test_empty_vec_encrypt_decrypt() {
        let (private_key, public_key) = create_test_keys();

        let empty_meta: Vec<DecryptedDocumentMetadata> = vec![];
        let encrypted = empty_meta
            .encrypt_all(&private_key, &public_key)
            .expect("Empty vec encryption failed");
        assert_eq!(encrypted.len(), 0);

        let empty_keys: Vec<DecryptedDocumentKey> = vec![];
        let encrypted = empty_keys
            .encrypt_all(&private_key, &public_key, &public_key)
            .expect("Empty vec encryption failed");
        assert_eq!(encrypted.len(), 0);
    }

    #[test]
    fn test_encrypt_for_role_reader_gets_empty_signing_key() {
        let (private_key, public_key) = create_test_keys();

        let original = DecryptedKeyData {
            encryption_key: vec![1, 2, 3, 4, 5],
            signing_private_key: vec![6, 7, 8, 9, 10],
        };

        let encrypted = original
            .encrypt_for_role(Role::Reader, &private_key, &public_key, &public_key)
            .expect("Encryption failed");

        let decrypted =
            DecryptedKeyData::from_encrypted(&encrypted, &private_key).expect("Decryption failed");

        // Reader should get the encryption key
        assert_eq!(decrypted.encryption_key, original.encryption_key);
        // Reader should NOT get the signing key (empty)
        assert!(decrypted.signing_private_key.is_empty());
    }

    #[test]
    fn test_encrypt_for_role_editor_gets_full_key() {
        let (private_key, public_key) = create_test_keys();

        let original = DecryptedKeyData {
            encryption_key: vec![1, 2, 3, 4, 5],
            signing_private_key: vec![6, 7, 8, 9, 10],
        };

        let encrypted = original
            .encrypt_for_role(Role::Editor, &private_key, &public_key, &public_key)
            .expect("Encryption failed");

        let decrypted =
            DecryptedKeyData::from_encrypted(&encrypted, &private_key).expect("Decryption failed");

        // Editor should get both keys
        assert_eq!(decrypted.encryption_key, original.encryption_key);
        assert_eq!(decrypted.signing_private_key, original.signing_private_key);
    }

    #[test]
    fn test_encrypt_for_role_owner_gets_full_key() {
        let (private_key, public_key) = create_test_keys();

        let original = DecryptedKeyData {
            encryption_key: vec![1, 2, 3, 4, 5],
            signing_private_key: vec![6, 7, 8, 9, 10],
        };

        let encrypted = original
            .encrypt_for_role(Role::Owner, &private_key, &public_key, &public_key)
            .expect("Encryption failed");

        let decrypted =
            DecryptedKeyData::from_encrypted(&encrypted, &private_key).expect("Decryption failed");

        // Owner should get both keys
        assert_eq!(decrypted.encryption_key, original.encryption_key);
        assert_eq!(decrypted.signing_private_key, original.signing_private_key);
    }
}
