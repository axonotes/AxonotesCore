use crate::stdb_bindings::Role;
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
