#![allow(dead_code)]

use crate::batch_handler::block_types::Block;
use crate::call_reducer_await;
use crate::crypto::chacha::encrypt;
use crate::database::get_active_user_keys;
use crate::database::keys::Keys;
use crate::encryption::document::DecryptedDocumentMetadata;
use crate::encryption::document::{DecryptDocumentMetaAndKeyVec, DecryptedDocumentKey};
use crate::encryption::helpers::find_correct_decryption_key;
use crate::encryption::live_block::DecryptLiveBlockVec;
use crate::encryption::live_block::DecryptedLiveBlock;
use crate::encryption::user::UserKeysEncrypted;
use crate::encryption::version_tag::{DecryptVersionTagVec, DecryptedVersionTag};
use crate::stdb::{
    disconnect_profile, ensure_connection_for_profile, get_connection_for_profile,
    is_profile_connected,
};
use crate::stdb_bindings::*;
use crate::utils::timestamp::timestamp;
use crate::utils::vec_array::ByteArrayConversion;
use postcard::to_allocvec;
use spacetimedb_sdk::__codegen::log;
use spacetimedb_sdk::{DbContext, Identity, Table};
use std::sync::Arc;
use tokio::sync::Mutex;

/// A context for performing SpacetimeDB operations for a specific profile
///
/// This context lazily creates connections when needed.
///
/// # Examples
///
/// ```ignore
/// // Use active profile
/// stdb::active_profile().create_user(keys).await?;
///
/// // Use specific profile
/// stdb::profile("profile_123").update_encryption_keys(keys, signature).await?;
/// ```
pub struct ProfileStdbContext {
    profile_id: Option<String>,
}

impl ProfileStdbContext {
    /// Create context for active profile (resolved lazily)
    pub(crate) fn new_active() -> Self {
        Self { profile_id: None }
    }

    /// Create context for specific profile
    pub(crate) fn new(profile_id: String) -> Self {
        Self {
            profile_id: Some(profile_id),
        }
    }

    /// Resolve the profile from the database
    async fn resolve_profile(&self) -> Result<crate::workos_auth::Profile, String> {
        if let Some(id) = &self.profile_id {
            // Get specific profile by ID
            crate::database::get_all_profiles()
                .await?
                .into_iter()
                .find(|p| &p.id == id)
                .ok_or_else(|| format!("Profile {id} not found"))
        } else {
            // Get active profile
            crate::database::get_active_profile()
                .await?
                .ok_or_else(|| "No active profile".to_string())
        }
    }

    /// Ensure connection exists for this profile, return profile_id
    async fn ensure_connected(&self) -> Result<String, String> {
        let profile = self.resolve_profile().await?;
        ensure_connection_for_profile(&profile.id, &profile.access_token).await?;
        Ok(profile.id)
    }

    /// Get the connection for this profile
    async fn get_connection(&self) -> Result<Arc<Mutex<DbConnection>>, String> {
        let profile_id = self.ensure_connected().await?;
        get_connection_for_profile(&profile_id).await
    }

    // ==========================================
    // Public API - Connection Management
    // ==========================================

    /// Explicitly connect to SpacetimeDB (usually not needed, auto-connects on first use)
    pub async fn connect(&self) -> Result<(), String> {
        self.ensure_connected().await?;
        Ok(())
    }

    /// Disconnect from SpacetimeDB
    pub async fn disconnect(&self) -> Result<(), String> {
        let profile_id = self.resolve_profile().await?.id;
        disconnect_profile(&profile_id).await
    }

    /// Check if this profile is connected
    pub async fn is_connected(&self) -> Result<bool, String> {
        let profile_id = self.resolve_profile().await?.id;
        Ok(is_profile_connected(&profile_id).await)
    }

    /// Get the SpacetimeDB identity for this profile
    pub async fn get_identity(&self) -> Result<Option<Identity>, String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;
        Ok(conn.try_identity())
    }

    // ==========================================
    // Getters
    // ==========================================

    /// Get cached user data from SpacetimeDB (if available)
    pub async fn get_cached_user(&self) -> Result<Option<User>, String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        // The 'user' view only shows current user's data (filtered by JWT)
        let user = conn.db.user().iter().next();
        Ok(user)
    }

    /// Get cached document metadata from SpacetimeDB
    pub async fn get_cached_metadata(&self) -> Result<Vec<DecryptedDocumentMetadata>, String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        let user_keys: Option<Keys> = get_active_user_keys().await?;
        if let Some(user_keys) = user_keys {
            let private_encryption_key = user_keys.private_encryption_key.as_array()?;
            conn.db
                .user_metadata()
                .iter()
                .collect::<Vec<_>>()
                .decrypt_all(private_encryption_key)
        } else {
            Err("No active user keys available. Not synced with stdb?".to_string())
        }
    }

    /// Get cached document keys from SpacetimeDB
    pub async fn get_cached_document_keys(&self) -> Result<Vec<DecryptedDocumentKey>, String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        let user_keys: Option<Keys> = get_active_user_keys().await?;
        if let Some(user_keys) = user_keys {
            let private_encryption_key = user_keys.private_encryption_key.as_array()?;
            conn.db
                .user_document_keys()
                .iter()
                .collect::<Vec<_>>()
                .decrypt_all(private_encryption_key)
        } else {
            Err("No active user keys available. Not synced with stdb?".to_string())
        }
    }

    /// Get cached live blocks from SpacetimeDB
    pub async fn get_cached_live_blocks(&self) -> Result<Vec<DecryptedLiveBlock>, String> {
        // Get document keys BEFORE acquiring connection lock to avoid deadlock
        // (get_cached_document_keys also acquires the connection lock)
        let cached_document_keys = self.get_cached_document_keys().await?;

        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        conn.db
            .accessible_live_blocks()
            .iter()
            .collect::<Vec<_>>()
            .decrypt_all(cached_document_keys.as_slice())
    }

    /// Get document permissions for a specific document (unencrypted)
    /// Returns permissions from the manageable_permissions view
    pub async fn get_document_permissions(
        &self,
        doc_id: &str,
    ) -> Result<Vec<DocumentPermission>, String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        Ok(conn
            .db
            .manageable_permissions()
            .iter()
            .filter(|p| p.doc_id == doc_id)
            .collect())
    }

    /// Get public keys of collaborators (unencrypted)
    /// Returns public keys for all users who have access to documents you can access
    pub async fn get_public_user_keys(&self) -> Result<Vec<PublicUserInfo>, String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        Ok(conn.db.public_user_keys().iter().collect())
    }

    /// Get cached version tags for a specific document (decrypted)
    pub async fn get_cached_version_tags(
        &self,
        doc_id: &str,
    ) -> Result<Vec<DecryptedVersionTag>, String> {
        // Get document keys BEFORE acquiring connection lock to avoid deadlock
        let document_keys = self.get_cached_document_keys().await?;

        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        conn.db
            .accessible_version_tags()
            .iter()
            .filter(|t| t.doc_id == doc_id)
            .collect::<Vec<_>>()
            .decrypt_all(&document_keys)
    }

    // ==========================================
    // Reducer wrappers
    // ==========================================

    /// Create a new user in SpacetimeDB with encryption keys
    pub async fn create_user(&self, stdb_keys: UserKeysEncrypted) -> Result<(), String> {
        log::debug!("[stdb::create_user] Getting connection...");
        let conn = self.get_connection().await?;
        log::debug!("[stdb::create_user] Locking connection...");
        let conn = conn.lock().await;
        log::debug!("[stdb::create_user] Connection acquired");

        let public_encryption_key = stdb_keys.public_encryption_key;
        let pwd_encrypted_private_encryption_key = stdb_keys.pwd_encrypted_private_encryption_key;
        let mnemonic_encrypted_private_encryption_key =
            stdb_keys.mnemonic_encrypted_private_encryption_key;
        let public_signing_key = stdb_keys.public_signing_key;
        let pwd_encrypted_private_signing_key = stdb_keys.pwd_encrypted_private_signing_key;
        let mnemonic_encrypted_private_signing_key =
            stdb_keys.mnemonic_encrypted_private_signing_key;

        log::info!("[stdb::create_user] Calling create_user reducer...");
        let result = call_reducer_await!(
            conn,
            create_user,
            public_encryption_key,
            pwd_encrypted_private_encryption_key,
            mnemonic_encrypted_private_encryption_key,
            public_signing_key,
            pwd_encrypted_private_signing_key,
            mnemonic_encrypted_private_signing_key
        );

        match &result {
            Ok(()) => log::info!("[stdb::create_user] ✓ Reducer completed successfully"),
            Err(e) => log::error!("[stdb::create_user] ✗ Reducer failed: {e}"),
        }

        result
    }

    /// Update encryption keys (requires signature for verification)
    pub async fn update_encryption_keys(
        &self,
        stdb_keys: UserKeysEncrypted,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        let public_encryption_key = stdb_keys.public_encryption_key;
        let pwd_encrypted_private_encryption_key = stdb_keys.pwd_encrypted_private_encryption_key;
        let mnemonic_encrypted_private_encryption_key =
            stdb_keys.mnemonic_encrypted_private_encryption_key;
        let public_signing_key = stdb_keys.public_signing_key;
        let pwd_encrypted_private_signing_key = stdb_keys.pwd_encrypted_private_signing_key;
        let mnemonic_encrypted_private_signing_key =
            stdb_keys.mnemonic_encrypted_private_signing_key;

        call_reducer_await!(
            conn,
            set_encryption_keys,
            public_encryption_key,
            pwd_encrypted_private_encryption_key,
            mnemonic_encrypted_private_encryption_key,
            public_signing_key,
            pwd_encrypted_private_signing_key,
            mnemonic_encrypted_private_signing_key,
            signature
        )
    }

    /// Upload a batch of patches for a document
    pub async fn upload_batch(
        &self,
        batch_id: String,
        doc_id: String,
        timestamp: u128,
        encrypted_data: Vec<u8>,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(
            conn,
            upload_batch,
            batch_id,
            doc_id,
            timestamp,
            encrypted_data,
            signature
        )
    }

    /// Creates a new document owned by the caller
    pub async fn create_document(
        &self,
        doc_id: String,
        new_public_signing_key: Vec<u8>,
        key_timestamp: u128,
        encrypted_key_data: Vec<u8>, // DocumentKeyData encrypted for owner
        encrypted_metadata_blob: Vec<u8>, // Default path "/Untitled.doc"
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(
            conn,
            create_document,
            doc_id,
            new_public_signing_key,
            key_timestamp,
            encrypted_key_data,
            encrypted_metadata_blob
        )
    }

    /// Deletes a document and all related data
    /// Only the owner can delete a document
    pub async fn delete_document(&self, doc_id: String, signature: Vec<u8>) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, delete_document, doc_id, signature)
    }

    /// Rotate document keys (called after user removal)
    ///
    /// This is a CRITICAL atomic operation that:
    /// 1. Updates document's current signing key
    /// 2. Deletes old keys for revoked users (already done in remove_user)
    /// 3. Inserts new keys for all remaining users
    /// 4. Stores snapshot batches for ALL blocks
    /// 5. Re-encrypts all version tags with new key
    #[allow(clippy::too_many_arguments)] // Mirrors database reducer signature
    pub async fn rotate_document_keys(
        &self,
        doc_id: String,
        new_key_timestamp: u128,
        new_public_signing_key: Vec<u8>,
        user_keys: Vec<UserKeyEntry>,
        snapshot_batches: Vec<SnapshotBatch>,
        re_encrypted_tags: Vec<ReEncryptedTag>,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(
            conn,
            rotate_document_keys,
            doc_id,
            new_key_timestamp,
            new_public_signing_key,
            user_keys,
            snapshot_batches,
            re_encrypted_tags,
            signature
        )
    }

    /// Try to acquire a lock on a block for editing
    /// Returns error if block is locked by someone else
    pub async fn try_lock_block(
        &self,
        doc_id: String,
        block_id: u64,
        content: &Block,
        username: String,
    ) -> Result<(), String> {
        // Get document keys BEFORE acquiring connection lock to avoid deadlock
        let cached_document_keys = self.get_cached_document_keys().await?;
        let latest_document_key =
            find_correct_decryption_key(&doc_id, timestamp(), cached_document_keys.as_slice())?;

        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        let username_blob =
            to_allocvec(&username).map_err(|e| format!("Error serializing batch data: {e}"))?;

        let content_blob = serde_json::to_vec(content).map_err(|e| e.to_string())?;

        let encrypted_content = encrypt(
            latest_document_key.key_data.encryption_key.as_slice(),
            content_blob.as_slice(),
        )
        .map_err(|e| format!("Error when encrypting live block content: {e}"))?;

        let encrypted_username = encrypt(
            latest_document_key.key_data.encryption_key.as_slice(),
            username_blob.as_slice(),
        )
        .map_err(|e| format!("Error when encrypting live block username: {e}"))?;

        call_reducer_await!(
            conn,
            try_lock_block,
            doc_id,
            block_id,
            encrypted_content,
            encrypted_username
        )
    }

    /// Update live block content (while maintaining lock)
    pub async fn update_live_block(
        &self,
        doc_id: String,
        block_id: u64,
        content: &Block,
    ) -> Result<(), String> {
        // Get document keys BEFORE acquiring connection lock to avoid deadlock
        let cached_document_keys = self.get_cached_document_keys().await?;
        let latest_document_key =
            find_correct_decryption_key(&doc_id, timestamp(), cached_document_keys.as_slice())?;

        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        let content_blob = serde_json::to_vec(content).map_err(|e| e.to_string())?;

        let encrypted_content = encrypt(
            latest_document_key.key_data.encryption_key.as_slice(),
            content_blob.as_slice(),
        )
        .map_err(|e| format!("Error when encrypting live block content: {e}"))?;

        call_reducer_await!(conn, update_live_block, doc_id, block_id, encrypted_content)
    }

    /// Unlock a block (change to focused state or delete)
    pub async fn unlock_block(
        &self,
        doc_id: String,
        block_id: u64,
        delete: bool, // true = delete row, false = set locked_at to None
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, unlock_block, doc_id, block_id, delete)
    }

    /// Create document metadata for the current user
    /// Called when user gains access to a document
    pub async fn create_document_metadata(
        &self,
        doc_id: String,
        encrypted_blob: Vec<u8>, // path, tags, etc.
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, create_document_metadata, doc_id, encrypted_blob)
    }

    /// Update document metadata (path, tags)
    pub async fn update_document_metadata(
        &self,
        doc_id: String,
        encrypted_blob: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, update_document_metadata, doc_id, encrypted_blob)
    }

    /// Delete document metadata
    pub async fn delete_document_metadata(&self, doc_id: String) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, delete_document_metadata, doc_id)
    }

    /// Add a user to a document
    /// Owner or Editor can add users
    /// Editors cannot assign Owner role
    pub async fn add_user_to_document(
        &self,
        doc_id: String,
        new_user_id: Identity,
        role: Role,
        encrypted_keys: Vec<EncryptedKeyEntry>,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(
            conn,
            add_user_to_document,
            doc_id,
            new_user_id,
            role,
            encrypted_keys,
            signature
        )
    }

    /// Remove a user from a document
    /// ALWAYS triggers key rotation
    /// Owner/Editor can remove users (except Owner)
    pub async fn remove_user_from_document(
        &self,
        doc_id: String,
        removed_user_id: Identity,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(
            conn,
            remove_user_from_document,
            doc_id,
            removed_user_id,
            signature
        )
    }

    /// Change a user's role (Editor ↔ Reader)
    ///
    /// For Reader → Editor: Provide `updated_key` with the current key encrypted with signing key
    /// For Editor → Reader: Client should call key rotation AFTER this
    pub async fn change_user_role(
        &self,
        doc_id: String,
        target_user_id: Identity,
        new_role: Role,
        updated_key: Option<EncryptedKeyEntry>,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(
            conn,
            change_user_role,
            doc_id,
            target_user_id,
            new_role,
            updated_key,
            signature
        )
    }

    /// Transfer ownership to another user
    /// Old owner becomes Editor
    pub async fn transfer_ownership(
        &self,
        doc_id: String,
        new_owner_id: Identity,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, transfer_ownership, doc_id, new_owner_id, signature)
    }

    /// Create a version tag for a document
    /// Only Owner or Editor can create tags
    pub async fn create_version_tag(
        &self,
        tag_id: String,
        doc_id: String,
        encrypted_blob: Vec<u8>, // tag_name, timestamp, created_by, created_at
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(
            conn,
            create_version_tag,
            tag_id,
            doc_id,
            encrypted_blob,
            signature
        )
    }

    /// Delete a version tag
    /// Only Owner can delete tags
    pub async fn delete_version_tag(
        &self,
        tag_id: String,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, delete_version_tag, tag_id, signature)
    }

    // ==========================================
    // Share Operations
    // ==========================================

    /// Create a pending share for a document
    /// Returns after reducer completes - client reads share_code from subscription
    pub async fn create_pending_share(
        &self,
        doc_id: String,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, create_pending_share, doc_id, signature)
    }

    /// Join a pending share using a share code
    pub async fn join_pending_share(&self, share_code: String) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, join_pending_share, share_code)
    }

    /// Close a pending share (cleanup after done or abandon)
    pub async fn close_pending_share(
        &self,
        share_code: String,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, close_pending_share, share_code, signature)
    }

    /// Leave a pending share (joiner withdraws)
    pub async fn leave_pending_share(&self, share_code: String) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        call_reducer_await!(conn, leave_pending_share, share_code)
    }
}

pub fn get_message_to_sign(stdb_keys: UserKeysEncrypted) -> Vec<u8> {
    let vecs = [
        stdb_keys.public_encryption_key,
        stdb_keys.pwd_encrypted_private_encryption_key,
        stdb_keys.mnemonic_encrypted_private_encryption_key,
        stdb_keys.public_signing_key,
        stdb_keys.pwd_encrypted_private_signing_key,
        stdb_keys.mnemonic_encrypted_private_signing_key,
    ];
    vecs.concat()
}
