//! # Tauri Commands Module
//!
//! Defines all IPC commands exposed to the frontend via Tauri's invoke system.
//!
//! ## Command Categories
//!
//! - **`auth_cmd`**: Authentication flows (login, logout via WorkOS)
//! - **`block_cmd`**: Block CRUD operations (create, read, update, delete)
//! - **`database_cmd`**: Local database management (lock, unlock, wipe)
//! - **`document_cmd`**: Document lifecycle (create, delete, list, metadata)
//! - **`encryption_cmd`**: Key management (create user, sync keys, update password)
//! - **`live_lock_cmd`**: Real-time block locking for collaborative editing
//! - **`profile_cmd`**: User profile management (switch, refresh token)
//! - **`share_cmd`**: Document sharing (create share, join, permissions)
//! - **`version_tag_cmd`**: Version tagging (create, delete, list tags)
//!
//! ## Error Handling
//!
//! All commands return `Result<T, String>` where errors are human-readable
//! messages suitable for display in the UI.
//!
//! ## Async Execution
//!
//! Most commands are async and use `tokio::task::spawn_blocking` for
//! database operations to avoid blocking the Tauri event loop.

pub(crate) mod auth_cmd;
pub(crate) mod block_cmd;
pub(crate) mod database_cmd;
pub(crate) mod document_cmd;
pub(crate) mod encryption_cmd;
pub(crate) mod live_lock_cmd;
pub(crate) mod profile_cmd;
pub(crate) mod share_cmd;
pub(crate) mod version_tag_cmd;
