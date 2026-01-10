//! # High-Level Encryption Module
//!
//! Provides encryption and decryption operations for all application data types.
//! This module builds on the low-level cryptographic primitives in [`crate::crypto`].
//!
//! ## Submodules
//!
//! - **`batch`**: Encryption/decryption of document batches (patches)
//! - **`document`**: Document key and metadata encryption
//! - **`helpers`**: Common encryption utilities (key finding, etc.)
//! - **`live_block`**: Real-time collaborative block encryption
//! - **`user`**: User keypair encryption with password/mnemonic
//! - **`version_tag`**: Version tag encryption for document history
//!
//! ## Encryption Hierarchy
//!
//! ```text
//! User Password/Mnemonic
//!     ↓ (Argon2id)
//! User Private Keys (Ed25519 + X25519)
//!     ↓ (X25519 key exchange)
//! Document Keys (per-document, rotated on user removal)
//!     ↓ (ChaCha20-Poly1305)
//! Document Content (batches, live blocks, metadata, version tags)
//! ```
//!
//! ## Key Rotation
//!
//! When a user is removed from a document, all document keys are rotated:
//! 1. New encryption and signing keys are generated
//! 2. All existing content is re-encrypted with the new key
//! 3. The new key is encrypted to remaining users only

pub(crate) mod batch;
pub(crate) mod conflict_history;
pub(crate) mod document;
pub(crate) mod helpers;
pub(crate) mod key_data;
pub(crate) mod live_block;
pub(crate) mod user;
pub(crate) mod version_tag;
