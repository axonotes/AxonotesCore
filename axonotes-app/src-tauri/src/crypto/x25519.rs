use rand_core::OsRng;
use x25519_dalek::{PublicKey, StaticSecret};

use super::chacha::{decrypt, encrypt};

pub fn generate_x25519_keys() -> ([u8; 32], [u8; 32]) {
    let private_key = StaticSecret::random_from_rng(OsRng);
    let public_key = PublicKey::from(&private_key);

    let private_bytes = private_key.to_bytes();
    let public_bytes = *public_key.as_bytes();

    (private_bytes, public_bytes)
}

pub fn encrypt_for_recipient(
    my_private_key: &[u8; 32],
    my_public_key: &[u8; 32],
    their_public_key: &[u8; 32],
    content: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    // Reconstruct keys from bytes
    let my_secret = StaticSecret::from(*my_private_key);
    let their_public = PublicKey::from(*their_public_key);

    // Perform X25519 key exchange to get shared secret
    let shared_secret = my_secret.diffie_hellman(&their_public);

    // Encrypt with shared secret
    let encrypted_content = encrypt(shared_secret.as_bytes(), content)?;

    // Prepend sender's public key
    let mut result = my_public_key.to_vec();
    result.extend_from_slice(&encrypted_content);

    Ok(result)
}

pub fn decrypt_from_anyone(
    my_private_key: &[u8; 32],
    encrypted_data: &[u8],
) -> Result<(Vec<u8>, [u8; 32]), Box<dyn std::error::Error>> {
    // Check minimum size: 32 (sender pub) + 12 (nonce) + 16 (tag)
    if encrypted_data.len() < 60 {
        return Err("Encrypted data too short".into());
    }

    // Extract sender's public key (first 32 bytes)
    let sender_public_key: [u8; 32] = encrypted_data[0..32].try_into()?;
    let sender_public = PublicKey::from(sender_public_key);
    let ciphertext = &encrypted_data[32..];

    // Reconstruct your private key
    let my_secret = StaticSecret::from(*my_private_key);

    // Perform key exchange with sender's public key
    let shared_secret = my_secret.diffie_hellman(&sender_public);

    // Decrypt with the shared secret
    let plaintext = decrypt(shared_secret.as_bytes(), ciphertext)?;

    Ok((plaintext, sender_public_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_x25519_keys_correct_length() {
        let (private_key, public_key) = generate_x25519_keys();
        assert_eq!(
            private_key.len(),
            32,
            "X25519 private key should be 32 bytes"
        );
        assert_eq!(public_key.len(), 32, "X25519 public key should be 32 bytes");
    }

    #[test]
    fn test_generate_x25519_keys_are_unique() {
        let (priv1, pub1) = generate_x25519_keys();
        let (priv2, pub2) = generate_x25519_keys();
        let (priv3, pub3) = generate_x25519_keys();

        assert_ne!(priv1, priv2, "Generated private keys should be unique");
        assert_ne!(priv2, priv3, "Generated private keys should be unique");
        assert_ne!(pub1, pub2, "Generated public keys should be unique");
        assert_ne!(pub2, pub3, "Generated public keys should be unique");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let (alice_private, alice_public) = generate_x25519_keys();
        let (bob_private, bob_public) = generate_x25519_keys();
        let plaintext = b"Hello from Alice to Bob!";

        // Alice encrypts for Bob
        let encrypted =
            encrypt_for_recipient(&alice_private, &alice_public, &bob_public, plaintext)
                .expect("Encryption should succeed");

        // Bob decrypts from Alice
        let (decrypted, sender_public) = decrypt_from_anyone(&bob_private, encrypted.as_slice())
            .expect("Decryption should succeed");

        assert_eq!(decrypted, plaintext, "Decrypted text should match original");
        assert_eq!(
            sender_public, alice_public,
            "Should identify correct sender"
        );
    }

    #[test]
    fn test_bidirectional_communication() {
        let (alice_private, alice_public) = generate_x25519_keys();
        let (bob_private, bob_public) = generate_x25519_keys();

        // Alice -> Bob
        let msg_to_bob = b"Hi Bob!";
        let encrypted_to_bob =
            encrypt_for_recipient(&alice_private, &alice_public, &bob_public, msg_to_bob)
                .expect("Encryption should succeed");
        let (decrypted_by_bob, sender1) =
            decrypt_from_anyone(&bob_private, encrypted_to_bob.as_slice())
                .expect("Decryption should succeed");

        assert_eq!(decrypted_by_bob, msg_to_bob);
        assert_eq!(sender1, alice_public);

        // Bob -> Alice
        let msg_to_alice = b"Hi Alice!";
        let encrypted_to_alice =
            encrypt_for_recipient(&bob_private, &bob_public, &alice_public, msg_to_alice)
                .expect("Encryption should succeed");
        let (decrypted_by_alice, sender2) =
            decrypt_from_anyone(&alice_private, encrypted_to_alice.as_slice())
                .expect("Decryption should succeed");

        assert_eq!(decrypted_by_alice, msg_to_alice);
        assert_eq!(sender2, bob_public);
    }

    #[test]
    fn test_decrypt_with_wrong_private_key_fails() {
        let (alice_private, alice_public) = generate_x25519_keys();
        let (_bob_private, bob_public) = generate_x25519_keys();
        let (charlie_private, _charlie_public) = generate_x25519_keys();
        let plaintext = b"Secret message for Bob";

        // Alice encrypts for Bob
        let encrypted =
            encrypt_for_recipient(&alice_private, &alice_public, &bob_public, plaintext)
                .expect("Encryption should succeed");

        // Charlie tries to decrypt (should fail)
        let result = decrypt_from_anyone(&charlie_private, encrypted.as_slice());
        assert!(
            result.is_err(),
            "Decryption with wrong private key should fail"
        );
    }

    #[test]
    fn test_decrypt_with_truncated_data_fails() {
        let (alice_private, alice_public) = generate_x25519_keys();
        let (_bob_private, bob_public) = generate_x25519_keys();
        let plaintext = b"Test message";

        let encrypted =
            encrypt_for_recipient(&alice_private, &alice_public, &bob_public, plaintext)
                .expect("Encryption should succeed");

        // Try to decrypt with truncated data (less than minimum size of 60 bytes)
        let truncated = &encrypted[..30];
        let result = decrypt_from_anyone(&alice_private, truncated);

        assert!(result.is_err(), "Decryption of truncated data should fail");
    }

    #[test]
    fn test_encrypt_decrypt_empty_data() {
        let (alice_private, alice_public) = generate_x25519_keys();
        let (bob_private, bob_public) = generate_x25519_keys();
        let plaintext = b"";

        let encrypted =
            encrypt_for_recipient(&alice_private, &alice_public, &bob_public, plaintext)
                .expect("Should encrypt empty data");
        let (decrypted, _) = decrypt_from_anyone(&bob_private, encrypted.as_slice())
            .expect("Should decrypt empty data");

        assert_eq!(
            decrypted, plaintext,
            "Empty data should roundtrip correctly"
        );
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let (alice_private, alice_public) = generate_x25519_keys();
        let (bob_private, bob_public) = generate_x25519_keys();
        let plaintext = vec![0xABu8; 10000]; // 10KB of data

        let encrypted = encrypt_for_recipient(
            &alice_private,
            &alice_public,
            &bob_public,
            plaintext.as_slice(),
        )
        .expect("Should encrypt large data");
        let (decrypted, _) = decrypt_from_anyone(&bob_private, encrypted.as_slice())
            .expect("Should decrypt large data");

        assert_eq!(
            decrypted, plaintext,
            "Large data should roundtrip correctly"
        );
    }

    #[test]
    fn test_encrypted_data_includes_sender_public_key() {
        let (alice_private, alice_public) = generate_x25519_keys();
        let (_bob_private, bob_public) = generate_x25519_keys();
        let plaintext = b"Test";

        let encrypted =
            encrypt_for_recipient(&alice_private, &alice_public, &bob_public, plaintext)
                .expect("Encryption should succeed");

        // Encrypted data should be: sender public key (32) + nonce (12) + ciphertext + tag (16)
        // Minimum length: 32 + 12 + plaintext.len() + 16
        assert!(
            encrypted.len() >= 32 + 12 + plaintext.len() + 16,
            "Encrypted data should include sender public key, nonce, and authentication tag"
        );
    }

    #[test]
    fn test_same_message_produces_different_ciphertexts() {
        let (alice_private, alice_public) = generate_x25519_keys();
        let (_bob_private, bob_public) = generate_x25519_keys();
        let plaintext = b"Same message";

        let encrypted1 =
            encrypt_for_recipient(&alice_private, &alice_public, &bob_public, plaintext)
                .expect("Encryption should succeed");
        let encrypted2 =
            encrypt_for_recipient(&alice_private, &alice_public, &bob_public, plaintext)
                .expect("Encryption should succeed");

        assert_ne!(
            encrypted1, encrypted2,
            "Same plaintext should produce different ciphertexts (due to random nonces)"
        );
    }

    #[test]
    fn test_decrypt_with_corrupted_ciphertext_fails() {
        let (alice_private, alice_public) = generate_x25519_keys();
        let (bob_private, bob_public) = generate_x25519_keys();
        let plaintext = b"Important data";

        let mut encrypted =
            encrypt_for_recipient(&alice_private, &alice_public, &bob_public, plaintext)
                .expect("Encryption should succeed");

        // Corrupt the ciphertext (skip sender public key + nonce, corrupt encrypted data)
        if encrypted.len() > 50 {
            encrypted[50] ^= 0xFF;
        }

        let result = decrypt_from_anyone(&bob_private, encrypted.as_slice());
        assert!(
            result.is_err(),
            "Decryption of corrupted ciphertext should fail"
        );
    }
}
