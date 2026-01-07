//! # Centralized Event Emission
//!
//! All Tauri events emitted to the frontend are defined here.
//! Events are organized by category for easy discovery.
//!
//! ## Architecture
//!
//! Events flow from backend to frontend via Tauri's event system:
//!
//! 1. Backend operation occurs (sync, share join, lock acquired, etc.)
//! 2. Appropriate `emit_*` function is called
//! 3. Event is serialized and sent to all frontend listeners
//! 4. Frontend handles the event (update UI, show notification, etc.)
//!
//! ## Event Naming Convention
//!
//! Event names follow the pattern: `{category}-{action}`
//! - `stdb-connected`, `stdb-disconnected`
//! - `document-created`, `document-deleted`
//! - `share-code-ready`, `share-user-added`
//!
//! ## Event Categories
//!
//! | Category | Description |
//! |----------|-------------|
//! | `connection` | SpacetimeDB connection lifecycle |
//! | `document` | Document CRUD operations |
//! | `collaborator` | User permission and collaboration changes |
//! | `share` | Share session workflow events |
//! | `lock` | Real-time block lock state changes |
//! | `version_tag` | Version tag create/delete |
//! | `sync` | Batch sync progress and completion |
//! | `key_rotation` | Document key rotation for security |
//!
//! ## Usage
//!
//! ```ignore
//! use crate::events::emit_document_created;
//!
//! // After creating a document
//! emit_document_created(doc_id);
//! ```
//!
//! ## Error Handling
//!
//! All emitters silently log failures to stderr. Event emission failures
//! should not halt backend operations - the event is supplementary.

mod collaborator;
mod connection;
mod document;
mod key_rotation;
mod lock;
mod share;
mod sync;
mod version_tag;

pub use collaborator::*;
pub use connection::*;
pub use document::*;
pub use key_rotation::*;
pub use lock::*;
pub use share::*;
pub use sync::*;
pub use version_tag::*;
