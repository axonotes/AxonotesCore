//! # Batch Handler Module
//!
//! Manages document blocks and batches for CRDT-like collaborative editing.
//!
//! ## Architecture
//!
//! Documents are composed of **blocks** (paragraphs, headings, lists, etc.).
//! Changes to blocks are recorded as **batches** containing patches.
//!
//! ```text
//! Document
//!   └── Block 1 (paragraph)
//!   │     └── Batch A (initial content)
//!   │     └── Batch B (edit: insert text)
//!   │     └── Batch C (edit: delete text)
//!   └── Block 2 (heading)
//!         └── Batch D (initial content)
//! ```
//!
//! ## Submodules
//!
//! - **`block_getter`**: Retrieves and reconstructs blocks from batches
//! - **`block_setter`**: Creates new batches from block changes
//! - **`block_type_helpers`**: Type conversion and validation utilities
//! - **`block_types`**: Block type definitions (paragraph, heading, list, etc.)
//! - **`sync`**: Synchronization logic between local and remote batches
//!
//! ## Offline Support
//!
//! Batches are stored locally first, then synced to SpacetimeDB when online.
//! The system handles merge conflicts through timestamp-based ordering.

mod block_benchmarks;
pub(crate) mod block_getter;
pub(crate) mod block_setter;
mod block_tests;
pub(crate) mod block_type_helpers;
pub(crate) mod block_types;
pub(crate) mod sync;
