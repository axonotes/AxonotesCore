//! # Document State Management
//!
//! Backend-managed document state that provides a clean API for the frontend.
//! The backend handles all block state reconstruction, live update forwarding,
//! sync integration, lock management, and time-travel — the frontend simply
//! sends user actions and receives block updates through a single event channel.
//!
//! ## API
//!
//! | Command | Purpose |
//! |---------|---------|
//! | `open_document` | Open a document, start receiving updates via event |
//! | `close_document` | Close document, release locks, stop events |
//! | `set_time` | Time-travel; diffs sent via event |
//! | `update_block` | Edit a block; backend handles lock + live + persist |
//! | `create_block` | Create block, returns real ID |
//! | `delete_block` | Soft-delete a block |
//!
//! All block state changes are pushed through the `block_update_{doc_id}` event channel
//! as batched `Vec<BlockUpdate>` payloads.

pub mod handle;
pub mod mirror;
pub mod types;
