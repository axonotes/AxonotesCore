use crate::utils::{AppError, Result};
use dashmap::DashMap;
use jsonwebtoken::DecodingKey;
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info};

/// JWKS key cache entry
#[derive(Clone)]
struct CachedKey {
    key: DecodingKey,
    expires_at: Instant,
}

/// JWKS response from issuer
#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<JsonWebKey>,
}

/// Individual JSON Web Key
#[derive(Debug, Deserialize)]
struct JsonWebKey {
    #[allow(dead_code)]
    kty: String, // Key type (e.g., "RSA")
    #[allow(dead_code)]
    kid: Option<String>, // Key ID
    #[serde(rename = "use")]
    #[allow(dead_code)]
    key_use: Option<String>, // e.g., "sig"
    n: Option<String>, // RSA modulus
    e: Option<String>, // RSA exponent
}

/// JWKS fetcher and cache
pub struct JwksCache {
    cache: Arc<DashMap<String, CachedKey>>,
    client: reqwest::Client,
    ttl: Duration,
}

impl JwksCache {
    /// Create new JWKS cache with 1-hour TTL
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            client: reqwest::Client::new(),
            ttl: Duration::from_secs(3600), // 1 hour
        }
    }

    /// Get decoding key for issuer (fetch if not cached or expired)
    pub async fn get_key(&self, issuer: &str) -> Result<DecodingKey> {
        // Check cache first
        if let Some(entry) = self.cache.get(issuer) {
            if entry.expires_at > Instant::now() {
                debug!("JWKS cache hit for issuer: {}", issuer);
                return Ok(entry.key.clone());
            } else {
                debug!("JWKS cache expired for issuer: {}", issuer);
            }
        }

        // Fetch from issuer
        info!("Fetching JWKS for issuer: {}", issuer);
        let key = self.fetch_jwks(issuer).await?;

        // Cache it
        self.cache.insert(
            issuer.to_string(),
            CachedKey {
                key: key.clone(),
                expires_at: Instant::now() + self.ttl,
            },
        );

        Ok(key)
    }

    /// Fetch JWKS from issuer's well-known endpoint
    async fn fetch_jwks(&self, issuer: &str) -> Result<DecodingKey> {
        let jwks_url = format!("{}/.well-known/jwks.json", issuer.trim_end_matches('/'));

        let response = self
            .client
            .get(&jwks_url)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::Config(format!("Failed to fetch JWKS: {}", e)))?;

        if !response.status().is_success() {
            return Err(AppError::Config(format!(
                "JWKS fetch failed with status: {}",
                response.status()
            )));
        }

        let jwks: JwksResponse = response
            .json()
            .await
            .map_err(|e| AppError::Config(format!("Failed to parse JWKS: {}", e)))?;

        // Take the first RSA key
        let key = jwks
            .keys
            .into_iter()
            .find(|k| k.kty == "RSA" && k.n.is_some() && k.e.is_some())
            .ok_or_else(|| AppError::Config("No valid RSA key found in JWKS".to_string()))?;

        // Construct DecodingKey from RSA components
        let n = key.n.unwrap();
        let e = key.e.unwrap();

        DecodingKey::from_rsa_components(&n, &e)
            .map_err(|e| AppError::Config(format!("Failed to create decoding key: {}", e)))
    }

    /// Clear expired entries (call periodically)
    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.cache.retain(|issuer, entry| {
            let keep = entry.expires_at > now;
            if !keep {
                debug!("Removing expired JWKS cache entry for: {}", issuer);
            }
            keep
        });
    }
}

impl Default for JwksCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwks_cache_creation() {
        let cache = JwksCache::new();
        assert_eq!(cache.cache.len(), 0);
    }

    #[test]
    fn test_cleanup_expired() {
        let cache = JwksCache::new();

        // Insert an expired entry manually
        cache.cache.insert(
            "test".to_string(),
            CachedKey {
                key: DecodingKey::from_secret(b"test"),
                expires_at: Instant::now() - Duration::from_secs(1),
            },
        );

        assert_eq!(cache.cache.len(), 1);
        cache.cleanup_expired();
        assert_eq!(cache.cache.len(), 0);
    }
}
