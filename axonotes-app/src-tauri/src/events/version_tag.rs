//! # Version Tag Events
//!
//! Events for version tag create/delete operations.
//!
//! ## Event Types
//!
//! | Event | Trigger | Payload |
//! |-------|---------|---------|
//! | `version-tag-created` | New tag created | tag_id, doc_id, tag_name, timestamp |
//! | `version-tag-deleted` | Tag removed | tag_id, doc_id |
//!
//! ## Frontend Handling
//!
//! - Update version tag list
//! - Show success notification
//! - Enable "Restore" UI if tags exist

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_VERSION_TAG_CREATED: &str = "version-tag-created";
pub const EVENT_VERSION_TAG_DELETED: &str = "version-tag-deleted";

// ==========================================
// Payloads
// ==========================================

/// Emitted when a version tag is created
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionTagCreatedPayload {
    pub tag_id: String,
    pub doc_id: String,
    pub tag_name: String,
    pub timestamp: u128,
}

/// Emitted when a version tag is deleted
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionTagDeletedPayload {
    pub tag_id: String,
    pub doc_id: String,
}

// ==========================================
// Emitters
// ==========================================

pub fn emit_version_tag_created(tag_id: String, doc_id: String, tag_name: String, timestamp: u128) {
    let payload = VersionTagCreatedPayload {
        tag_id,
        doc_id,
        tag_name,
        timestamp,
    };
    if let Err(e) = app_handle::emit(EVENT_VERSION_TAG_CREATED, &payload) {
        eprintln!("Failed to emit {EVENT_VERSION_TAG_CREATED}: {e}");
    }
}

pub fn emit_version_tag_deleted(tag_id: String, doc_id: String) {
    let payload = VersionTagDeletedPayload { tag_id, doc_id };
    if let Err(e) = app_handle::emit(EVENT_VERSION_TAG_DELETED, &payload) {
        eprintln!("Failed to emit {EVENT_VERSION_TAG_DELETED}: {e}");
    }
}
