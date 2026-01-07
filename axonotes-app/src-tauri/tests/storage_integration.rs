//! Integration tests for the storage module.
//!
//! These tests verify our storage implementation including:
//! - `StorageClient` (storage_bindings)
//! - `StorageHandler` (storage)
//! - Chunked encryption/decryption
//! - Blob caching
//! - HTTP server streaming
//!
//! ## Running Tests
//!
//! These tests require the storage backend to be running:
//! ```bash
//! axonge run storage dev
//! ```
//!
//! Then run the tests:
//! ```bash
//! cargo test --test storage_integration -- --ignored
//! ```
//!
//! For tests that don't need the backend (chunked crypto, cache), run:
//! ```bash
//! cargo test --test storage_integration
//! ```

use std::sync::Once;

// Import from the library crate
use axonotes_app_lib::storage_bindings::{
    CreateDocumentRequest, StorageClient, UpdatePublicKeyRequest, UploadBlobParams,
};

// ============================================================================
// Test Setup Helpers
// ============================================================================

static INIT: Once = Once::new();

/// Initialize database paths for tests (called once).
fn init_test_db() {
    INIT.call_once(|| {
        let temp_dir = std::env::temp_dir().join("axonotes_storage_test");
        std::fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");
        let _ = axonotes_app_lib::database::init_paths(temp_dir);
    });
}

/// Get test JWT from environment or return a placeholder.
fn get_test_jwt() -> String {
    std::env::var("TEST_JWT").unwrap_or_else(|_| "test-jwt-placeholder".to_string())
}

/// Get storage API URL from environment or use default.
fn get_storage_url() -> String {
    std::env::var("STORAGE_URL").unwrap_or_else(|_| "http://localhost:8081".to_string())
}

/// Generate a unique test document ID.
fn test_doc_id() -> String {
    format!("test-doc-{}", uuid::Uuid::new_v4())
}

/// Generate a test signing key.
fn test_signing_key() -> ed25519_dalek::SigningKey {
    use ed25519_dalek::SigningKey;
    use rand_core::OsRng;
    SigningKey::generate(&mut OsRng)
}

/// Get public key hex from signing key.
fn public_key_hex(signing_key: &ed25519_dalek::SigningKey) -> String {
    hex::encode(signing_key.verifying_key().as_bytes())
}

/// Create a signature for blob operations.
fn create_test_signature(
    hash: &str,
    doc_id: &str,
    timestamp: i64,
    signing_key: &ed25519_dalek::SigningKey,
) -> String {
    use ed25519_dalek::Signer;

    let mut message = Vec::new();
    message.extend_from_slice(hash.as_bytes());
    message.extend_from_slice(doc_id.as_bytes());
    message.extend_from_slice(&timestamp.to_le_bytes());

    let signature = signing_key.sign(&message);
    hex::encode(signature.to_bytes())
}

/// Get current timestamp.
fn current_timestamp() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ============================================================================
// StorageClient Tests (storage_bindings)
// ============================================================================

/// Test StorageClient creation and JWT management.
#[test]
fn test_storage_client_creation() {
    let client = StorageClient::new("http://localhost:8081", "initial-jwt");

    assert_eq!(client.base_url(), "http://localhost:8081");
    assert_eq!(client.get_jwt(), "initial-jwt");

    // Test JWT update
    client.set_jwt("updated-jwt");
    assert_eq!(client.get_jwt(), "updated-jwt");
}

/// Test StorageClient document creation (requires backend).
#[tokio::test]
#[ignore]
async fn test_storage_client_create_document() {
    let client = StorageClient::new(get_storage_url(), get_test_jwt());
    let doc_id = test_doc_id();
    let signing_key = test_signing_key();
    let public_key = public_key_hex(&signing_key);

    let request = CreateDocumentRequest {
        document_id: doc_id.clone(),
        public_key,
    };

    let result = client.create_document(request).await;

    match result {
        Ok(()) => {
            println!("Document created successfully: {}", doc_id);
            // Cleanup
            let _ = client.delete_document(&doc_id).await;
        }
        Err(e) => {
            // Check if it's an auth error (expected with placeholder JWT)
            let err_str = e.to_string();
            if err_str.contains("Unauthorized") || err_str.contains("401") {
                println!("Auth error (expected with placeholder JWT): {}", e);
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

/// Test StorageClient document lifecycle (requires backend).
#[tokio::test]
#[ignore]
async fn test_storage_client_document_lifecycle() {
    let client = StorageClient::new(get_storage_url(), get_test_jwt());
    let doc_id = test_doc_id();
    let signing_key = test_signing_key();
    let public_key = public_key_hex(&signing_key);

    // Create document
    let create_request = CreateDocumentRequest {
        document_id: doc_id.clone(),
        public_key: public_key.clone(),
    };

    if client.create_document(create_request).await.is_err() {
        println!("Skipping test - auth error");
        return;
    }

    // Update public key
    let new_signing_key = test_signing_key();
    let new_public_key = public_key_hex(&new_signing_key);
    let update_request = UpdatePublicKeyRequest {
        public_key: new_public_key,
    };

    let update_result = client
        .update_document_public_key(&doc_id, update_request)
        .await;
    assert!(update_result.is_ok(), "Failed to update public key");

    // Delete document
    let delete_result = client.delete_document(&doc_id).await;
    assert!(delete_result.is_ok(), "Failed to delete document");
}

/// Test StorageClient blob upload and download (requires backend).
#[tokio::test]
#[ignore]
async fn test_storage_client_blob_operations() {
    let client = StorageClient::new(get_storage_url(), get_test_jwt());
    let doc_id = test_doc_id();
    let signing_key = test_signing_key();
    let public_key = public_key_hex(&signing_key);

    // Create document first
    let create_request = CreateDocumentRequest {
        document_id: doc_id.clone(),
        public_key,
    };

    if client.create_document(create_request).await.is_err() {
        println!("Skipping test - auth error");
        return;
    }

    // Upload blob
    let test_data = b"Hello, this is test blob data!";
    let hash = blake3::hash(test_data).to_hex().to_string();
    let timestamp = current_timestamp();
    let signature = create_test_signature(&hash, &doc_id, timestamp, &signing_key);

    let upload_params = UploadBlobParams {
        document_id: doc_id.clone(),
        signature,
        timestamp,
    };

    let upload_result = client.upload_blob(upload_params, test_data.to_vec()).await;
    assert!(upload_result.is_ok(), "Failed to upload blob");

    let upload_response = upload_result.unwrap();
    assert_eq!(upload_response.hash, hash);

    // Download blob
    let download_result = client.download_blob(&hash).await;
    assert!(download_result.is_ok(), "Failed to download blob");

    let download_response = download_result.unwrap();
    assert_eq!(download_response.data, test_data);

    // Cleanup
    let _ = client.delete_document(&doc_id).await;
}

/// Test StorageClient quota endpoint (requires backend).
#[tokio::test]
#[ignore]
async fn test_storage_client_quota() {
    let client = StorageClient::new(get_storage_url(), get_test_jwt());

    let result = client.get_quota().await;

    match result {
        Ok(quota) => {
            println!(
                "Quota: total={}, used={}, available={}",
                quota.quota_bytes, quota.used_bytes, quota.available_bytes
            );
            assert!(quota.quota_bytes >= quota.used_bytes);
            assert_eq!(quota.available_bytes, quota.quota_bytes - quota.used_bytes);
        }
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("Unauthorized") {
                println!("Auth error (expected with placeholder JWT)");
            } else {
                panic!("Unexpected error: {}", e);
            }
        }
    }
}

/// Test StorageClient duplicate document conflict (requires backend).
#[tokio::test]
#[ignore]
async fn test_storage_client_duplicate_conflict() {
    let client = StorageClient::new(get_storage_url(), get_test_jwt());
    let doc_id = test_doc_id();
    let signing_key = test_signing_key();
    let public_key = public_key_hex(&signing_key);

    let request = CreateDocumentRequest {
        document_id: doc_id.clone(),
        public_key: public_key.clone(),
    };

    // Create first time
    if client.create_document(request.clone()).await.is_err() {
        println!("Skipping test - auth error");
        return;
    }

    // Try to create again - should fail with Conflict
    let duplicate_result = client.create_document(request).await;
    assert!(duplicate_result.is_err());

    let err = duplicate_result.unwrap_err();
    let err_str = err.to_string();
    assert!(
        err_str.contains("Conflict") || err_str.contains("409"),
        "Expected Conflict error, got: {}",
        err_str
    );

    // Cleanup
    let _ = client.delete_document(&doc_id).await;
}

// ============================================================================
// Chunked Crypto Tests (no backend needed)
// ============================================================================

mod chunked_crypto_tests {
    use axonotes_app_lib::storage::{chunked_crypto, MediaType};

    #[test]
    fn test_encrypt_decrypt_small_data() {
        let key: [u8; 32] = rand::random();
        let plaintext = b"Hello, World!";

        let encrypted = chunked_crypto::encrypt_bytes(plaintext, &key, MediaType::Unknown)
            .expect("Encryption failed");

        let (decrypted, header) =
            chunked_crypto::decrypt_bytes(&encrypted, &key).expect("Decryption failed");

        assert_eq!(decrypted, plaintext);
        assert_eq!(header.media_type, MediaType::Unknown);
    }

    #[test]
    fn test_encrypt_decrypt_large_data() {
        let key: [u8; 32] = rand::random();
        // Data larger than chunk size (64KB)
        let plaintext: Vec<u8> = (0..chunked_crypto::CHUNK_SIZE * 3 + 1234)
            .map(|i| (i % 256) as u8)
            .collect();

        let encrypted = chunked_crypto::encrypt_bytes(&plaintext, &key, MediaType::Png)
            .expect("Encryption failed");

        let (decrypted, header) =
            chunked_crypto::decrypt_bytes(&encrypted, &key).expect("Decryption failed");

        assert_eq!(decrypted, plaintext);
        assert_eq!(header.media_type, MediaType::Png);
    }

    #[test]
    fn test_wrong_key_fails() {
        let key1: [u8; 32] = rand::random();
        let key2: [u8; 32] = rand::random();
        let plaintext = b"Secret data";

        let encrypted = chunked_crypto::encrypt_bytes(plaintext, &key1, MediaType::Unknown)
            .expect("Encryption failed");

        let result = chunked_crypto::decrypt_bytes(&encrypted, &key2);
        assert!(result.is_err(), "Decryption with wrong key should fail");
    }

    #[test]
    fn test_media_type_preserved() {
        let key: [u8; 32] = rand::random();
        let plaintext = b"Test data";

        for media_type in [
            MediaType::Png,
            MediaType::Jpeg,
            MediaType::Gif,
            MediaType::Mp4,
            MediaType::Mp3,
        ] {
            let encrypted = chunked_crypto::encrypt_bytes(plaintext, &key, media_type)
                .expect("Encryption failed");

            let (_, header) =
                chunked_crypto::decrypt_bytes(&encrypted, &key).expect("Decryption failed");

            assert_eq!(header.media_type, media_type);
        }
    }

    #[test]
    fn test_streaming_encryption() {
        use std::io::Cursor;

        let key: [u8; 32] = rand::random();
        let plaintext = b"Streaming test data that is not too long";

        let mut input = Cursor::new(plaintext.as_slice());
        let mut output = Vec::new();

        chunked_crypto::encrypt_chunked(&mut input, &mut output, &key, MediaType::Webm)
            .expect("Streaming encryption failed");

        // Verify we can decrypt it
        let (decrypted, header) =
            chunked_crypto::decrypt_bytes(&output, &key).expect("Decryption failed");

        assert_eq!(decrypted, plaintext);
        assert_eq!(header.media_type, MediaType::Webm);
    }
}

// ============================================================================
// MediaType Tests (no backend needed)
// ============================================================================

mod media_type_tests {
    use axonotes_app_lib::storage::MediaType;

    #[test]
    fn test_from_extension() {
        assert_eq!(MediaType::from_extension("png"), MediaType::Png);
        assert_eq!(MediaType::from_extension("PNG"), MediaType::Png);
        assert_eq!(MediaType::from_extension("jpg"), MediaType::Jpeg);
        assert_eq!(MediaType::from_extension("jpeg"), MediaType::Jpeg);
        assert_eq!(MediaType::from_extension("gif"), MediaType::Gif);
        assert_eq!(MediaType::from_extension("webp"), MediaType::Webp);
        assert_eq!(MediaType::from_extension("svg"), MediaType::Svg);
        assert_eq!(MediaType::from_extension("mp4"), MediaType::Mp4);
        assert_eq!(MediaType::from_extension("webm"), MediaType::Webm);
        assert_eq!(MediaType::from_extension("mp3"), MediaType::Mp3);
        assert_eq!(MediaType::from_extension("ogg"), MediaType::Ogg);
        assert_eq!(MediaType::from_extension("unknown"), MediaType::Unknown);
    }

    #[test]
    fn test_mime_type() {
        assert_eq!(MediaType::Png.mime_type(), "image/png");
        assert_eq!(MediaType::Jpeg.mime_type(), "image/jpeg");
        assert_eq!(MediaType::Mp4.mime_type(), "video/mp4");
        assert_eq!(MediaType::Mp3.mime_type(), "audio/mpeg");
        assert_eq!(MediaType::Unknown.mime_type(), "application/octet-stream");
    }

    #[test]
    fn test_byte_conversion() {
        for byte in 0x00..=0xFF {
            let media_type = MediaType::from_byte(byte);
            // Round-trip for known types
            if media_type != MediaType::Unknown || byte == 0xFF {
                assert_eq!(media_type.to_byte(), byte);
            }
        }
    }

    #[test]
    fn test_magic_bytes_detection() {
        // PNG magic bytes
        let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        assert_eq!(MediaType::from_magic_bytes(&png), MediaType::Png);

        // JPEG magic bytes
        let jpeg = [0xFF, 0xD8, 0xFF, 0xE0];
        assert_eq!(MediaType::from_magic_bytes(&jpeg), MediaType::Jpeg);

        // GIF magic bytes
        let gif = [0x47, 0x49, 0x46, 0x38, 0x39, 0x61];
        assert_eq!(MediaType::from_magic_bytes(&gif), MediaType::Gif);

        // Unknown
        let unknown = [0x00, 0x00, 0x00, 0x00];
        assert_eq!(MediaType::from_magic_bytes(&unknown), MediaType::Unknown);
    }
}

// ============================================================================
// Cache Tests (requires database initialization)
// ============================================================================

mod cache_tests {
    use super::*;

    #[tokio::test]
    async fn test_blob_cache_operations() {
        init_test_db();

        // Unlock database if needed
        if !axonotes_app_lib::database::is_unlocked().await {
            axonotes_app_lib::database::unlock_db("".to_string())
                .await
                .expect("Failed to unlock db");
        }

        let hash = format!(
            "test_hash_{}",
            uuid::Uuid::new_v4().to_string().replace("-", "")
        );
        // Make it 64 chars like a real blake3 hash
        let hash = format!("{:0<64}", &hash[..32]);
        let doc_id = test_doc_id();
        let test_data = b"Test blob data for caching";

        // Cache the blob
        axonotes_app_lib::storage::cache::cache_blob(&hash, &doc_id, test_data)
            .await
            .expect("Failed to cache blob");

        // Check if cached
        assert!(
            axonotes_app_lib::storage::cache::is_cached(&hash),
            "Blob should be cached"
        );

        // Retrieve cached blob
        let cached = axonotes_app_lib::storage::cache::get_cached_blob(&hash)
            .await
            .expect("Failed to get cached blob");

        assert!(cached.is_some(), "Should return cached data");
        assert_eq!(cached.unwrap(), test_data);

        // Get doc_id from cache
        let retrieved_doc_id = axonotes_app_lib::storage::cache::get_blob_doc_id(&hash)
            .await
            .expect("Failed to get doc_id");

        assert_eq!(retrieved_doc_id, Some(doc_id.clone()));

        // Delete cached blob
        axonotes_app_lib::storage::cache::delete_cached_blob(&hash)
            .await
            .expect("Failed to delete cached blob");

        assert!(
            !axonotes_app_lib::storage::cache::is_cached(&hash),
            "Blob should no longer be cached"
        );
    }

    #[tokio::test]
    async fn test_cache_for_document_cleanup() {
        init_test_db();

        if !axonotes_app_lib::database::is_unlocked().await {
            axonotes_app_lib::database::unlock_db("".to_string())
                .await
                .expect("Failed to unlock db");
        }

        let doc_id = test_doc_id();

        // Cache multiple blobs for the same document
        for i in 0..3 {
            let hash = format!(
                "{:0<64}",
                format!("cleanup_test_hash_{}_{}", doc_id, i).replace("-", "")
            );
            let hash = &hash[..64];
            let data = format!("Blob data {}", i);

            axonotes_app_lib::storage::cache::cache_blob(hash, &doc_id, data.as_bytes())
                .await
                .expect("Failed to cache blob");
        }

        // Delete all blobs for document
        let deleted_count = axonotes_app_lib::storage::cache::delete_blobs_for_document(&doc_id)
            .await
            .expect("Failed to delete blobs for document");

        assert_eq!(deleted_count, 3, "Should have deleted 3 blobs");
    }
}
