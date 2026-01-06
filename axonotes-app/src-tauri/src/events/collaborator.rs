//! Collaborator events
//!
//! Events for user permissions and collaboration changes.

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_COLLABORATOR_ADDED: &str = "collaborator-added";
pub const EVENT_COLLABORATOR_REMOVED: &str = "collaborator-removed";
pub const EVENT_COLLABORATOR_ROLE_CHANGED: &str = "collaborator-role-changed";
pub const EVENT_OWNERSHIP_TRANSFERRED: &str = "ownership-transferred";

// ==========================================
// Payloads
// ==========================================

/// Emitted when a collaborator is added to a document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollaboratorAddedPayload {
    pub doc_id: String,
    pub user_id: String,
    pub role: String,
}

/// Emitted when a collaborator is removed from a document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollaboratorRemovedPayload {
    pub doc_id: String,
    pub user_id: String,
}

/// Emitted when a collaborator's role is changed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollaboratorRoleChangedPayload {
    pub doc_id: String,
    pub user_id: String,
    pub old_role: String,
    pub new_role: String,
}

/// Emitted when document ownership is transferred
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipTransferredPayload {
    pub doc_id: String,
    pub old_owner_id: String,
    pub new_owner_id: String,
}

// ==========================================
// Emitters
// ==========================================

pub fn emit_collaborator_added(doc_id: String, user_id: String, role: String) {
    let payload = CollaboratorAddedPayload {
        doc_id,
        user_id,
        role,
    };
    if let Err(e) = app_handle::emit(EVENT_COLLABORATOR_ADDED, &payload) {
        eprintln!("Failed to emit {}: {}", EVENT_COLLABORATOR_ADDED, e);
    }
}

pub fn emit_collaborator_removed(doc_id: String, user_id: String) {
    let payload = CollaboratorRemovedPayload { doc_id, user_id };
    if let Err(e) = app_handle::emit(EVENT_COLLABORATOR_REMOVED, &payload) {
        eprintln!("Failed to emit {}: {}", EVENT_COLLABORATOR_REMOVED, e);
    }
}

pub fn emit_collaborator_role_changed(
    doc_id: String,
    user_id: String,
    old_role: String,
    new_role: String,
) {
    let payload = CollaboratorRoleChangedPayload {
        doc_id,
        user_id,
        old_role,
        new_role,
    };
    if let Err(e) = app_handle::emit(EVENT_COLLABORATOR_ROLE_CHANGED, &payload) {
        eprintln!("Failed to emit {}: {}", EVENT_COLLABORATOR_ROLE_CHANGED, e);
    }
}

pub fn emit_ownership_transferred(doc_id: String, old_owner_id: String, new_owner_id: String) {
    let payload = OwnershipTransferredPayload {
        doc_id,
        old_owner_id,
        new_owner_id,
    };
    if let Err(e) = app_handle::emit(EVENT_OWNERSHIP_TRANSFERRED, &payload) {
        eprintln!("Failed to emit {}: {}", EVENT_OWNERSHIP_TRANSFERRED, e);
    }
}
