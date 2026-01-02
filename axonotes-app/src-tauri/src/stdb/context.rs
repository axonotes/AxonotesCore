use super::*;

/// A context for performing SpacetimeDB operations for a specific profile
///
/// This context lazily creates connections when needed.
///
/// # Examples
///
/// ```rust
/// // Use active profile
/// stdb::active_profile().create_user(...).await?;
///
/// // Use specific profile
/// stdb::profile("profile_123").update_keys(...).await?;
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
                .ok_or_else(|| format!("Profile {} not found", id))
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
    // Public API - User Operations
    // ==========================================

    /// Create a new user in SpacetimeDB with encryption keys
    pub async fn create_user(
        &self,
        public_encryption_key: Vec<u8>,
        pwd_encrypted_private_encryption_key: Vec<u8>,
        mnemonic_encrypted_private_encryption_key: Vec<u8>,
        public_signing_key: Vec<u8>,
        pwd_encrypted_private_signing_key: Vec<u8>,
        mnemonic_encrypted_private_signing_key: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        conn.reducers
            .create_user(
                public_encryption_key,
                pwd_encrypted_private_encryption_key,
                mnemonic_encrypted_private_encryption_key,
                public_signing_key,
                pwd_encrypted_private_signing_key,
                mnemonic_encrypted_private_signing_key,
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Update encryption keys (requires signature for verification)
    pub async fn update_encryption_keys(
        &self,
        public_encryption_key: Vec<u8>,
        pwd_encrypted_private_encryption_key: Vec<u8>,
        mnemonic_encrypted_private_encryption_key: Vec<u8>,
        public_signing_key: Vec<u8>,
        pwd_encrypted_private_signing_key: Vec<u8>,
        mnemonic_encrypted_private_signing_key: Vec<u8>,
        signature: Vec<u8>,
    ) -> Result<(), String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        conn.reducers
            .set_encryption_keys(
                public_encryption_key,
                pwd_encrypted_private_encryption_key,
                mnemonic_encrypted_private_encryption_key,
                public_signing_key,
                pwd_encrypted_private_signing_key,
                mnemonic_encrypted_private_signing_key,
                signature,
            )
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Get cached user data from SpacetimeDB (if available)
    pub async fn get_cached_user(&self) -> Result<Option<User>, String> {
        let conn = self.get_connection().await?;
        let conn = conn.lock().await;

        // The 'user' view only shows current user's data (filtered by JWT)
        let user = conn.db.user().iter().next();
        Ok(user)
    }
}
