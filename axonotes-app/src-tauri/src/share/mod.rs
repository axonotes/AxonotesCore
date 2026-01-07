//! # Document Sharing Module
//!
//! Handles document sharing workflows and key rotation for collaborative access.
//!
//! ## Share Flow
//!
//! 1. **Owner creates share**: Generates a share code and pending share entry
//! 2. **Invitee joins**: Uses share code to request access
//! 3. **Owner approves**: Encrypts document keys for the new user
//! 4. **Share closes**: Pending share entry is cleaned up
//!
//! ## Submodules
//!
//! - **`key_rotation`**: Key rotation logic when users are removed
//! - **`processor`**: Share request processing and approval
//! - **`sync`**: Real-time sync for share state changes
//! - **`types`**: Share-related type definitions
//!
//! ## Security
//!
//! - Share codes are short-lived and single-use
//! - Document keys are encrypted per-user with X25519
//! - Key rotation ensures removed users lose access to new content

pub(crate) mod key_rotation;
pub(crate) mod processor;
pub(crate) mod sync;
pub(crate) mod types;
