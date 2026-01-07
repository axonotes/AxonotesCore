//! Chunked streaming encryption/decryption for blob storage.
//!
//! ## Blob Format
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Base Nonce (12 bytes) - stored unencrypted                      │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Encrypted Header (Chunk 0)                                      │
//! │ [ciphertext: version(1) + media_type(1) + chunk_size(4)][tag]   │
//! ├─────────────────────────────────────────────────────────────────┤
//! │ Encrypted Data Chunks (1 to N)                                  │
//! │ [ciphertext (≤64KB)][tag (16B)] per chunk                       │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! - **Nonce derivation**: `chunk_nonce = base_nonce XOR chunk_index`
//! - **Chunk size**: 64KB (65536 bytes) plaintext per chunk
//! - **Memory usage**: ~128KB during streaming (one chunk buffer)

#![allow(clippy::missing_errors_doc)] // Error conditions documented in prose
#![allow(clippy::must_use_candidate)] // Crypto functions - callers understand return semantics

use super::media_type::MediaType;
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use std::io::{Read, Write};

/// Default chunk size for plaintext data (64KB).
pub const CHUNK_SIZE: usize = 65536;

/// Blob format version.
pub const VERSION: u8 = 1;

/// Nonce size in bytes.
pub const NONCE_SIZE: usize = 12;

/// Authentication tag size in bytes.
pub const TAG_SIZE: usize = 16;

/// Header size in bytes (version + `media_type` + `chunk_size`).
pub const HEADER_SIZE: usize = 6;

/// Errors that can occur during chunked crypto operations.
#[derive(Debug)]
pub enum CryptoError {
    /// Failed to encrypt data.
    EncryptionFailed(String),
    /// Failed to decrypt data.
    DecryptionFailed(String),
    /// Invalid blob format.
    InvalidFormat(String),
    /// IO error during read/write.
    IoError(std::io::Error),
    /// Unsupported blob version.
    UnsupportedVersion(u8),
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EncryptionFailed(msg) => write!(f, "Encryption failed: {msg}"),
            Self::DecryptionFailed(msg) => write!(f, "Decryption failed: {msg}"),
            Self::InvalidFormat(msg) => write!(f, "Invalid blob format: {msg}"),
            Self::IoError(err) => write!(f, "IO error: {err}"),
            Self::UnsupportedVersion(v) => write!(f, "Unsupported blob version: {v}"),
        }
    }
}

impl std::error::Error for CryptoError {}

impl From<std::io::Error> for CryptoError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

/// Blob header containing metadata.
#[derive(Debug, Clone)]
pub struct BlobHeader {
    /// Blob format version.
    pub version: u8,
    /// Media type of the content.
    pub media_type: MediaType,
    /// Chunk size used for encryption.
    pub chunk_size: u32,
}

impl BlobHeader {
    /// Create a new header with default settings.
    #[allow(clippy::cast_possible_truncation)] // CHUNK_SIZE is always < u32::MAX
    pub const fn new(media_type: MediaType) -> Self {
        Self {
            version: VERSION,
            media_type,
            chunk_size: CHUNK_SIZE as u32,
        }
    }

    /// Serialize header to bytes.
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0] = self.version;
        bytes[1] = self.media_type.to_byte();
        bytes[2..6].copy_from_slice(&self.chunk_size.to_le_bytes());
        bytes
    }

    /// Deserialize header from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() < HEADER_SIZE {
            return Err(CryptoError::InvalidFormat(format!(
                "Header too short: {} bytes, expected {}",
                bytes.len(),
                HEADER_SIZE
            )));
        }

        let version = bytes[0];
        if version != VERSION {
            return Err(CryptoError::UnsupportedVersion(version));
        }

        let media_type = MediaType::from_byte(bytes[1]);
        let chunk_size = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);

        Ok(Self {
            version,
            media_type,
            chunk_size,
        })
    }
}

/// Derive chunk nonce from base nonce and chunk index.
///
/// Uses XOR to combine the base nonce with the chunk index,
/// ensuring each chunk has a unique nonce.
fn derive_chunk_nonce(base: &[u8; NONCE_SIZE], index: u64) -> [u8; NONCE_SIZE] {
    let mut nonce = *base;
    let index_bytes = index.to_le_bytes();
    for i in 0..8 {
        nonce[i] ^= index_bytes[i];
    }
    nonce
}

/// Encrypt data from a reader and write to output.
///
/// ## Format
///
/// Output: `[base_nonce (12B)][encrypted_header + tag][chunk1 + tag][chunk2 + tag]...`
///
/// ## Arguments
///
/// * `input` - Reader providing plaintext data
/// * `output` - Writer for encrypted output
/// * `key` - 32-byte encryption key
/// * `media_type` - Content type for the blob header
pub fn encrypt_chunked<R: Read, W: Write>(
    input: &mut R,
    output: &mut W,
    key: &[u8; 32],
    media_type: MediaType,
) -> Result<(), CryptoError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    // Generate random base nonce
    let base_nonce: [u8; NONCE_SIZE] = rand::random();

    // Write base nonce (unencrypted)
    output.write_all(&base_nonce)?;

    // Encrypt and write header (chunk 0)
    let header = BlobHeader::new(media_type);
    let header_bytes = header.to_bytes();
    let header_nonce = derive_chunk_nonce(&base_nonce, 0);
    let encrypted_header = cipher
        .encrypt(Nonce::from_slice(&header_nonce), header_bytes.as_ref())
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;
    output.write_all(&encrypted_header)?;

    // Encrypt data chunks
    let mut chunk_index: u64 = 1;
    let mut buffer = vec![0u8; CHUNK_SIZE];

    loop {
        let bytes_read = read_exact_or_eof(input, &mut buffer)?;
        if bytes_read == 0 {
            break;
        }

        let chunk_nonce = derive_chunk_nonce(&base_nonce, chunk_index);
        let encrypted_chunk = cipher
            .encrypt(Nonce::from_slice(&chunk_nonce), &buffer[..bytes_read])
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        output.write_all(&encrypted_chunk)?;
        chunk_index += 1;
    }

    output.flush()?;
    Ok(())
}

/// Encrypt bytes in memory.
///
/// Convenience wrapper around `encrypt_chunked` for in-memory data.
pub fn encrypt_bytes(
    data: &[u8],
    key: &[u8; 32],
    media_type: MediaType,
) -> Result<Vec<u8>, CryptoError> {
    let mut input = std::io::Cursor::new(data);
    let mut output = Vec::new();
    encrypt_chunked(&mut input, &mut output, key, media_type)?;
    Ok(output)
}

/// Read and decrypt blob header from encrypted data.
///
/// Returns the header and base nonce for subsequent chunk decryption.
pub fn read_header<R: Read>(
    input: &mut R,
    key: &[u8; 32],
) -> Result<(BlobHeader, [u8; NONCE_SIZE]), CryptoError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    // Read base nonce
    let mut base_nonce = [0u8; NONCE_SIZE];
    input.read_exact(&mut base_nonce)?;

    // Read encrypted header (HEADER_SIZE + TAG_SIZE bytes)
    let mut encrypted_header = vec![0u8; HEADER_SIZE + TAG_SIZE];
    input.read_exact(&mut encrypted_header)?;

    // Decrypt header
    let header_nonce = derive_chunk_nonce(&base_nonce, 0);
    let header_bytes = cipher
        .decrypt(Nonce::from_slice(&header_nonce), encrypted_header.as_ref())
        .map_err(|e| CryptoError::DecryptionFailed(format!("Header decryption failed: {e}")))?;

    let header = BlobHeader::from_bytes(&header_bytes)?;
    Ok((header, base_nonce))
}

/// Decrypt next data chunk from stream.
///
/// Returns `None` when EOF is reached.
///
/// ## Arguments
///
/// * `input` - Reader positioned after the header
/// * `key` - 32-byte decryption key
/// * `chunk_index` - Current chunk index (starts at 1 for first data chunk)
/// * `base_nonce` - Base nonce from header
/// * `chunk_size` - Chunk size from header
pub fn decrypt_next_chunk<R: Read>(
    input: &mut R,
    key: &[u8; 32],
    chunk_index: u64,
    base_nonce: &[u8; NONCE_SIZE],
    chunk_size: u32,
) -> Result<Option<Vec<u8>>, CryptoError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    // Read encrypted chunk (up to chunk_size + TAG_SIZE bytes)
    let max_encrypted_size = chunk_size as usize + TAG_SIZE;
    let mut buffer = vec![0u8; max_encrypted_size];

    let bytes_read = read_exact_or_eof(input, &mut buffer)?;
    if bytes_read == 0 {
        return Ok(None);
    }

    // Decrypt chunk
    let chunk_nonce = derive_chunk_nonce(base_nonce, chunk_index);
    let decrypted = cipher
        .decrypt(Nonce::from_slice(&chunk_nonce), &buffer[..bytes_read])
        .map_err(|e| {
            CryptoError::DecryptionFailed(format!("Chunk {chunk_index} decryption failed: {e}"))
        })?;

    Ok(Some(decrypted))
}

/// Iterator for streaming decryption.
///
/// Yields decrypted data chunks, skipping the header.
pub struct ChunkedDecryptor<R: Read> {
    reader: R,
    cipher: ChaCha20Poly1305,
    base_nonce: [u8; NONCE_SIZE],
    chunk_size: u32,
    chunk_index: u64,
    finished: bool,
}

impl<R: Read> ChunkedDecryptor<R> {
    /// Create a new decryptor from a reader.
    ///
    /// Reads and decrypts the header, then yields data chunks.
    pub fn new(mut reader: R, key: &[u8; 32]) -> Result<(Self, BlobHeader), CryptoError> {
        let (header, base_nonce) = read_header(&mut reader, key)?;
        let cipher = ChaCha20Poly1305::new_from_slice(key)
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

        let decryptor = Self {
            reader,
            cipher,
            base_nonce,
            chunk_size: header.chunk_size,
            chunk_index: 1,
            finished: false,
        };

        Ok((decryptor, header))
    }

    /// Get the chunk size from the header.
    pub const fn chunk_size(&self) -> u32 {
        self.chunk_size
    }
}

impl<R: Read> Iterator for ChunkedDecryptor<R> {
    type Item = Result<Vec<u8>, CryptoError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }

        // Read encrypted chunk
        let max_encrypted_size = self.chunk_size as usize + TAG_SIZE;
        let mut buffer = vec![0u8; max_encrypted_size];

        let bytes_read = match read_exact_or_eof(&mut self.reader, &mut buffer) {
            Ok(n) => n,
            Err(e) => return Some(Err(e)),
        };

        if bytes_read == 0 {
            self.finished = true;
            return None;
        }

        // Decrypt chunk
        let chunk_nonce = derive_chunk_nonce(&self.base_nonce, self.chunk_index);
        let result = self
            .cipher
            .decrypt(Nonce::from_slice(&chunk_nonce), &buffer[..bytes_read])
            .map_err(|e| {
                CryptoError::DecryptionFailed(format!(
                    "Chunk {} decryption failed: {}",
                    self.chunk_index, e
                ))
            });

        self.chunk_index += 1;
        Some(result)
    }
}

/// Decrypt entire blob to bytes.
///
/// Convenience function for small blobs that fit in memory.
pub fn decrypt_bytes(
    encrypted: &[u8],
    key: &[u8; 32],
) -> Result<(Vec<u8>, BlobHeader), CryptoError> {
    let mut input = std::io::Cursor::new(encrypted);
    let (decryptor, header) = ChunkedDecryptor::new(&mut input, key)?;

    let mut output = Vec::new();
    for chunk_result in decryptor {
        output.extend(chunk_result?);
    }

    Ok((output, header))
}

/// Read exact bytes or return count if EOF reached.
fn read_exact_or_eof<R: Read>(reader: &mut R, buffer: &mut [u8]) -> Result<usize, CryptoError> {
    let mut total_read = 0;
    while total_read < buffer.len() {
        match reader.read(&mut buffer[total_read..]) {
            Ok(0) => break, // EOF
            Ok(n) => total_read += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e) => return Err(CryptoError::IoError(e)),
        }
    }
    Ok(total_read)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let header = BlobHeader::new(MediaType::Png);
        let bytes = header.to_bytes();
        let restored = BlobHeader::from_bytes(&bytes).unwrap();

        assert_eq!(restored.version, VERSION);
        assert_eq!(restored.media_type, MediaType::Png);
        assert_eq!(restored.chunk_size, CHUNK_SIZE as u32);
    }

    #[test]
    fn test_nonce_derivation() {
        let base = [0u8; 12];
        let nonce_0 = derive_chunk_nonce(&base, 0);
        let nonce_1 = derive_chunk_nonce(&base, 1);
        let nonce_2 = derive_chunk_nonce(&base, 2);

        // Each nonce should be different
        assert_ne!(nonce_0, nonce_1);
        assert_ne!(nonce_1, nonce_2);
        assert_ne!(nonce_0, nonce_2);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key: [u8; 32] = rand::random();
        let plaintext = b"Hello, World! This is a test message for encryption.";

        // Encrypt
        let encrypted = encrypt_bytes(plaintext, &key, MediaType::Unknown).unwrap();

        // Decrypt
        let (decrypted, header) = decrypt_bytes(&encrypted, &key).unwrap();

        assert_eq!(decrypted, plaintext);
        assert_eq!(header.media_type, MediaType::Unknown);
        assert_eq!(header.version, VERSION);
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let key: [u8; 32] = rand::random();
        // Create data larger than one chunk
        let plaintext: Vec<u8> = (0..CHUNK_SIZE * 3 + 1234)
            .map(|i| (i % 256) as u8)
            .collect();

        let encrypted = encrypt_bytes(&plaintext, &key, MediaType::Mp4).unwrap();
        let (decrypted, header) = decrypt_bytes(&encrypted, &key).unwrap();

        assert_eq!(decrypted, plaintext);
        assert_eq!(header.media_type, MediaType::Mp4);
    }

    #[test]
    fn test_chunked_decryptor_iterator() {
        let key: [u8; 32] = rand::random();
        let plaintext: Vec<u8> = (0..CHUNK_SIZE * 2 + 500).map(|i| (i % 256) as u8).collect();

        let encrypted = encrypt_bytes(&plaintext, &key, MediaType::Jpeg).unwrap();
        let mut cursor = std::io::Cursor::new(&encrypted);

        let (decryptor, header) = ChunkedDecryptor::new(&mut cursor, &key).unwrap();
        assert_eq!(header.media_type, MediaType::Jpeg);

        let mut decrypted = Vec::new();
        for chunk in decryptor {
            decrypted.extend(chunk.unwrap());
        }

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1: [u8; 32] = rand::random();
        let key2: [u8; 32] = rand::random();
        let plaintext = b"Secret data";

        let encrypted = encrypt_bytes(plaintext, &key1, MediaType::Unknown).unwrap();

        // Decryption with wrong key should fail
        let result = decrypt_bytes(&encrypted, &key2);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_data() {
        let key: [u8; 32] = rand::random();
        let plaintext = b"";

        let encrypted = encrypt_bytes(plaintext, &key, MediaType::Unknown).unwrap();
        let (decrypted, _) = decrypt_bytes(&encrypted, &key).unwrap();

        assert_eq!(decrypted, plaintext);
    }
}
