//! # Document Operation Events
//!
//! Events for document CRUD operations and access changes.
//!
//! ## Event Types
//!
//! | Event | Trigger | Payload |
//! |-------|---------|---------|
//! | `document-created` | New document created | doc_id |
//! | `document-deleted` | Document deleted | doc_id |
//! | `document-metadata-updated` | Path or tags changed | doc_id, path, tags |
//! | `document-access-granted` | Gained access via share | doc_id |
//! | `document-access-revoked` | Lost access to document | doc_id |
//!
//! ## Frontend Handling
//!
//! - Update document list/tree
//! - Refresh open document if affected
//! - Navigate away if access revoked

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_DOCUMENT_CREATED: &str = "document-created";
pub const EVENT_DOCUMENT_DELETED: &str = "document-deleted";
pub const EVENT_DOCUMENT_METADATA_UPDATED: &str = "document-metadata-updated";
pub const EVENT_DOCUMENT_ACCESS_GRANTED: &str = "document-access-granted";
pub const EVENT_DOCUMENT_ACCESS_REVOKED: &str = "document-access-revoked";

// ==========================================
// Payloads
// ==========================================

/// Emitted when a new document is created
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentCreatedPayload {
    pub doc_id: String,
}

/// Emitted when a document is deleted
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDeletedPayload {
    pub doc_id: String,
}

/// Emitted when document metadata is updated
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMetadataUpdatedPayload {
    pub doc_id: String,
    pub path: String,
    pub tags: Vec<String>,
}

/// Emitted when access to a document is granted (e.g., via share)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentAccessGrantedPayload {
    pub doc_id: String,
}

/// Emitted when access to a document is revoked
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentAccessRevokedPayload {
    pub doc_id: String,
}

// ==========================================
// Emitters
// ==========================================

pub fn emit_document_created(doc_id: String) {
    let payload = DocumentCreatedPayload { doc_id };
    if let Err(e) = app_handle::emit(EVENT_DOCUMENT_CREATED, &payload) {
        eprintln!("Failed to emit {EVENT_DOCUMENT_CREATED}: {e}");
    }
}

pub fn emit_document_deleted(doc_id: String) {
    let payload = DocumentDeletedPayload { doc_id };
    if let Err(e) = app_handle::emit(EVENT_DOCUMENT_DELETED, &payload) {
        eprintln!("Failed to emit {EVENT_DOCUMENT_DELETED}: {e}");
    }
}

pub fn emit_document_metadata_updated(doc_id: String, path: String, tags: Vec<String>) {
    let payload = DocumentMetadataUpdatedPayload { doc_id, path, tags };
    if let Err(e) = app_handle::emit(EVENT_DOCUMENT_METADATA_UPDATED, &payload) {
        eprintln!("Failed to emit {EVENT_DOCUMENT_METADATA_UPDATED}: {e}");
    }
}

pub fn emit_document_access_granted(doc_id: String) {
    let payload = DocumentAccessGrantedPayload { doc_id };
    if let Err(e) = app_handle::emit(EVENT_DOCUMENT_ACCESS_GRANTED, &payload) {
        eprintln!("Failed to emit {EVENT_DOCUMENT_ACCESS_GRANTED}: {e}");
    }
}

pub fn emit_document_access_revoked(doc_id: String) {
    let payload = DocumentAccessRevokedPayload { doc_id };
    if let Err(e) = app_handle::emit(EVENT_DOCUMENT_ACCESS_REVOKED, &payload) {
        eprintln!("Failed to emit {EVENT_DOCUMENT_ACCESS_REVOKED}: {e}");
    }
}
