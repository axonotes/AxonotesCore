//! # Conflict History Encryption
//!
//! Handles encryption and decryption of sync conflict history.
//!
//! ## Conflict History
//!
//! Conflict history records batches that were lost during sync due to conflicts
//! with server-side batches. This allows recovery and future branch visualization.
//!
//! ## Two-Timestamp Pattern
//!
//! This module uses two timestamps:
//! - `timestamp`: First lost batch timestamp (for ordering/display)
//! - `key_timestamp`: Encryption key timestamp (for decryption key selection)
//!
//! This pattern ensures correct key selection for batches that were created
//! offline and encrypted later after a key rotation.
//!
//! ## Note
//!
//! This module depends on STDB bindings being regenerated after adding
//! the `UserSyncConflictHistory` table to `axonotes-stdb`.

#![allow(dead_code)] // Decryption functions used after STDB bindings are regenerated

use crate::crypto::chacha::{decrypt, encrypt};
use crate::encryption::batch::DecryptedBatch;
use crate::encryption::document::DecryptedDocumentKey;
use crate::encryption::helpers::find_correct_decryption_key;
use crate::stdb_bindings::UserSyncConflictHistory;
use postcard::{from_bytes, to_allocvec};
use serde::{Deserialize, Serialize};
use spacetimedb_sdk::Identity;

/// Trait for decrypting a Vec of conflict history entries
pub trait DecryptConflictHistoryVec {
    fn decrypt_all(
        self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<Vec<DecryptedConflictHistory>, String>;
}

/// The inner content of a conflict history entry (encrypted with document key)
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConflictHistoryData {
    pub block_id: u64,
    pub lost_batches: Vec<DecryptedBatch>,
}

/// A fully decrypted conflict history entry
#[derive(Clone, Debug)]
pub struct DecryptedConflictHistory {
    pub history_id: String,
    pub user_id: Identity,
    pub doc_id: String,
    pub block_id: u64,
    pub lost_batches: Vec<DecryptedBatch>,
    /// First lost batch timestamp (for ordering/display)
    pub timestamp: u128,
}

impl ConflictHistoryData {
    /// Encrypt the conflict history data with a document key
    pub fn encrypt(&self, encryption_key: &[u8]) -> Result<Vec<u8>, String> {
        let blob =
            to_allocvec(self).map_err(|e| format!("Error serializing conflict history: {e}"))?;
        encrypt(encryption_key, &blob)
            .map_err(|e| format!("Error encrypting conflict history: {e}"))
    }

    /// Decrypt conflict history data from encrypted blob
    pub fn from_encrypted(encrypted_blob: &[u8], encryption_key: &[u8]) -> Result<Self, String> {
        let decrypted = decrypt(encryption_key, encrypted_blob)
            .map_err(|e| format!("Error decrypting conflict history: {e}"))?;

        from_bytes(&decrypted).map_err(|e| format!("Error deserializing conflict history: {e}"))
    }
}

impl UserSyncConflictHistory {
    /// Decrypt this conflict history entry using document keys
    /// Uses key_timestamp to find the correct decryption key
    pub fn decrypt(
        &self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<DecryptedConflictHistory, String> {
        // Find decryption key by key_timestamp (NOT timestamp)
        let decryption_key =
            find_correct_decryption_key(&self.doc_id, self.key_timestamp, document_keys)?;

        // Decrypt blob
        let data = ConflictHistoryData::from_encrypted(
            &self.encrypted_blob,
            &decryption_key.key_data.encryption_key,
        )?;

        Ok(DecryptedConflictHistory {
            history_id: self.history_id.clone(),
            user_id: self.user_id,
            doc_id: self.doc_id.clone(),
            block_id: data.block_id,
            lost_batches: data.lost_batches,
            timestamp: self.timestamp, // Preserve original timestamp for ordering
        })
    }
}

impl DecryptConflictHistoryVec for Vec<UserSyncConflictHistory> {
    fn decrypt_all(
        self,
        document_keys: &[DecryptedDocumentKey],
    ) -> Result<Vec<DecryptedConflictHistory>, String> {
        self.into_iter().map(|h| h.decrypt(document_keys)).collect()
    }
}

impl DecryptedConflictHistory {
    /// Create new conflict history entry
    /// timestamp = first lost batch timestamp (for ordering)
    pub fn new(
        history_id: String,
        user_id: Identity,
        doc_id: String,
        block_id: u64,
        lost_batches: Vec<DecryptedBatch>,
        timestamp: u128, // First lost batch timestamp
    ) -> Self {
        Self {
            history_id,
            user_id,
            doc_id,
            block_id,
            lost_batches,
            timestamp,
        }
    }

    /// Encrypt for upload to STDB
    /// key_timestamp is set from the encryption key used
    pub fn encrypt(
        &self,
        latest_key: &DecryptedDocumentKey,
    ) -> Result<UserSyncConflictHistory, String> {
        let data = ConflictHistoryData {
            block_id: self.block_id,
            lost_batches: self.lost_batches.clone(),
        };

        let encrypted_blob = data.encrypt(&latest_key.key_data.encryption_key)?;

        Ok(UserSyncConflictHistory {
            history_id: self.history_id.clone(),
            user_id: self.user_id,
            doc_id: self.doc_id.clone(),
            encrypted_blob,
            timestamp: self.timestamp, // First lost batch timestamp (ordering)
            key_timestamp: latest_key.key_timestamp, // Key used for encryption
        })
    }
}
