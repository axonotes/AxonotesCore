use chacha20poly1305::aead::{Aead, KeyInit, OsRng};
use chacha20poly1305::{AeadCore, ChaCha20Poly1305, Key};

pub fn generate_key() -> Vec<u8> {
    ChaCha20Poly1305::generate_key(&mut OsRng)
        .as_slice()
        .to_vec()
}

pub fn encrypt(key: &[u8], content: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);
    let enc_content = cipher
        .encrypt(&nonce, content)
        .map_err(|e| format!("Encryption failed: {:?}", e))?;

    let mut encrypted = nonce.as_slice().to_vec();
    encrypted.extend_from_slice(&enc_content);

    Ok(encrypted)
}

pub fn decrypt(key: &[u8], encrypted: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if encrypted.len() < 12 {
        return Err("Encrypted data too short".into());
    }

    let cipher = ChaCha20Poly1305::new(Key::from_slice(key));
    let (nonce, enc_content) = encrypted.split_at(12);

    let plaintext = cipher
        .decrypt(nonce.into(), enc_content)
        .map_err(|e| format!("Decryption failed: {:?}", e))?;

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

        let encrypted = encrypt(&key.as_slice(), plaintext).expect("Encryption should succeed");
        let decrypted =
            decrypt(&key.as_slice(), &encrypted.as_slice()).expect("Decryption should succeed");

        assert_eq!(decrypted, plaintext, "Decrypted text should match original");
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let key = generate_key();
        let plaintext = b"Same message";

        let encrypted1 = encrypt(&key.as_slice(), plaintext).expect("Encryption should succeed");
        let encrypted2 = encrypt(&key.as_slice(), plaintext).expect("Encryption should succeed");

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

        let encrypted = encrypt(&key1.as_slice(), plaintext).expect("Encryption should succeed");
        let result = decrypt(&key2.as_slice(), &encrypted.as_slice());

        assert!(result.is_err(), "Decryption with wrong key should fail");
    }

    #[test]
    fn test_decrypt_with_corrupted_ciphertext_fails() {
        let key = generate_key();
        let plaintext = b"Important data";

        let mut encrypted = encrypt(&key.as_slice(), plaintext).expect("Encryption should succeed");

        // Corrupt the ciphertext (skip nonce, corrupt the actual encrypted data)
        if encrypted.len() > 13 {
            encrypted[13] ^= 0xFF;
        }

        let result = decrypt(&key.as_slice(), &encrypted.as_slice());
        assert!(
            result.is_err(),
            "Decryption of corrupted ciphertext should fail"
        );
    }

    #[test]
    fn test_encrypt_decrypt_empty_data() {
        let key = generate_key();
        let plaintext = b"";

        let encrypted = encrypt(&key.as_slice(), plaintext).expect("Should encrypt empty data");
        let decrypted =
            decrypt(&key.as_slice(), &encrypted.as_slice()).expect("Should decrypt empty data");

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
            encrypt(&key.as_slice(), &plaintext.as_slice()).expect("Should encrypt large data");
        let decrypted =
            decrypt(&key.as_slice(), &encrypted.as_slice()).expect("Should decrypt large data");

        assert_eq!(
            decrypted, plaintext,
            "Large data should roundtrip correctly"
        );
    }

    #[test]
    fn test_encrypted_data_includes_nonce() {
        let key = generate_key();
        let plaintext = b"Test";

        let encrypted = encrypt(&key.as_slice(), plaintext).expect("Encryption should succeed");

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

        let encrypted = encrypt(&key.as_slice(), plaintext).expect("Encryption should succeed");

        // Try to decrypt with truncated data (less than nonce size)
        let truncated = &encrypted[..5];
        let result = decrypt(&key.as_slice(), truncated);

        assert!(result.is_err(), "Decryption of truncated data should fail");
    }
}
