pub mod jwks;

use crate::utils::{AppError, Result, hash::generate_user_id};
use jsonwebtoken::{Algorithm, Validation, decode};
use jwks::JwksCache;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;

/// JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user ID from issuer)
    pub iss: String, // Issuer
    pub exp: u64,    // Expiration
    #[serde(flatten)]
    pub extra: serde_json::Value, // All other claims for quota rules
}

/// User authentication info extracted from JWT
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub user_id: String,        // BLAKE3(sub || iss) as hex
    pub user_id_bytes: Vec<u8>, // BLAKE3(sub || iss) as bytes
    #[allow(dead_code)]
    pub sub: String,
    #[allow(dead_code)]
    pub iss: String,
    pub claims: serde_json::Value, // All claims for quota evaluation
}

/// JWT authenticator
pub struct Authenticator {
    jwks_cache: Arc<JwksCache>,
    allowed_issuers: Vec<String>,
    dev_mode: bool,
}

impl Authenticator {
    pub fn new(allowed_issuers: Vec<String>, dev_mode: bool) -> Self {
        if dev_mode {
            warn!("⚠️  JWT dev_mode is ENABLED - signature verification is DISABLED!");
            warn!("⚠️  This is a SECURITY RISK - do NOT use in production!");
        }
        Self {
            jwks_cache: Arc::new(JwksCache::new()),
            allowed_issuers,
            dev_mode,
        }
    }

    /// Verify JWT and extract user info
    pub async fn verify_token(&self, token: &str) -> Result<AuthUser> {
        // Decode header to get issuer
        let _header = jsonwebtoken::decode_header(token)
            .map_err(|e| AppError::Unauthorized(format!("Invalid token header: {}", e)))?;

        // Decode without verification first to get issuer and check expiration
        let unverified: jsonwebtoken::TokenData<Claims> =
            jsonwebtoken::dangerous::insecure_decode(token)
                .map_err(|e| AppError::Unauthorized(format!("Invalid token: {}", e)))?;

        // Always validate expiration, even in dev mode
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| AppError::Internal(format!("System time error: {}", e)))?
            .as_secs();

        if unverified.claims.exp < now {
            return Err(AppError::Unauthorized("Token has expired".to_string()));
        }

        let issuer = &unverified.claims.iss;

        // Dev mode: skip signature verification (for development only!)
        let claims = if self.dev_mode {
            unverified.claims
        } else {
            // Check if issuer is allowed
            if !self.allowed_issuers.contains(issuer) {
                return Err(AppError::Unauthorized(format!(
                    "Issuer not allowed: {}",
                    issuer
                )));
            }

            // Get decoding key from JWKS
            let key = self.jwks_cache.get_key(issuer).await?;

            // Verify signature
            let mut validation = Validation::new(Algorithm::RS256);
            validation.set_issuer(&[issuer]);

            let token_data = decode::<Claims>(token, &key, &validation)
                .map_err(|e| AppError::Unauthorized(format!("Token verification failed: {}", e)))?;

            token_data.claims
        };

        // Generate user_id from sub and iss
        let user_id_hex = generate_user_id(&claims.sub, &claims.iss);
        let user_id_bytes = hex::decode(&user_id_hex)
            .map_err(|e| AppError::Internal(format!("Failed to decode user_id hex: {}", e)))?;

        Ok(AuthUser {
            user_id: user_id_hex,
            user_id_bytes,
            sub: claims.sub.clone(),
            iss: claims.iss.clone(),
            claims: claims.extra,
        })
    }

    /// Get JWKS cache for cleanup
    pub fn jwks_cache(&self) -> Arc<JwksCache> {
        self.jwks_cache.clone()
    }

    /// Check if dev mode is enabled
    #[allow(dead_code)]
    pub fn is_dev_mode(&self) -> bool {
        self.dev_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authenticator_creation() {
        let auth = Authenticator::new(vec!["https://auth.example.com".to_string()], false);
        assert_eq!(auth.allowed_issuers.len(), 1);
        assert!(!auth.is_dev_mode());
    }

    #[test]
    fn test_authenticator_dev_mode() {
        let auth = Authenticator::new(vec![], true);
        assert!(auth.is_dev_mode());
    }
}
