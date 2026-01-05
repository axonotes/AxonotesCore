use super::errors::{AppError, Result};
use blake3::Hasher;

/// Generate user_id from JWT sub and iss claims
/// user_id = BLAKE3(sub || iss)
pub fn generate_user_id(sub: &str, iss: &str) -> String {
    let input = format!("{}{}", sub, iss);
    blake3::hash(input.as_bytes()).to_hex().to_string()
}

/// Validate hash format (64 hex characters)
pub fn validate_hash(hash: &str) -> Result<()> {
    if hash.len() != 64 {
        return Err(AppError::InvalidHash);
    }

    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::InvalidHash);
    }

    Ok(())
}

/// Compute BLAKE3 hash of data
#[allow(dead_code)]
pub fn hash_data(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Create a new BLAKE3 hasher for incremental hashing
pub fn new_hasher() -> Hasher {
    Hasher::new()
}

/// Generate storage path from hash (e.g., "ab/cd/abcd123...")
pub fn hash_to_path(hash: &str) -> String {
    format!("{}/{}/{}", &hash[0..2], &hash[2..4], hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_user_id() {
        let user_id = generate_user_id("user_123", "https://auth.example.com");
        assert_eq!(user_id.len(), 64);
        assert!(user_id.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_generate_user_id_deterministic() {
        let user_id1 = generate_user_id("user_123", "https://auth.example.com");
        let user_id2 = generate_user_id("user_123", "https://auth.example.com");
        assert_eq!(user_id1, user_id2);
    }

    #[test]
    fn test_generate_user_id_different_issuers() {
        let user_id1 = generate_user_id("user_123", "https://auth1.example.com");
        let user_id2 = generate_user_id("user_123", "https://auth2.example.com");
        assert_ne!(user_id1, user_id2);
    }

    #[test]
    fn test_validate_hash_valid() {
        let valid_hash = "a".repeat(64);
        assert!(validate_hash(&valid_hash).is_ok());
    }

    #[test]
    fn test_validate_hash_invalid_length() {
        let invalid_hash = "a".repeat(63);
        assert!(validate_hash(&invalid_hash).is_err());
    }

    #[test]
    fn test_validate_hash_invalid_chars() {
        let invalid_hash = "g".repeat(64);
        assert!(validate_hash(&invalid_hash).is_err());
    }

    #[test]
    fn test_hash_data() {
        let data = b"hello world";
        let hash = hash_data(data);
        assert_eq!(hash.len(), 64);
        // Verify it's deterministic
        let hash2 = hash_data(data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hash_to_path() {
        let hash = "abcd1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
        let path = hash_to_path(hash);
        assert_eq!(
            path,
            "ab/cd/abcd1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        );
    }

    #[test]
    fn test_incremental_hashing() {
        let data = b"hello world";
        let mut hasher = new_hasher();
        hasher.update(&data[0..5]);
        hasher.update(&data[5..]);
        let hash = hasher.finalize().to_hex().to_string();

        // Should equal direct hash
        let direct_hash = hash_data(data);
        assert_eq!(hash, direct_hash);
    }
}
