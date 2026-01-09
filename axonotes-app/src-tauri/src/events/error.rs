//! # Error Events
//!
//! Events for reporting errors and bugs to the frontend.
//!
//! ## Event Types
//!
//! | Event | Severity | Description |
//! |-------|----------|-------------|
//! | `error-critical` | Critical | Unrecoverable errors requiring user attention |
//! | `error-warning` | Warning | Issues that may affect functionality |
//! | `error-bug` | Bug | Unexpected behavior that should be reported |
//!
//! ## Frontend Handling
//!
//! - Show error dialog for critical errors
//! - Log warnings for debugging
//! - Provide "Report Bug" option for bug events

use crate::app_handle;
use serde::{Deserialize, Serialize};

// ==========================================
// Event Names
// ==========================================

pub const EVENT_ERROR_CRITICAL: &str = "error-critical";
pub const EVENT_ERROR_WARNING: &str = "error-warning";
pub const EVENT_ERROR_BUG: &str = "error-bug";

// ==========================================
// Payloads
// ==========================================

/// Severity level for error events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    Critical,
    Warning,
    Bug,
}

/// Emitted when a critical error occurs that requires user attention
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CriticalErrorPayload {
    /// Error code for categorization
    pub code: String,
    /// Human-readable error message
    pub message: String,
    /// Technical details for debugging
    pub details: Option<String>,
    /// Suggested action for the user
    pub action: Option<String>,
}

/// Emitted when a warning occurs that may affect functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WarningPayload {
    /// Warning code for categorization
    pub code: String,
    /// Human-readable warning message
    pub message: String,
    /// Technical details for debugging
    pub details: Option<String>,
}

/// Emitted when unexpected behavior is detected that should be reported
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BugReportPayload {
    /// Bug code for categorization
    pub code: String,
    /// Description of what went wrong
    pub message: String,
    /// Technical details for debugging
    pub details: Option<String>,
    /// Context about what operation was being performed
    pub context: Option<String>,
}

// ==========================================
// Error Codes
// ==========================================

/// Well-known error codes for categorization
pub mod codes {
    /// Identity mismatch between JWT-derived and STDB-provided identity
    pub const IDENTITY_MISMATCH: &str = "IDENTITY_MISMATCH";
    /// Failed to derive identity from JWT
    pub const IDENTITY_DERIVATION_FAILED: &str = "IDENTITY_DERIVATION_FAILED";
    /// Database corruption or inconsistency detected
    pub const DATABASE_CORRUPTION: &str = "DATABASE_CORRUPTION";
    /// Encryption/decryption failure
    pub const CRYPTO_ERROR: &str = "CRYPTO_ERROR";
    /// Sync conflict that couldn't be resolved
    pub const SYNC_CONFLICT: &str = "SYNC_CONFLICT";
}

// ==========================================
// Emitters
// ==========================================

pub fn emit_critical_error(code: &str, message: &str, details: Option<&str>, action: Option<&str>) {
    let payload = CriticalErrorPayload {
        code: code.to_string(),
        message: message.to_string(),
        details: details.map(|s| s.to_string()),
        action: action.map(|s| s.to_string()),
    };
    log::error!("[CRITICAL] {}: {} - {:?}", code, message, details);
    if let Err(e) = app_handle::emit(EVENT_ERROR_CRITICAL, &payload) {
        eprintln!("Failed to emit {EVENT_ERROR_CRITICAL}: {e}");
    }
}

pub fn emit_warning(code: &str, message: &str, details: Option<&str>) {
    let payload = WarningPayload {
        code: code.to_string(),
        message: message.to_string(),
        details: details.map(|s| s.to_string()),
    };
    log::warn!("[WARNING] {}: {} - {:?}", code, message, details);
    if let Err(e) = app_handle::emit(EVENT_ERROR_WARNING, &payload) {
        eprintln!("Failed to emit {EVENT_ERROR_WARNING}: {e}");
    }
}

pub fn emit_bug_report(code: &str, message: &str, details: Option<&str>, context: Option<&str>) {
    let payload = BugReportPayload {
        code: code.to_string(),
        message: message.to_string(),
        details: details.map(|s| s.to_string()),
        context: context.map(|s| s.to_string()),
    };
    log::error!(
        "[BUG] {}: {} - {:?} (context: {:?})",
        code,
        message,
        details,
        context
    );
    if let Err(e) = app_handle::emit(EVENT_ERROR_BUG, &payload) {
        eprintln!("Failed to emit {EVENT_ERROR_BUG}: {e}");
    }
}
