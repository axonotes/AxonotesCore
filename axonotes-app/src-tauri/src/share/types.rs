use crate::stdb_bindings::Role;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ==========================================
// Session State
// ==========================================

/// Active share session configuration
/// Stored in memory while a share is active
#[derive(Clone)]
pub struct ActiveShareSession {
    pub doc_id: String,
    pub role: Role,
    pub full_history: bool,
    /// The share code (None until received from server via subscription)
    pub share_code: Option<String>,
    /// Track which share requests have already been processed
    pub processed_requests: HashSet<String>,
}

impl ActiveShareSession {
    pub fn new(doc_id: String, role: Role, full_history: bool) -> Self {
        Self {
            doc_id,
            role,
            full_history,
            share_code: None,
            processed_requests: HashSet::new(),
        }
    }
}

// ==========================================
// Event Payloads
// ==========================================

/// Emitted when a share code is ready after calling create_share
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareCodeReadyPayload {
    pub doc_id: String,
    pub share_code: String,
}

/// Emitted when a user has been successfully added to a document via share
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareUserAddedPayload {
    pub doc_id: String,
    pub user_id: String, // Identity as hex string
}

/// Emitted when an error occurs during share processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareErrorPayload {
    pub doc_id: String,
    pub error: String,
}

/// Emitted when a share session is closed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareClosedPayload {
    pub doc_id: String,
    pub share_code: String,
}
