//! # Share Types
//!
//! Data structures for managing active share sessions.
//!
//! ## Session Lifecycle
//!
//! 1. User calls `create_share` → `ActiveShareSession` created
//! 2. Server generates share code → stored in `share_code`
//! 3. Joiners use code → requests tracked in `processed_requests`
//! 4. User calls `close_share` → session removed from memory
//!
//! ## Deduplication
//!
//! The `processed_requests` set prevents double-processing of join
//! requests in case of duplicate subscription callbacks.

use crate::stdb_bindings::Role;
use std::collections::HashSet;

// ==========================================
// Session State
// ==========================================

/// Active share session configuration.
///
/// Stored in memory while a share is active. Tracks the share parameters
/// and which join requests have already been processed.
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
