//! # Key Rotation Events
//!
//! Events for document key rotation operations.
//!
//! ## When Keys Rotate
//!
//! Key rotation occurs when:
//! - User is removed from a shared document
//! - Share with `full_history=false` completes
//! - Owner explicitly rotates keys for security
//!
//! ## Event Types
//!
//! | Event | Trigger | Payload |
//! |-------|---------|---------|
//! | `document-keys-rotated` | New encryption key active | doc_id, new_key_timestamp |
//!
//! ## Security Note
//!
//! After key rotation, old keys remain valid for existing content.
//! New content is encrypted with the rotated key. Users without
//! access to the new key cannot decrypt new content.

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_DOCUMENT_KEYS_ROTATED: &str = "document-keys-rotated";

// ==========================================
// Payloads
// ==========================================

/// Emitted when document keys are rotated
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentKeysRotatedPayload {
    pub doc_id: String,
    pub new_key_timestamp: u128,
}

// ==========================================
// Emitters
// ==========================================

pub fn emit_document_keys_rotated(doc_id: String, new_key_timestamp: u128) {
    let payload = DocumentKeysRotatedPayload {
        doc_id,
        new_key_timestamp,
    };
    if let Err(e) = app_handle::emit(EVENT_DOCUMENT_KEYS_ROTATED, &payload) {
        eprintln!("Failed to emit {EVENT_DOCUMENT_KEYS_ROTATED}: {e}");
    }
}
