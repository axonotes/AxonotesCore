use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params, Version,
};

/// Argon2id parameters optimized for mobile devices
/// ~300ms on desktop, ~1s on mobile
/// Provides ~24,000+ years brute-force resistance
const MEMORY_COST: u32 = 19456; // 19 MB (OWASP minimum)
const TIME_COST: u32 = 2; // iterations
const PARALLELISM: u32 = 2; // threads

// ========================================
// Public API - Simple & Clean
// ========================================

pub const MASTER_PASSWORD_ENCRYPTION_CONTEXT: &str = "mp_encryption_context";
pub const MASTER_PASSWORD_KEY_SIGNING_CONTEXT: &str = "mp_signing_context";

/// Derive a deterministic 32-byte encryption key from a password
///
/// Use this for encryption keys where you need the same password
/// to always produce the same key (e.g., database encryption).
///
/// The context parameter ensures different purposes produce different keys.
///
/// Use `MASTER_PASSWORD_ENCRYPTION_CONTEXT` or `MASTER_PASSWORD_KEY_SIGNING_CONTEXT` for
/// master password key derive.
///
/// # Examples
///
/// ```
/// let db_key = derive_key("mypassword", "database");
/// let file_key = derive_key("mypassword", "file_encryption");
/// assert_ne!(db_key, file_key); // Different contexts = different keys
/// ```
pub fn derive_key(password: &str, context: &str) -> Vec<u8> {
    // Create a deterministic salt based on context
    let salt_bytes = create_context_salt(context);

    let params = Params::new(MEMORY_COST, TIME_COST, PARALLELISM, Some(32))
        .expect("Invalid Argon2 parameters");

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);

    let mut output = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), &salt_bytes, &mut output)
        .expect("Failed to derive key");

    output.to_vec()
}

/// Hash a password for secure storage
///
/// Each call produces a different hash (using a random salt).
/// Use `verify()` to check if a password matches.
///
/// # Examples
///
/// ```
/// let hash = hash("mypassword");
/// // Store hash in database
/// ```
pub fn hash(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);

    let params = Params::new(MEMORY_COST, TIME_COST, PARALLELISM, Some(32))
        .expect("Invalid Argon2 parameters");

    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);

    argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string()
}

/// Verify a password against a stored hash
///
/// # Examples
///
/// ```
/// let hash = hash("mypassword");
/// assert!(verify("mypassword", &hash));
/// assert!(!verify("wrongpassword", &hash));
/// ```
pub fn verify(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

// ========================================
// Convenience Functions
// ========================================

/// Derive database encryption key (convenience wrapper)
pub fn derive_database_key(password: &str) -> Vec<u8> {
    derive_key(password, "database_encryption")
}

/// Derive file encryption key (convenience wrapper)
pub fn derive_file_key(password: &str, file_id: &str) -> Vec<u8> {
    derive_key(password, format!("file_{}", file_id).as_str())
}

// ========================================
// Internal Helpers
// ========================================

fn create_context_salt(context: &str) -> [u8; 16] {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Create deterministic 16-byte salt from context
    let mut hasher = DefaultHasher::new();
    context.hash(&mut hasher);
    let hash1 = hasher.finish();

    // Get second hash for more entropy
    let mut hasher = DefaultHasher::new();
    format!("{}:salt", context).hash(&mut hasher);
    let hash2 = hasher.finish();

    let mut salt = [0u8; 16];
    salt[0..8].copy_from_slice(&hash1.to_le_bytes());
    salt[8..16].copy_from_slice(&hash2.to_le_bytes());
    salt
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_derive_key_deterministic() {
        let key1 = derive_key("password", "test");
        let key2 = derive_key("password", "test");
        assert_eq!(
            key1, key2,
            "Same password and context should derive same key"
        );
    }

    #[test]
    fn test_derive_key_different_context() {
        let key1 = derive_key("password", "context1");
        let key2 = derive_key("password", "context2");
        assert_ne!(
            key1, key2,
            "Different contexts should produce different keys"
        );
    }

    #[test]
    fn test_derive_key_different_password() {
        let key1 = derive_key("password1", "test");
        let key2 = derive_key("password2", "test");
        assert_ne!(
            key1, key2,
            "Different passwords should produce different keys"
        );
    }

    #[test]
    fn test_hash_verify() {
        let password = "MySecurePassword123";
        let hash = hash(password);

        assert!(
            verify(password, hash.as_str()),
            "Correct password should verify"
        );
        assert!(
            !verify("WrongPassword", hash.as_str()),
            "Wrong password should not verify"
        );
    }

    #[test]
    fn test_hash_different_each_time() {
        let password = "test";
        let hash1 = hash(password);
        let hash2 = hash(password);

        assert_ne!(hash1, hash2, "Each hash should use different salt");
        assert!(verify(password, hash1.as_str()));
        assert!(verify(password, hash2.as_str()));
    }

    #[test]
    fn test_key_length() {
        let key = derive_key("test", "test");
        assert_eq!(key.len(), 32, "Key should be 32 bytes for AES-256");
    }

    #[test]
    fn test_convenience_functions() {
        let db_key = derive_database_key("password");
        let file_key = derive_file_key("password", "doc123");

        assert_ne!(db_key, file_key);
        assert_eq!(db_key.len(), 32);
        assert_eq!(file_key.len(), 32);
    }

    #[test]
    fn benchmark() {
        println!("\n=== Argon2id Performance ===");
        println!(
            "Config: {}MB memory, {} iterations, {} parallelism\n",
            MEMORY_COST / 1024,
            TIME_COST,
            PARALLELISM
        );

        // Test derive_key
        let start = Instant::now();
        let _key = derive_key("ABC12345", "database");
        let derive_time = start.elapsed();
        println!(
            "derive_key: {:?} ({:.2}ms)",
            derive_time,
            derive_time.as_secs_f64() * 1000.0
        );

        // Test hash
        let start = Instant::now();
        let hash_result = hash("MyPassword123");
        let hash_time = start.elapsed();
        println!(
            "hash:       {:?} ({:.2}ms)",
            hash_time,
            hash_time.as_secs_f64() * 1000.0
        );

        // Test verify
        let start = Instant::now();
        let _verified = verify("MyPassword123", hash_result.as_str());
        let verify_time = start.elapsed();
        println!(
            "verify:     {:?} ({:.2}ms)",
            verify_time,
            verify_time.as_secs_f64() * 1000.0
        );

        // Brute-force estimate
        let single_hash = derive_time.as_secs_f64();
        let combinations = 36_u64.pow(8) as f64; // 8-char alphanumeric
        let years = (combinations * single_hash) / (60.0 * 60.0 * 24.0 * 365.25);

        println!("\n=== Security ===");
        println!("8-char PIN combinations: {:.2e}", combinations);
        println!("Time to brute-force: {:.0} years", years);
    }
}
