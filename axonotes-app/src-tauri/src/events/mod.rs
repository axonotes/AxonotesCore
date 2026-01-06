//! Centralized event emission module
//!
//! All Tauri events emitted to the frontend are defined here.
//! Events are organized by category for easy discovery.
//!
//! # Event Categories
//! - `connection` - SpacetimeDB connection status events
//! - `document` - Document CRUD operation events
//! - `collaborator` - User permission and collaboration events
//! - `share` - Share session events (code ready, user joined, etc.)
//! - `lock` - Live block lock events
//! - `version_tag` - Version tag events
//! - `sync` - Batch sync status events
//! - `key_rotation` - Document key rotation events

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
