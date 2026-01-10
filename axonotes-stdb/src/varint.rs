//! # LEB128 Varint Encoding
//!
//! Encodes/decodes unsigned integers using Little Endian Base 128 (LEB128) format.
//!
//! Used for space-efficient storage of `key_index` in STDB tables:
//! - Values 0-127: 1 byte
//! - Values 128-16383: 2 bytes
//! - Values 16384-2097151: 3 bytes

/// Encode u32 as LEB128 varint
pub fn encode(value: u32) -> Vec<u8> {
    let mut result = Vec::new();
    let mut val = value;
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80; // Set continuation bit
        }
        result.push(byte);
        if val == 0 {
            break;
        }
    }
    result
}

/// Decode LEB128 varint to u32
pub fn decode(bytes: &[u8]) -> Result<u32, String> {
    if bytes.is_empty() {
        return Err("Empty varint".to_string());
    }
    let mut result: u32 = 0;
    let mut shift = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        if shift == 28 && (byte & 0x70) != 0 {
            // At shift 28, only lower 4 bits (0x0F) are valid for u32
            return Err("Varint too large for u32".to_string());
        }
        if shift > 28 {
            return Err("Varint too large for u32".to_string());
        }
        result |= ((byte & 0x7F) as u32) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
        if i >= 4 {
            return Err("Varint too long".to_string());
        }
    }
    Err("Incomplete varint".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        for val in [0, 1, 127, 128, 255, 16383, 16384, u32::MAX] {
            assert_eq!(decode(&encode(val)).unwrap(), val);
        }
    }

    #[test]
    fn test_encoding_sizes() {
        assert_eq!(encode(0).len(), 1);
        assert_eq!(encode(127).len(), 1);
        assert_eq!(encode(128).len(), 2);
        assert_eq!(encode(16383).len(), 2);
        assert_eq!(encode(16384).len(), 3);
    }

    #[test]
    fn test_empty_input() {
        assert!(decode(&[]).is_err());
    }

    #[test]
    fn test_specific_encodings() {
        assert_eq!(encode(0), vec![0x00]);
        assert_eq!(encode(127), vec![0x7F]);
        assert_eq!(encode(128), vec![0x80, 0x01]);
        assert_eq!(encode(300), vec![0xAC, 0x02]);
    }
}
