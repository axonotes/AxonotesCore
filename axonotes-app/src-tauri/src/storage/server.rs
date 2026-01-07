//! HTTP server for streaming decrypted blobs to the frontend.
//!
//! Binds to `127.0.0.1` on a random port with token-based authentication
//! to prevent request spoofing from other local processes.

use super::cache;
use super::chunked_crypto::ChunkedDecryptor;
use crate::database;
use crate::encryption::key_data;
use crate::storage_bindings::StorageClient;
use std::io::Read;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tiny_http::{Header, Method, Response, Server, StatusCode};

/// Start the blob HTTP server.
///
/// Returns `(port, token, shutdown_sender)`.
pub fn start_blob_server(
    client: Arc<StorageClient>,
) -> Result<(u16, String, tokio::sync::oneshot::Sender<()>), String> {
    // Generate random token (32 bytes = 64 hex chars)
    let token_bytes: [u8; 32] = rand::random();
    let token = hex::encode(token_bytes);

    // Bind to localhost on random port
    let server = Server::http("127.0.0.1:0").map_err(|e| format!("Failed to start server: {e}"))?;

    let port = server
        .server_addr()
        .to_ip()
        .ok_or("Failed to get server address")?
        .port();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Clone values for the server thread
    let token_clone = token.clone();

    // Spawn server thread
    std::thread::spawn(move || {
        run_server(server, token_clone, client, shutdown_rx);
    });

    Ok((port, token, shutdown_tx))
}

/// Run the HTTP server (blocking, runs in dedicated thread).
#[allow(clippy::needless_pass_by_value)] // Server requires ownership, token is moved to thread
fn run_server(
    server: Server,
    token: String,
    _client: Arc<StorageClient>,
    mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
) {
    let token_bytes = token.as_bytes();

    loop {
        // Check for shutdown signal (non-blocking)
        if shutdown_rx.try_recv().is_ok() {
            break;
        }

        // Accept request with timeout
        let Ok(Some(request)) = server.recv_timeout(std::time::Duration::from_millis(100)) else {
            continue; // Timeout or error, check shutdown again
        };

        // Handle request
        if let Err(e) = handle_request(request, token_bytes) {
            eprintln!("Blob server error: {e}");
        }
    }
}

/// Handle a single HTTP request.
fn handle_request(request: tiny_http::Request, token: &[u8]) -> Result<(), String> {
    let url = request.url().to_string();
    let method = request.method().clone();

    // Only allow GET requests
    if method != Method::Get {
        let response = Response::from_string("Method not allowed")
            .with_status_code(StatusCode(405))
            .with_header(Header::from_bytes("Allow", "GET").unwrap());
        request
            .respond(response)
            .map_err(|e| format!("Response error: {e}"))?;
        return Ok(());
    }

    // Parse URL: /blob/{hash}?token={token}
    let parts: Vec<&str> = url.splitn(2, '?').collect();
    let path = parts[0];
    let query = parts.get(1).unwrap_or(&"");

    // Check path format
    if !path.starts_with("/blob/") {
        let response = Response::from_string("Not found").with_status_code(StatusCode(404));
        request
            .respond(response)
            .map_err(|e| format!("Response error: {e}"))?;
        return Ok(());
    }

    let hash = &path[6..]; // Skip "/blob/"

    // Validate hash format (64 hex chars for blake3)
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        let response = Response::from_string("Invalid hash").with_status_code(StatusCode(400));
        request
            .respond(response)
            .map_err(|e| format!("Response error: {e}"))?;
        return Ok(());
    }

    // Extract and validate token
    let provided_token = extract_token(query);
    if !validate_token(provided_token.as_bytes(), token) {
        let response = Response::from_string("Unauthorized").with_status_code(StatusCode(401));
        request
            .respond(response)
            .map_err(|e| format!("Response error: {e}"))?;
        return Ok(());
    }

    // Serve the blob
    serve_blob(request, hash)
}

/// Extract token from query string.
fn extract_token(query: &str) -> String {
    for param in query.split('&') {
        if let Some(value) = param.strip_prefix("token=") {
            return value.to_string();
        }
    }
    String::new()
}

/// Validate token using constant-time comparison.
fn validate_token(provided: &[u8], expected: &[u8]) -> bool {
    if provided.len() != expected.len() {
        return false;
    }
    provided.ct_eq(expected).into()
}

/// Serve a blob with streaming decryption.
fn serve_blob(request: tiny_http::Request, hash: &str) -> Result<(), String> {
    // Check if blob is cached
    let Some(file) = cache::open_cached_blob(hash)? else {
        let response = Response::from_string("Blob not cached - call prefetch_blob first")
            .with_status_code(StatusCode(404));
        request
            .respond(response)
            .map_err(|e| format!("Response error: {e}"))?;
        return Ok(());
    };

    // Get document ID for this blob (need it to get encryption key)
    let Some(doc_id) = get_blob_doc_id_sync(hash)? else {
        let response =
            Response::from_string("Blob metadata not found").with_status_code(StatusCode(404));
        request
            .respond(response)
            .map_err(|e| format!("Response error: {e}"))?;
        return Ok(());
    };

    // Get document encryption key
    let Some(key) = get_document_encryption_key_sync(&doc_id)? else {
        let response =
            Response::from_string("Document key not found").with_status_code(StatusCode(500));
        request
            .respond(response)
            .map_err(|e| format!("Response error: {e}"))?;
        return Ok(());
    };

    // Create decryptor and read header
    let (decryptor, header) = match ChunkedDecryptor::new(file, &key) {
        Ok(d) => d,
        Err(e) => {
            let response = Response::from_string(format!("Decryption error: {e}"))
                .with_status_code(StatusCode(500));
            request
                .respond(response)
                .map_err(|e| format!("Response error: {e}"))?;
            return Ok(());
        }
    };

    // Stream decrypted content
    let mime_type = header.media_type.mime_type();
    let reader = DecryptorReader::new(decryptor);

    // Create response with streaming body
    let response = Response::new(
        StatusCode(200),
        vec![
            Header::from_bytes("Content-Type", mime_type).unwrap(),
            Header::from_bytes("Cache-Control", "private, max-age=3600").unwrap(),
            Header::from_bytes("X-Content-Type-Options", "nosniff").unwrap(),
        ],
        reader,
        None, // Unknown content length (streaming)
        None,
    );

    request
        .respond(response)
        .map_err(|e| format!("Response error: {e}"))?;

    Ok(())
}

/// Get blob `doc_id` synchronously (for use in server thread).
fn get_blob_doc_id_sync(hash: &str) -> Result<Option<String>, String> {
    // Use tokio runtime to run async code
    let hash = hash.to_string();

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        // We're in an async context, use block_in_place
        tokio::task::block_in_place(|| {
            handle.block_on(async { database::get_blob_doc_id(hash).await })
        })
    } else {
        // Not in async context, create a new runtime
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;
        rt.block_on(async { database::get_blob_doc_id(hash).await })
    }
}

/// Get document encryption key synchronously.
fn get_document_encryption_key_sync(doc_id: &str) -> Result<Option<[u8; 32]>, String> {
    let doc_id = doc_id.to_string();

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| {
            handle.block_on(async { key_data::get_document_encryption_key(&doc_id).await })
        })
    } else {
        let rt =
            tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;
        rt.block_on(async { key_data::get_document_encryption_key(&doc_id).await })
    }
}

/// Reader wrapper for `ChunkedDecryptor` to implement `std::io::Read`.
struct DecryptorReader<R: Read> {
    decryptor: ChunkedDecryptor<R>,
    buffer: Vec<u8>,
    position: usize,
    finished: bool,
}

impl<R: Read> DecryptorReader<R> {
    const fn new(decryptor: ChunkedDecryptor<R>) -> Self {
        Self {
            decryptor,
            buffer: Vec::new(),
            position: 0,
            finished: false,
        }
    }
}

impl<R: Read> Read for DecryptorReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // If we have buffered data, return it first
        if self.position < self.buffer.len() {
            let remaining = &self.buffer[self.position..];
            let to_copy = remaining.len().min(buf.len());
            buf[..to_copy].copy_from_slice(&remaining[..to_copy]);
            self.position += to_copy;
            return Ok(to_copy);
        }

        // Buffer exhausted, get next chunk
        if self.finished {
            return Ok(0);
        }

        match self.decryptor.next() {
            Some(Ok(chunk)) => {
                self.buffer = chunk;
                self.position = 0;

                let to_copy = self.buffer.len().min(buf.len());
                buf[..to_copy].copy_from_slice(&self.buffer[..to_copy]);
                self.position = to_copy;
                Ok(to_copy)
            }
            Some(Err(e)) => Err(std::io::Error::other(format!("Decryption error: {e}"))),
            None => {
                self.finished = true;
                Ok(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_token() {
        assert_eq!(extract_token("token=abc123"), "abc123");
        assert_eq!(extract_token("foo=bar&token=xyz"), "xyz");
        assert_eq!(extract_token("other=value"), "");
        assert_eq!(extract_token(""), "");
    }

    #[test]
    fn test_validate_token() {
        let token = b"secret_token_123";
        assert!(validate_token(b"secret_token_123", token));
        assert!(!validate_token(b"wrong_token", token));
        assert!(!validate_token(b"secret_token_12", token));
    }
}
