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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FormatSpan {
    pub start: usize,
    pub end: usize,
    #[serde(rename = "type")]
    pub kind: String,
}

// ============ Block Types ============
// To add a new block:
// 1. Add struct here
// 2. Register in define_blocks! macro

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ParagraphV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    pub author: Identity,
    pub group_id: String,
    pub group_row: String,
    pub text: String,
    #[serde(default)]
    pub formatting: Vec<FormatSpan>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadingV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    pub author: Identity,
    pub group_id: String,
    pub group_row: String,
    pub text: String,
    pub level: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetadataV1 {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,
    pub author: Identity,
    pub group_id: String,
    pub group_row: String,
    pub field: String,
    pub value: Value,
}
