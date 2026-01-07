//! Storage bindings for the Axonotes Storage API.
//!
//! This module provides a client for interacting with the Axonotes Storage backend,
//! which handles encrypted media blob storage with quota management.

#![allow(dead_code)] // Public API - functions will be used by storage module
#![allow(unused_imports)] // Re-exports for public API
#![allow(clippy::wildcard_imports)] // Explicit re-export of types module
#![allow(clippy::doc_markdown)] // URLs and technical terms don't need backticks
#![allow(clippy::missing_errors_doc)] // Error conditions documented in prose
#![allow(clippy::use_self)] // Using full enum name for clarity in match arms
#![allow(clippy::uninlined_format_args)] // Using explicit format args for readability
#![allow(clippy::option_if_let_else)] // if-let-else is more readable for complex conditionals
//!
//! # Example
//!
//! ```rust,no_run
//! use axonotes_app_lib::storage_bindings::{StorageClient, CreateDocumentRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a client
//!     let client = StorageClient::new("http://localhost:8081", "your-jwt-token");
//!
//!     // Check quota
//!     let quota = client.get_quota().await?;
//!     println!("Available: {} bytes", quota.available_bytes);
//!
//!     // Create a document
//!     let request = CreateDocumentRequest {
//!         document_id: "doc-123".to_string(),
//!         public_key: "hex-encoded-ed25519-public-key".to_string(),
//!     };
//!     client.create_document(request).await?;
//!
//!     Ok(())
//! }
//! ```

mod client;
mod error;
mod types;

pub use client::StorageClient;
pub use error::{StorageError, StorageResult};
pub use types::*;
