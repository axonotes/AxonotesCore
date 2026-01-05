use crate::batch_handler::block_types::Block;
use crate::encryption::batch::Patch;

pub fn encode_patch(
    tag: usize,
    time_delta: u8,
    base: &Block,
    new: &Block,
) -> Result<Patch, String> {
    let base_bytes: Vec<u8> = serde_json::to_vec(base).map_err(|e| e.to_string())?;
    let new_bytes: Vec<u8> = serde_json::to_vec(new).map_err(|e| e.to_string())?;

    let delta = xpatch::encode(tag, base_bytes.as_slice(), new_bytes.as_slice(), true);

    Ok(Patch { delta, time_delta })
}

pub fn encode_initial_patch(tag: usize, time_delta: u8, new: &Block) -> Result<Patch, String> {
    let new_bytes: Vec<u8> = serde_json::to_vec(new).map_err(|e| e.to_string())?;

    let delta = xpatch::encode(tag, &[], new_bytes.as_slice(), true);

    Ok(Patch { delta, time_delta })
}

/// returns new block and time_delta
pub fn decode_patch(base: &Block, patch: &Patch) -> Result<(Block, u8), String> {
    let base_bytes: Vec<u8> = serde_json::to_vec(base).map_err(|e| e.to_string())?;

    let new_bytes = xpatch::decode(base_bytes.as_slice(), patch.delta.as_slice())?;

    let block: Block = serde_json::from_slice(new_bytes.as_slice()).map_err(|e| e.to_string())?;

    Ok((block, patch.time_delta))
}

/// returns new block and time_delta
pub fn decode_initial_patch(patch: &Patch) -> Result<(Block, u8), String> {
    let new_bytes = xpatch::decode(&[], patch.delta.as_slice())?;

    let block: Block = serde_json::from_slice(new_bytes.as_slice()).map_err(|e| e.to_string())?;

    Ok((block, patch.time_delta))
}

// ============ Block Definition Macro ============
#[macro_export]
macro_rules! define_blocks {
    ($(
        $variant:ident => $type:ty, $name:literal, $version:literal
    );* $(;)?) => {
        /// All possible block content types
        #[derive(Clone, Debug)]
        pub enum BlockContent {
            $($variant($type),)*
        }

        /// Full block with ID and timestamp
        #[derive(Clone, Debug)]
        pub struct Block {
            pub id: u64,
            pub timestamp: u64,
            pub content: BlockContent,
        }

        // --- Deserialization ---

        #[derive(Deserialize)]
        struct RawBlock {
            id: u64,
            timestamp: u64,
            block_type: String,
            block_type_version: u32,
            #[serde(flatten)]
            rest: Value,
        }

        impl TryFrom<RawBlock> for Block {
            type Error = String;

            fn try_from(raw: RawBlock) -> Result<Self, Self::Error> {
                let content = match (raw.block_type.as_str(), raw.block_type_version) {
                    $(
                        ($name, $version) => {
                            let data: $type = serde_json::from_value(raw.rest)
                                .map_err(|e| format!("failed to parse {}: {}", $name, e))?;
                            BlockContent::$variant(data)
                        }
                    )*
                    (t, v) => return Err(format!("unknown block type: {} v{}", t, v)),
                };

                Ok(Block {
                    id: raw.id,
                    timestamp: raw.timestamp,
                    content,
                })
            }
        }

        impl<'de> Deserialize<'de> for Block {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let raw = RawBlock::deserialize(deserializer)?;
                Block::try_from(raw).map_err(serde::de::Error::custom)
            }
        }

        // --- Serialization ---

        impl Serialize for Block {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::Error;

                let (type_name, version, content_value) = match &self.content {
                    $(
                        BlockContent::$variant(data) => {
                            let v = serde_json::to_value(data).map_err(S::Error::custom)?;
                            ($name, $version, v)
                        }
                    )*
                };

                let mut map = match content_value {
                    Value::Object(m) => m,
                    _ => return Err(S::Error::custom("block content must serialize to object")),
                };

                map.insert("id".into(), serde_json::json!(self.id));
                map.insert("timestamp".into(), serde_json::json!(self.timestamp));
                map.insert("block_type".into(), serde_json::json!(type_name));
                map.insert("block_type_version".into(), serde_json::json!(version));

                map.serialize(serializer)
            }
        }

        // --- Helper Methods ---

        impl BlockContent {
            pub fn type_name(&self) -> &'static str {
                match self {
                    $(BlockContent::$variant(_) => $name,)*
                }
            }

            pub fn type_version(&self) -> u32 {
                match self {
                    $(BlockContent::$variant(_) => $version,)*
                }
            }
        }

        impl Block {
            pub fn new(id: u64, timestamp: u64, content: BlockContent) -> Self {
                Self { id, timestamp, content }
            }

            pub fn type_name(&self) -> &'static str {
                self.content.type_name()
            }

            pub fn type_version(&self) -> u32 {
                self.content.type_version()
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch_handler::block_types::*;
    use spacetimedb_sdk::Identity;

    fn make_paragraph(id: u64, text: &str) -> Block {
        Block::new(
            id,
            1234567890,
            BlockContent::ParagraphV1(ParagraphV1 {
                deleted: None, // not present initially
                author: Identity::from_byte_array([0u8; 32]),
                group_id: "main".to_string(),
                group_row: "0".to_string(),
                text: text.to_string(),
                formatting: vec![],
            }),
        )
    }

    fn make_deleted_paragraph(id: u64, text: &str) -> Block {
        Block::new(
            id,
            1234567890,
            BlockContent::ParagraphV1(ParagraphV1 {
                deleted: Some(true),
                author: Identity::from_byte_array([0u8; 32]),
                group_id: "main".to_string(),
                group_row: "0".to_string(),
                text: text.to_string(),
                formatting: vec![],
            }),
        )
    }

    #[test]
    fn block_serialize_deserialize_roundtrip() {
        let block = make_paragraph(1, "Hello world");

        let json = serde_json::to_string(&block).unwrap();
        let parsed: Block = serde_json::from_str(json.as_str()).unwrap();

        assert_eq!(block.id, parsed.id);
        assert_eq!(block.timestamp, parsed.timestamp);
        assert_eq!(block.type_name(), "paragraph");
        assert_eq!(block.type_version(), 1);
    }

    #[test]
    fn block_json_has_required_fields() {
        let block = make_paragraph(42, "Test");

        let value: serde_json::Value = serde_json::to_value(&block).unwrap();

        assert_eq!(value["id"], 42);
        assert_eq!(value["block_type"], "paragraph");
        assert_eq!(value["block_type_version"], 1);
        assert_eq!(value["text"], "Test");
    }

    #[test]
    fn new_block_has_no_deleted_field() {
        let block = make_paragraph(1, "Fresh block");

        let value: serde_json::Value = serde_json::to_value(&block).unwrap();

        // deleted field should be null/absent for new blocks
        assert!(
            value.get("deleted").is_none() || value["deleted"].is_null(),
            "new block should not have deleted field set"
        );
    }

    #[test]
    fn deleted_block_has_deleted_field() {
        let block = make_deleted_paragraph(1, "Deleted block");

        let value: serde_json::Value = serde_json::to_value(&block).unwrap();

        assert_eq!(
            value["deleted"], true,
            "deleted block should have deleted: true"
        );
    }

    #[test]
    fn deletion_adds_deleted_field_via_patch() {
        let base = make_paragraph(1, "Hello");

        // simulate deletion by creating a new version with deleted: true
        let deleted = make_deleted_paragraph(1, "Hello");

        let patch = encode_patch(0, 50, &base, &deleted).unwrap();
        let (decoded, time_delta) = decode_patch(&base, &patch).unwrap();

        assert_eq!(time_delta, 50);

        if let BlockContent::ParagraphV1(p) = &decoded.content {
            assert_eq!(
                p.deleted,
                Some(true),
                "decoded block should be marked deleted"
            );
            assert_eq!(p.text, "Hello", "text should be unchanged");
        } else {
            panic!("wrong block type");
        }
    }

    #[test]
    fn patch_encode_decode_roundtrip() {
        let base = make_paragraph(1, "Hello");
        let new = make_paragraph(1, "Hello world");

        let patch = encode_patch(0, 100, &base, &new).unwrap();
        let (decoded, time_delta) = decode_patch(&base, &patch).unwrap();

        assert_eq!(time_delta, 100);
        assert_eq!(decoded.id, new.id);

        if let BlockContent::ParagraphV1(p) = &decoded.content {
            assert_eq!(p.text, "Hello world");
            assert_eq!(p.deleted, None, "edited block should not be deleted");
        } else {
            panic!("wrong block type");
        }
    }

    #[test]
    fn patch_is_small_for_minor_edit() {
        let base = make_paragraph(1, "Hello");
        let new = make_paragraph(1, "Hello!");

        let patch = encode_patch(0, 0, &base, &new).unwrap();

        // Delta should be small for a 1-char edit
        assert!(
            patch.delta.len() < 50,
            "delta too large: {} bytes",
            patch.delta.len()
        );
    }

    #[test]
    fn unknown_block_type_fails() {
        let json = r#"{
            "id": 1,
            "timestamp": 123,
            "block_type": "nonexistent",
            "block_type_version": 1
        }"#;

        let result: Result<Block, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }
}
