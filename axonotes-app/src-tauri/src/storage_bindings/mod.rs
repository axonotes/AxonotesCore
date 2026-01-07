//! Storage bindings for the Axonotes Storage API.
//!
//! This module provides a client for interacting with the Axonotes Storage backend,
//! which handles encrypted media blob storage with quota management.
//!
//! # Example
//!
//! ```rust,no_run
//! use storage_bindings::{StorageClient, CreateDocumentRequest};
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
