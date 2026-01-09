//! # Identity Derivation
//!
//! Derives SpacetimeDB Identity from JWT claims.
//!
//! This allows computing the user's identity offline without needing
//! an active STDB connection. The identity is derived deterministically
//! from the JWT's issuer and subject claims using BLAKE3 hashing.
//!
//! ## Algorithm
//!
//! 1. Concatenate `"{issuer}|{subject}"`
//! 2. BLAKE3 hash → take first 26 bytes as `id_hash`
//! 3. Checksum: BLAKE3 hash of `[0xc2, 0x00, id_hash...]` → take first 4 bytes
//! 4. Final: `[0xc2, 0x00, checksum[0..4], id_hash[0..26]]`
//! 5. Interpret as big-endian 256-bit integer

use serde::Deserialize;
use spacetimedb_sdk::Identity;

/// JWT claims needed for identity derivation
#[derive(Debug, Deserialize)]
pub struct IdentityClaims {
    /// Issuer claim (iss)
    #[serde(rename = "iss")]
    pub issuer: String,
    /// Subject claim (sub)
    #[serde(rename = "sub")]
    pub subject: String,
}

/// Derives a SpacetimeDB Identity from JWT issuer and subject claims.
///
/// This matches the algorithm used by SpacetimeDB server to derive identity
/// from JWT tokens, allowing offline identity computation.
pub fn derive_identity_from_claims(issuer: &str, subject: &str) -> Identity {
    // Step 1: Concatenate issuer and subject
    let input = format!("{issuer}|{subject}");

    // Step 2: BLAKE3 hash and take first 26 bytes
    let first_hash = blake3::hash(input.as_bytes());
    let id_hash = &first_hash.as_bytes()[..26];

    // Step 3: Create checksum input and hash
    let mut checksum_input = [0u8; 28];
    checksum_input[0] = 0xc2;
    checksum_input[1] = 0x00;
    checksum_input[2..].copy_from_slice(id_hash);
    let checksum_hash = blake3::hash(&checksum_input);

    // Step 4: Build final 32-byte identity
    let mut final_bytes = [0u8; 32];
    final_bytes[0] = 0xc2;
    final_bytes[1] = 0x00;
    final_bytes[2..6].copy_from_slice(&checksum_hash.as_bytes()[..4]);
    final_bytes[6..].copy_from_slice(id_hash);

    // Step 5: Interpret as big-endian (most significant bytes first)
    Identity::from_byte_array(u256_from_be_bytes(final_bytes))
}

/// Convert big-endian bytes to little-endian for Identity storage
fn u256_from_be_bytes(be_bytes: [u8; 32]) -> [u8; 32] {
    let mut le_bytes = be_bytes;
    le_bytes.reverse();
    le_bytes
}

/// Decode JWT and extract identity claims without signature validation.
///
/// This is safe because we only use the claims for identity derivation,
/// not for authorization decisions.
pub fn decode_jwt_claims(token: &str) -> Result<IdentityClaims, String> {
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

    let mut validation = Validation::new(Algorithm::RS256);
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;
    validation.validate_aud = false;
    validation.set_required_spec_claims::<&str>(&[]);

    let dummy_key = DecodingKey::from_secret(&[]);

    decode::<IdentityClaims>(token, &dummy_key, &validation)
        .map(|data| data.claims)
        .map_err(|e| format!("Failed to decode JWT claims: {e}"))
}

/// Derive identity directly from a JWT token.
pub fn derive_identity_from_jwt(token: &str) -> Result<Identity, String> {
    let claims = decode_jwt_claims(token)?;
    Ok(derive_identity_from_claims(&claims.issuer, &claims.subject))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_derivation_deterministic() {
        let issuer = "https://example.com";
        let subject = "user_123";

        let id1 = derive_identity_from_claims(issuer, subject);
        let id2 = derive_identity_from_claims(issuer, subject);

        assert_eq!(id1, id2, "Identity derivation should be deterministic");
    }

    #[test]
    fn test_identity_starts_with_c200() {
        let issuer = "https://example.com";
        let subject = "user_123";

        let identity = derive_identity_from_claims(issuer, subject);
        let hex = identity.to_hex().to_string();

        assert!(
            hex.starts_with("c200"),
            "Identity hex should start with c200, got: {hex}"
        );
    }

    #[test]
    fn test_different_claims_different_identity() {
        let id1 = derive_identity_from_claims("issuer1", "subject1");
        let id2 = derive_identity_from_claims("issuer2", "subject2");

        assert_ne!(
            id1, id2,
            "Different claims should produce different identities"
        );
    }
}
