//! Key rotation events
//!
//! Events for document key rotation operations.

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
        eprintln!("Failed to emit {}: {}", EVENT_DOCUMENT_KEYS_ROTATED, e);
    }
}
