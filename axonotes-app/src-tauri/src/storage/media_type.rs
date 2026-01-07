//! Media type enum for blob content identification.
//!
//! The media type is stored as a single byte in the encrypted blob header,
//! allowing efficient content-type detection without exposing metadata.

#![allow(clippy::must_use_candidate)] // Type conversion methods - callers understand return semantics

use serde::{Deserialize, Serialize};

/// Supported media types for blob storage.
///
/// Each variant maps to a single byte value stored in the encrypted header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MediaType {
    // Images
    Png = 0x01,
    Jpeg = 0x02,
    Gif = 0x03,
    Webp = 0x04,
    Svg = 0x05,

    // Videos
    Mp4 = 0x10,
    Webm = 0x11,

    // Audio
    Mp3 = 0x20,
    Ogg = 0x21,

    // Documents
    Pdf = 0x30,

    // Fallback
    Unknown = 0xFF,
}

impl MediaType {
    /// Get the MIME type string for HTTP Content-Type header.
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Webp => "image/webp",
            Self::Svg => "image/svg+xml",
            Self::Mp4 => "video/mp4",
            Self::Webm => "video/webm",
            Self::Mp3 => "audio/mpeg",
            Self::Ogg => "audio/ogg",
            Self::Pdf => "application/pdf",
            Self::Unknown => "application/octet-stream",
        }
    }

    /// Detect media type from file extension.
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "png" => Self::Png,
            "jpg" | "jpeg" => Self::Jpeg,
            "gif" => Self::Gif,
            "webp" => Self::Webp,
            "svg" => Self::Svg,
            "mp4" | "m4v" => Self::Mp4,
            "webm" => Self::Webm,
            "mp3" => Self::Mp3,
            "ogg" | "oga" => Self::Ogg,
            "pdf" => Self::Pdf,
            _ => Self::Unknown,
        }
    }

    /// Detect media type from file magic bytes (fallback detection).
    ///
    /// Returns `Unknown` if the magic bytes don't match any known format.
    pub fn from_magic_bytes(data: &[u8]) -> Self {
        if data.len() < 4 {
            return Self::Unknown;
        }

        // PNG: 89 50 4E 47
        if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            return Self::Png;
        }

        // JPEG: FF D8 FF
        if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Self::Jpeg;
        }

        // GIF: 47 49 46 38
        if data.starts_with(&[0x47, 0x49, 0x46, 0x38]) {
            return Self::Gif;
        }

        // WebP: RIFF....WEBP
        if data.len() >= 12
            && data.starts_with(&[0x52, 0x49, 0x46, 0x46])
            && &data[8..12] == b"WEBP"
        {
            return Self::Webp;
        }

        // SVG: starts with <?xml or <svg (with possible whitespace)
        if let Ok(text) = std::str::from_utf8(&data[..data.len().min(256)]) {
            let trimmed = text.trim_start();
            if trimmed.starts_with("<?xml") || trimmed.starts_with("<svg") {
                return Self::Svg;
            }
        }

        // MP4/M4V: ....ftyp
        if data.len() >= 8 && &data[4..8] == b"ftyp" {
            return Self::Mp4;
        }

        // WebM: 1A 45 DF A3 (EBML header)
        if data.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
            return Self::Webm;
        }

        // MP3: ID3 tag or sync word
        if data.starts_with(&[0x49, 0x44, 0x33]) || data.starts_with(&[0xFF, 0xFB]) {
            return Self::Mp3;
        }

        // Ogg: OggS
        if data.starts_with(&[0x4F, 0x67, 0x67, 0x53]) {
            return Self::Ogg;
        }

        // PDF: %PDF
        if data.starts_with(&[0x25, 0x50, 0x44, 0x46]) {
            return Self::Pdf;
        }

        Self::Unknown
    }

    /// Convert from byte value.
    pub const fn from_byte(byte: u8) -> Self {
        match byte {
            0x01 => Self::Png,
            0x02 => Self::Jpeg,
            0x03 => Self::Gif,
            0x04 => Self::Webp,
            0x05 => Self::Svg,
            0x10 => Self::Mp4,
            0x11 => Self::Webm,
            0x20 => Self::Mp3,
            0x21 => Self::Ogg,
            0x30 => Self::Pdf,
            _ => Self::Unknown,
        }
    }

    /// Convert to byte value.
    pub const fn to_byte(self) -> u8 {
        self as u8
    }

    /// Get file extension for this media type.
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
            Self::Webp => "webp",
            Self::Svg => "svg",
            Self::Mp4 => "mp4",
            Self::Webm => "webm",
            Self::Mp3 => "mp3",
            Self::Ogg => "ogg",
            Self::Pdf => "pdf",
            Self::Unknown => "bin",
        }
    }
}

impl From<u8> for MediaType {
    fn from(byte: u8) -> Self {
        Self::from_byte(byte)
    }
}

impl From<MediaType> for u8 {
    fn from(media_type: MediaType) -> Self {
        media_type.to_byte()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_types() {
        assert_eq!(MediaType::Png.mime_type(), "image/png");
        assert_eq!(MediaType::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(MediaType::Mp4.mime_type(), "video/mp4");
        assert_eq!(MediaType::Pdf.mime_type(), "application/pdf");
        assert_eq!(MediaType::Unknown.mime_type(), "application/octet-stream");
    }

    #[test]
    fn test_from_extension() {
        assert_eq!(MediaType::from_extension("png"), MediaType::Png);
        assert_eq!(MediaType::from_extension("PNG"), MediaType::Png);
        assert_eq!(MediaType::from_extension("jpg"), MediaType::Jpeg);
        assert_eq!(MediaType::from_extension("jpeg"), MediaType::Jpeg);
        assert_eq!(MediaType::from_extension("pdf"), MediaType::Pdf);
        assert_eq!(MediaType::from_extension("PDF"), MediaType::Pdf);
        assert_eq!(MediaType::from_extension("unknown"), MediaType::Unknown);
    }

    #[test]
    fn test_from_magic_bytes() {
        // PNG magic bytes
        let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(MediaType::from_magic_bytes(&png), MediaType::Png);

        // JPEG magic bytes
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(MediaType::from_magic_bytes(&jpeg), MediaType::Jpeg);

        // GIF magic bytes
        let gif = [0x47, 0x49, 0x46, 0x38, 0x39, 0x61];
        assert_eq!(MediaType::from_magic_bytes(&gif), MediaType::Gif);

        // PDF magic bytes (%PDF)
        let pdf = [0x25, 0x50, 0x44, 0x46, 0x2D, 0x31, 0x2E, 0x34];
        assert_eq!(MediaType::from_magic_bytes(&pdf), MediaType::Pdf);

        // Unknown
        let unknown = [0x00, 0x00, 0x00, 0x00];
        assert_eq!(MediaType::from_magic_bytes(&unknown), MediaType::Unknown);
    }

    #[test]
    fn test_byte_conversion() {
        assert_eq!(MediaType::from_byte(0x01), MediaType::Png);
        assert_eq!(MediaType::Png.to_byte(), 0x01);
        assert_eq!(MediaType::from_byte(0x30), MediaType::Pdf);
        assert_eq!(MediaType::Pdf.to_byte(), 0x30);
        assert_eq!(MediaType::from_byte(0xFF), MediaType::Unknown);
        assert_eq!(MediaType::from_byte(0x99), MediaType::Unknown);
    }

    #[test]
    fn test_extension() {
        assert_eq!(MediaType::Png.extension(), "png");
        assert_eq!(MediaType::Jpeg.extension(), "jpg");
        assert_eq!(MediaType::Pdf.extension(), "pdf");
        assert_eq!(MediaType::Unknown.extension(), "bin");
    }
}
