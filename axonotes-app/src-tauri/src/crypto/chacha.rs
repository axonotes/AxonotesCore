//! # ChaCha20-Poly1305 Authenticated Encryption
//!
//! Provides symmetric authenticated encryption using the ChaCha20-Poly1305 AEAD cipher.
//!
//! ## Features
//!
//! - **256-bit keys**: Cryptographically secure random key generation
//! - **96-bit nonces**: Randomly generated per encryption (prepended to ciphertext)
//! - **Authentication**: Poly1305 MAC ensures data integrity and authenticity
//!
//! ## Wire Format
//!
//! Encrypted data format: `[nonce (12 bytes)][ciphertext][auth tag (16 bytes)]`
//!
//! ## Usage
//!
//! ```ignore
//! let key = generate_key();
//! let ciphertext = encrypt(&key, b"secret data")?;
//! let plaintext = decrypt(&key, &ciphertext)?;
//! ```

use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, Key};

/// Generates a cryptographically secure 256-bit (32-byte) encryption key.
///
/// Uses the operating system's secure random number generator.
///
/// # Returns
///
/// A 32-byte vector suitable for use with ChaCha20-Poly1305.
pub fn generate_key() -> Vec<u8> {
    ChaCha20Poly1305::generate_key(&mut OsRng)
        .as_slice()
        .to_vec()
}

/// Encrypts data using ChaCha20-Poly1305 authenticated encryption.
///
/// A random 96-bit nonce is generated for each encryption and prepended to the ciphertext.
/// The Poly1305 authentication tag is appended to ensure integrity.
///
/// # Arguments
///
/// * `key` - A 32-byte encryption key
/// * `content` - The plaintext data to encrypt
///
/// # Returns
///
/// A vector containing `[nonce (12 bytes)][ciphertext][auth tag (16 bytes)]`
///
/// # Errors
///
/// Returns an error if:
/// - The key is not exactly 32 bytes
/// - Encryption fails (rare, usually indicates system issues)
pub fn encrypt(key: &[u8], content: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let enc_content = cipher
        .encrypt(&nonce, content)
        .map_err(|e| format!("Encryption failed: {e:?}"))?;

    let mut encrypted = nonce.as_slice().to_vec();
    encrypted.extend_from_slice(&enc_content);

    Ok(encrypted)
}

/// Decrypts data encrypted with [`encrypt`].
///
/// Extracts the nonce from the first 12 bytes, then decrypts and verifies
/// the authentication tag.
///
/// # Arguments
///
/// * `key` - The same 32-byte key used for encryption
/// * `encrypted` - The ciphertext produced by [`encrypt`]
///
/// # Returns
///
/// The original plaintext data.
///
/// # Errors
///
/// Returns an error if:
/// - The encrypted data is shorter than 12 bytes (missing nonce)
/// - The key is incorrect
/// - The ciphertext has been tampered with (authentication failure)
pub fn decrypt(key: &[u8], encrypted: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if encrypted.len() < 12 {
        return Err("Encrypted data too short".into());
    }

    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let (nonce, enc_content) = encrypted.split_at(12);

    let plaintext = cipher
        .decrypt(nonce.into(), enc_content)
        .map_err(|e| format!("Decryption failed: {e:?}"))?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_key_correct_length() {
        let key = generate_key();
        assert_eq!(key.len(), 32, "ChaCha20Poly1305 key should be 32 bytes");
    }

    #[test]
    fn test_generate_key_is_unique() {
        let key1 = generate_key();
        let key2 = generate_key();
        let key3 = generate_key();

        assert_ne!(key1, key2, "Generated keys should be unique");
        assert_ne!(key2, key3, "Generated keys should be unique");
        assert_ne!(key1, key3, "Generated keys should be unique");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = generate_key();
        let plaintext = b"Hello, World!";

        let encrypted = encrypt(key.as_slice(), plaintext).expect("Encryption should succeed");
        let decrypted =
            decrypt(key.as_slice(), encrypted.as_slice()).expect("Decryption should succeed");

        assert_eq!(decrypted, plaintext, "Decrypted text should match original");
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let key = generate_key();
        let plaintext = b"Same message";

        let encrypted1 = encrypt(key.as_slice(), plaintext).expect("Encryption should succeed");
        let encrypted2 = encrypt(key.as_slice(), plaintext).expect("Encryption should succeed");

        assert_ne!(
            encrypted1, encrypted2,
            "Same plaintext should produce different ciphertexts (due to random nonces)"
        );
    }

    #[test]
    fn test_decrypt_with_wrong_key_fails() {
        let key1 = generate_key();
        let key2 = generate_key();
        let plaintext = b"Secret message";

        let encrypted = encrypt(key1.as_slice(), plaintext).expect("Encryption should succeed");
        let result = decrypt(key2.as_slice(), encrypted.as_slice());

        assert!(result.is_err(), "Decryption with wrong key should fail");
    }

    #[test]
    fn test_decrypt_with_corrupted_ciphertext_fails() {
        let key = generate_key();
        let plaintext = b"Important data";

        let mut encrypted = encrypt(key.as_slice(), plaintext).expect("Encryption should succeed");

        // Corrupt the ciphertext (skip nonce, corrupt the actual encrypted data)
        if encrypted.len() > 13 {
            encrypted[13] ^= 0xFF;
        }

        let result = decrypt(key.as_slice(), encrypted.as_slice());
        assert!(
            result.is_err(),
            "Decryption of corrupted ciphertext should fail"
        );
    }

    #[test]
    fn test_encrypt_decrypt_empty_data() {
        let key = generate_key();
        let plaintext = b"";

        let encrypted = encrypt(key.as_slice(), plaintext).expect("Should encrypt empty data");
        let decrypted =
            decrypt(key.as_slice(), encrypted.as_slice()).expect("Should decrypt empty data");

        assert_eq!(
            decrypted, plaintext,
            "Empty data should roundtrip correctly"
        );
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let key = generate_key();
        let plaintext = vec![0u8; 10000]; // 10KB of zeros

        let encrypted =
            encrypt(key.as_slice(), plaintext.as_slice()).expect("Should encrypt large data");
        let decrypted =
            decrypt(key.as_slice(), encrypted.as_slice()).expect("Should decrypt large data");

        assert_eq!(
            decrypted, plaintext,
            "Large data should roundtrip correctly"
        );
    }

    #[test]
    fn test_encrypted_data_includes_nonce() {
        let key = generate_key();
        let plaintext = b"Test";

        let encrypted = encrypt(key.as_slice(), plaintext).expect("Encryption should succeed");

        // Encrypted data should be: nonce (12 bytes) + ciphertext + tag (16 bytes)
        // So minimum length should be 12 + plaintext.len() + 16
        assert!(
            encrypted.len() >= 12 + plaintext.len() + 16,
            "Encrypted data should include nonce and authentication tag"
        );
    }

    #[test]
    fn test_decrypt_with_truncated_data_fails() {
        let key = generate_key();
        let plaintext = b"Test message";

        let encrypted = encrypt(key.as_slice(), plaintext).expect("Encryption should succeed");

        // Try to decrypt with truncated data (less than nonce size)
        let truncated = &encrypted[..5];
        let result = decrypt(key.as_slice(), truncated);

        assert!(result.is_err(), "Decryption of truncated data should fail");
    }
}
