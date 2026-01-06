//! # Ed25519 Digital Signatures
//!
//! Provides Ed25519 key generation, signing, and verification for authentication
//! and data integrity.
//!
//! ## Features
//!
//! - **32-byte keys**: Compact key sizes (32 bytes private, 32 bytes public)
//! - **64-byte signatures**: Deterministic signatures for the same message
//! - **Fast verification**: Batch verification support (via ed25519-dalek)
//!
//! ## Use Cases
//!
//! - Signing batch uploads to prove authorship
//! - Verifying document key rotation requests
//! - Authenticating user key updates
//!
//! ## Usage
//!
//! ```ignore
//! let (private_key, public_key) = generate_ed25519_keys();
//! let signature = sign_message(&private_key, b"message")?;
//! let valid = verify_signature(&public_key, b"message", &signature)?;
//! ```

#![allow(dead_code)]

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand_core::OsRng;

/// Generates a new Ed25519 keypair for digital signatures.
///
/// Uses the operating system's secure random number generator.
///
/// # Returns
///
/// A tuple of `(private_key, public_key)`, both 32 bytes.
/// - The private key should be kept secret and encrypted at rest
/// - The public key can be shared freely for signature verification
pub fn generate_ed25519_keys() -> ([u8; 32], [u8; 32]) {
    let signing_key = SigningKey::generate(&mut OsRng);
    let verifying_key = signing_key.verifying_key();

    let private_bytes = signing_key.to_bytes();
    let public_bytes = verifying_key.to_bytes();

    (private_bytes, public_bytes)
}

/// Signs a message using an Ed25519 private key.
///
/// Ed25519 signatures are deterministic: signing the same message with the
/// same key always produces the same signature.
///
/// # Arguments
///
/// * `private_key` - The 32-byte Ed25519 private key
/// * `message` - The message bytes to sign (any length)
///
/// # Returns
///
/// A 64-byte signature.
///
/// # Errors
///
/// Returns an error if the private key is malformed.
pub fn sign_message(
    private_key: &[u8; 32],
    message: &[u8],
) -> Result<[u8; 64], Box<dyn std::error::Error>> {
    let signing_key = SigningKey::from_bytes(private_key);
    let signature = signing_key.sign(message);
    Ok(signature.to_bytes())
}

/// Verifies an Ed25519 signature against a message and public key.
///
/// # Arguments
///
/// * `public_key` - The 32-byte Ed25519 public key of the signer
/// * `message` - The original message that was signed
/// * `signature` - The 64-byte signature to verify
///
/// # Returns
///
/// `true` if the signature is valid, `false` otherwise.
///
/// # Errors
///
/// Returns an error if the public key is malformed (not a valid curve point).
pub fn verify_signature(
    public_key: &[u8; 32],
    message: &[u8],
    signature: &[u8; 64],
) -> Result<bool, Box<dyn std::error::Error>> {
    let verifying_key = VerifyingKey::from_bytes(public_key)?;
    let signature = Signature::from_bytes(signature);

    Ok(verifying_key.verify(message, &signature).is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_ed25519_keys_correct_length() {
        let (private_key, public_key) = generate_ed25519_keys();
        assert_eq!(
            private_key.len(),
            32,
            "Ed25519 private key should be 32 bytes"
        );
        assert_eq!(
            public_key.len(),
            32,
            "Ed25519 public key should be 32 bytes"
        );
    }

    #[test]
    fn test_generate_ed25519_keys_are_unique() {
        let (priv1, pub1) = generate_ed25519_keys();
        let (priv2, pub2) = generate_ed25519_keys();
        let (priv3, pub3) = generate_ed25519_keys();

        assert_ne!(priv1, priv2, "Generated private keys should be unique");
        assert_ne!(priv2, priv3, "Generated private keys should be unique");
        assert_ne!(pub1, pub2, "Generated public keys should be unique");
        assert_ne!(pub2, pub3, "Generated public keys should be unique");
    }

    #[test]
    fn test_signature_correct_length() {
        let (private_key, _public_key) = generate_ed25519_keys();
        let message = b"Test message";

        let signature = sign_message(&private_key, message).expect("Signing should succeed");
        assert_eq!(signature.len(), 64, "Ed25519 signature should be 64 bytes");
    }

    #[test]
    fn test_sign_verify_roundtrip() {
        let (private_key, public_key) = generate_ed25519_keys();
        let message = b"Hello, this is a signed message!";

        // Sign the message
        let signature = sign_message(&private_key, message).expect("Signing should succeed");

        // Verify the signature
        let is_valid = verify_signature(&public_key, message, &signature)
            .expect("Verification should succeed");

        assert!(is_valid, "Signature should be valid");
    }

    #[test]
    fn test_verify_with_wrong_public_key_fails() {
        let (alice_private, _alice_public) = generate_ed25519_keys();
        let (_bob_private, bob_public) = generate_ed25519_keys();
        let message = b"Message from Alice";

        // Alice signs the message
        let signature = sign_message(&alice_private, message).expect("Signing should succeed");

        // Try to verify with Bob's public key (should fail)
        let is_valid = verify_signature(&bob_public, message, &signature)
            .expect("Verification should succeed");

        assert!(
            !is_valid,
            "Signature should not be valid with wrong public key"
        );
    }

    #[test]
    fn test_verify_with_modified_message_fails() {
        let (private_key, public_key) = generate_ed25519_keys();
        let original_message = b"Original message";
        let modified_message = b"Modified message";

        // Sign the original message
        let signature =
            sign_message(&private_key, original_message).expect("Signing should succeed");

        // Try to verify the signature with a different message (should fail)
        let is_valid = verify_signature(&public_key, modified_message, &signature)
            .expect("Verification should succeed");

        assert!(
            !is_valid,
            "Signature should not be valid for modified message"
        );
    }

    #[test]
    fn test_verify_with_corrupted_signature_fails() {
        let (private_key, public_key) = generate_ed25519_keys();
        let message = b"Important message";

        // Sign the message
        let mut signature = sign_message(&private_key, message).expect("Signing should succeed");

        // Corrupt the signature
        signature[0] ^= 0xFF;

        // Try to verify the corrupted signature (should fail)
        let is_valid = verify_signature(&public_key, message, &signature)
            .expect("Verification should succeed");

        assert!(!is_valid, "Corrupted signature should not be valid");
    }

    #[test]
    fn test_sign_verify_empty_message() {
        let (private_key, public_key) = generate_ed25519_keys();
        let message = b"";

        let signature = sign_message(&private_key, message).expect("Should sign empty message");
        let is_valid = verify_signature(&public_key, message, &signature)
            .expect("Should verify empty message");

        assert!(is_valid, "Empty message signature should be valid");
    }

    #[test]
    fn test_sign_verify_large_message() {
        let (private_key, public_key) = generate_ed25519_keys();
        let message = vec![0xABu8; 10000]; // 10KB of data

        let signature =
            sign_message(&private_key, message.as_slice()).expect("Should sign large message");
        let is_valid = verify_signature(&public_key, message.as_slice(), &signature)
            .expect("Should verify large message");

        assert!(is_valid, "Large message signature should be valid");
    }

    #[test]
    fn test_deterministic_signatures() {
        let (private_key, public_key) = generate_ed25519_keys();
        let message = b"Same message";

        // Sign the same message twice
        let signature1 = sign_message(&private_key, message).expect("Signing should succeed");
        let signature2 = sign_message(&private_key, message).expect("Signing should succeed");

        assert_eq!(
            signature1, signature2,
            "Ed25519 signatures should be deterministic"
        );

        // Both signatures should verify
        let is_valid1 = verify_signature(&public_key, message, &signature1)
            .expect("Verification should succeed");
        let is_valid2 = verify_signature(&public_key, message, &signature2)
            .expect("Verification should succeed");

        assert!(is_valid1, "First signature should be valid");
        assert!(is_valid2, "Second signature should be valid");
    }

    #[test]
    fn test_multiple_messages_same_keypair() {
        let (private_key, public_key) = generate_ed25519_keys();
        let message1 = b"First message";
        let message2 = b"Second message";
        let message3 = b"Third message";

        // Sign multiple messages
        let sig1 = sign_message(&private_key, message1).expect("Signing should succeed");
        let sig2 = sign_message(&private_key, message2).expect("Signing should succeed");
        let sig3 = sign_message(&private_key, message3).expect("Signing should succeed");

        // All signatures should be different
        assert_ne!(
            sig1, sig2,
            "Different messages should have different signatures"
        );
        assert_ne!(
            sig2, sig3,
            "Different messages should have different signatures"
        );
        assert_ne!(
            sig1, sig3,
            "Different messages should have different signatures"
        );

        // All signatures should verify
        assert!(
            verify_signature(&public_key, message1, &sig1).expect("Verification should succeed"),
            "First signature should be valid"
        );
        assert!(
            verify_signature(&public_key, message2, &sig2).expect("Verification should succeed"),
            "Second signature should be valid"
        );
        assert!(
            verify_signature(&public_key, message3, &sig3).expect("Verification should succeed"),
            "Third signature should be valid"
        );
    }

    #[test]
    fn test_cross_message_signature_swap_fails() {
        let (alice_private, alice_public) = generate_ed25519_keys();
        let message_a = b"Message A";
        let message_b = b"Message B";

        // Sign both messages
        let sig_a = sign_message(&alice_private, message_a).expect("Signing should succeed");
        let sig_b = sign_message(&alice_private, message_b).expect("Signing should succeed");

        // Try to verify message A with signature B (should fail)
        let is_valid = verify_signature(&alice_public, message_a, &sig_b)
            .expect("Verification should succeed");
        assert!(!is_valid, "Wrong signature should not validate");

        // Try to verify message B with signature A (should fail)
        let is_valid = verify_signature(&alice_public, message_b, &sig_a)
            .expect("Verification should succeed");
        assert!(!is_valid, "Wrong signature should not validate");
    }

    #[test]
    fn test_signature_with_unicode_message() {
        let (private_key, public_key) = generate_ed25519_keys();
        let message = "Hello 世界 🌍 Здравствуй мир".as_bytes();

        let signature = sign_message(&private_key, message).expect("Signing should succeed");
        let is_valid = verify_signature(&public_key, message, &signature)
            .expect("Verification should succeed");

        assert!(is_valid, "Unicode message signature should be valid");
    }
}
