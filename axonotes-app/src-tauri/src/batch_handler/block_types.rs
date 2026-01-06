//! # Block Type Definitions
//!
//! Defines the content block types used in documents.
//!
//! ## Architecture
//!
//! Blocks are the fundamental units of document content. Each block type
//! has a versioned struct (e.g., `ParagraphV1`) to allow schema evolution
//! while maintaining backward compatibility.
//!
//! ## Adding New Block Types
//!
//! 1. Define a new struct implementing the block's data model
//! 2. Register it in the `define_blocks!` macro
//! 3. The macro generates serialization, type checking, and dispatch code
//!
//! ## Current Block Types
//!
//! - **`ParagraphV1`**: Rich text paragraph with formatting spans
//! - **`HeadingV1`**: Heading with level (h1-h6)
//! - **`MetadataV1`**: Key-value metadata fields

use crate::define_blocks;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use spacetimedb_sdk::Identity;

// ========== Block Registration ========

// variant => type, name, version_number
define_blocks! {
    ParagraphV1 => ParagraphV1, "paragraph", 1;
    HeadingV1   => HeadingV1,   "heading",   1;
    MetadataV1  => MetadataV1,  "metadata",  1;
}

// ============ Shared Types ============

/// A formatting span within text content.
///
/// Represents a range of characters with a specific format (bold, italic, etc.).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormatSpan {
    /// Start character index (inclusive)
    pub start: usize,
    /// End character index (exclusive)
    pub end: usize,
    /// Format type: "bold", "italic", "code", "link", etc.
    #[serde(rename = "type")]
    pub kind: String,
}

// ============ Block Types ============
// To add a new block:
// 1. Add struct here
// 2. Register in define_blocks! macro

/// A paragraph block with rich text formatting.
///
/// Paragraphs are the most common block type, containing text content
/// with optional formatting spans for bold, italic, links, etc.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParagraphV1 {
    /// The user who last modified this block
    pub author: Identity,
    /// Group identifier for fractional indexing (ordering)
    pub group_id: String,
    /// Row within group for fractional indexing
    pub group_row: String,
    /// The plain text content
    pub text: String,
    /// Formatting spans (bold, italic, etc.)
    #[serde(default)]
    pub formatting: Vec<FormatSpan>,
}

/// A heading block with level indicator.
///
/// Headings provide document structure with levels 1-6 (like h1-h6 in HTML).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadingV1 {
    /// The user who last modified this block
    pub author: Identity,
    /// Group identifier for fractional indexing
    pub group_id: String,
    /// Row within group for fractional indexing
    pub group_row: String,
    /// The heading text
    pub text: String,
    /// Heading level: 1-6 (1 = largest)
    pub level: u8,
}

/// A metadata block for storing document properties.
///
/// Used for document-level metadata like title, author, creation date, etc.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetadataV1 {
    /// The user who last modified this block
    pub author: Identity,
    /// Group identifier for fractional indexing
    pub group_id: String,
    /// Row within group for fractional indexing
    pub group_row: String,
    /// The metadata field name
    pub field: String,
    /// The metadata value (can be any JSON type)
    pub value: Value,
}
