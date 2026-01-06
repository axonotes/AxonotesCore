//! # Cryptographic Primitives
//!
//! This module provides all cryptographic operations used throughout the application.
//!
//! ## Submodules
//!
//! - **`bip39`**: BIP-39 mnemonic phrase generation and validation for key recovery
//! - **`chacha`**: ChaCha20-Poly1305 authenticated encryption for symmetric encryption
//! - **`ed25519`**: Ed25519 digital signatures for authentication and integrity
//! - **`hash`**: Argon2id password hashing and key derivation
//! - **`x25519`**: X25519 Diffie-Hellman key exchange for asymmetric encryption
//!
//! ## Security Model
//!
//! The application uses a layered encryption approach:
//! 1. User passwords are hashed with Argon2id to derive encryption keys
//! 2. User keypairs (Ed25519 for signing, X25519 for encryption) are encrypted with derived keys
//! 3. Document keys are generated per-document and encrypted to authorized users
//! 4. All document content is encrypted with ChaCha20-Poly1305 using document keys

pub(crate) mod bip39;
pub(crate) mod chacha;
pub(crate) mod ed25519;
pub(crate) mod hash;
pub(crate) mod x25519;
