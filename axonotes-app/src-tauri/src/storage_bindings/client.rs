//! HTTP client for the Storage API.

use reqwest::{Client, StatusCode};
use std::sync::RwLock;

use super::error::{StorageError, StorageResult};
use super::types::*;

/// Client for interacting with the Axonotes Storage API.
///
/// The client stores the base URL and JWT token internally.
/// Use [`StorageClient::set_jwt`] to update the token when it's refreshed.
pub struct StorageClient {
    /// HTTP client instance.
    client: Client,
    /// Base URL of the storage server (e.g., "http://localhost:8081").
    base_url: String,
    /// JWT token for authentication, wrapped in RwLock for thread-safe updates.
    jwt: RwLock<String>,
}

impl StorageClient {
    /// Create a new storage client.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL of the storage server (e.g., "http://localhost:8081")
    /// * `jwt` - JWT token for authentication
    pub fn new(base_url: impl Into<String>, jwt: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
            jwt: RwLock::new(jwt.into()),
        }
    }

    /// Create a new storage client with a custom reqwest client.
    ///
    /// This allows configuring timeouts, connection pools, etc.
    pub fn with_client(
        client: Client,
        base_url: impl Into<String>,
        jwt: impl Into<String>,
    ) -> Self {
        Self {
            client,
            base_url: base_url.into(),
            jwt: RwLock::new(jwt.into()),
        }
    }

    /// Update the JWT token.
    ///
    /// Call this when the token is refreshed to ensure subsequent requests
    /// use the new token.
    pub fn set_jwt(&self, jwt: impl Into<String>) {
        let mut token = self.jwt.write().unwrap();
        *token = jwt.into();
    }

    /// Get the current JWT token.
    pub fn get_jwt(&self) -> String {
        self.jwt.read().unwrap().clone()
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    // ========================================================================
    // Document Operations
    // ========================================================================

    /// Create a new document.
    ///
    /// # Arguments
    ///
    /// * `request` - Document creation request containing the document ID and public key
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success (201 Created).
    ///
    /// # Errors
    ///
    /// - [`StorageError::Unauthorized`] if the JWT is invalid or expired
    /// - [`StorageError::Conflict`] if a document with the same ID already exists
    pub async fn create_document(&self, request: CreateDocumentRequest) -> StorageResult<()> {
        let url = format!("{}/documents", self.base_url);
        let jwt = self.get_jwt();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", jwt))
            .json(&request)
            .send()
            .await?;

        match response.status() {
            StatusCode::CREATED => Ok(()),
            status => Err(self.handle_error_response(status, response).await),
        }
    }

    /// Update a document's public key.
    ///
    /// # Arguments
    ///
    /// * `document_id` - ID of the document to update
    /// * `request` - Request containing the new public key
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success (200 OK).
    pub async fn update_document_public_key(
        &self,
        document_id: &str,
        request: UpdatePublicKeyRequest,
    ) -> StorageResult<()> {
        let url = format!("{}/documents/{}/public-key", self.base_url, document_id);
        let jwt = self.get_jwt();

        let response = self
            .client
            .put(&url)
            .header("Authorization", format!("Bearer {}", jwt))
            .json(&request)
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => Ok(()),
            status => Err(self.handle_error_response(status, response).await),
        }
    }

    /// Delete a document.
    ///
    /// # Arguments
    ///
    /// * `document_id` - ID of the document to delete
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success (204 No Content).
    pub async fn delete_document(&self, document_id: &str) -> StorageResult<()> {
        let url = format!("{}/documents/{}", self.base_url, document_id);
        let jwt = self.get_jwt();

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", jwt))
            .send()
            .await?;

        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            status => Err(self.handle_error_response(status, response).await),
        }
    }

    // ========================================================================
    // Blob Operations
    // ========================================================================

    /// Upload a blob.
    ///
    /// # Arguments
    ///
    /// * `params` - Upload parameters containing document_id, signature, and timestamp
    /// * `data` - Raw blob data to upload
    ///
    /// # Returns
    ///
    /// Returns the Blake3 hash of the uploaded blob on success.
    ///
    /// # Signature
    ///
    /// The signature must be created by signing `hash || document_id || timestamp`
    /// where:
    /// - `hash` is the hex-encoded Blake3 hash of the blob data
    /// - `document_id` is the document ID string
    /// - `timestamp` is the Unix timestamp as a string
    ///
    /// # Errors
    ///
    /// - [`StorageError::Forbidden`] if the signature is invalid
    /// - [`StorageError::QuotaExceeded`] if the user's quota would be exceeded
    /// - [`StorageError::InvalidTimestamp`] if the timestamp is too old or in the future
    pub async fn upload_blob(
        &self,
        params: UploadBlobParams,
        data: Vec<u8>,
    ) -> StorageResult<UploadBlobResponse> {
        let url = format!(
            "{}/blobs?document_id={}&signature={}&timestamp={}",
            self.base_url,
            urlencoding::encode(&params.document_id),
            urlencoding::encode(&params.signature),
            params.timestamp
        );
        let jwt = self.get_jwt();

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", jwt))
            .body(data)
            .send()
            .await?;

        match response.status() {
            StatusCode::CREATED => {
                let body: UploadBlobResponse = response.json().await?;
                Ok(body)
            }
            status => Err(self.handle_error_response(status, response).await),
        }
    }

    /// Download a blob by its hash.
    ///
    /// # Arguments
    ///
    /// * `hash` - Blake3 hash of the blob (hex-encoded, 64 characters)
    ///
    /// # Returns
    ///
    /// Returns the blob data on success.
    ///
    /// # Note
    ///
    /// Downloads are public and do not require authentication.
    pub async fn download_blob(&self, hash: &str) -> StorageResult<DownloadBlobResponse> {
        // Validate hash format
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(StorageError::InvalidHash(format!(
                "Hash must be 64 hex characters, got: {}",
                hash
            )));
        }

        let url = format!("{}/blobs/{}", self.base_url, hash);

        let response = self.client.get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let content_length = response.content_length().unwrap_or(0);
                let data = response.bytes().await?.to_vec();
                Ok(DownloadBlobResponse {
                    data,
                    content_length,
                })
            }
            status => Err(self.handle_error_response(status, response).await),
        }
    }

    /// Delete a blob by its hash.
    ///
    /// # Arguments
    ///
    /// * `hash` - Blake3 hash of the blob (hex-encoded, 64 characters)
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success (204 No Content).
    pub async fn delete_blob(&self, hash: &str) -> StorageResult<()> {
        // Validate hash format
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(StorageError::InvalidHash(format!(
                "Hash must be 64 hex characters, got: {}",
                hash
            )));
        }

        let url = format!("{}/blobs/{}", self.base_url, hash);
        let jwt = self.get_jwt();

        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", jwt))
            .send()
            .await?;

        match response.status() {
            StatusCode::NO_CONTENT => Ok(()),
            status => Err(self.handle_error_response(status, response).await),
        }
    }

    // ========================================================================
    // Quota Operations
    // ========================================================================

    /// Get the current user's quota information.
    ///
    /// # Returns
    ///
    /// Returns quota information including total, used, and available bytes.
    pub async fn get_quota(&self) -> StorageResult<QuotaResponse> {
        let url = format!("{}/quota", self.base_url);
        let jwt = self.get_jwt();

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", jwt))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => {
                let body: QuotaResponse = response.json().await?;
                Ok(body)
            }
            status => Err(self.handle_error_response(status, response).await),
        }
    }

    // ========================================================================
    // Error Handling
    // ========================================================================

    /// Convert an HTTP error response to a StorageError.
    async fn handle_error_response(
        &self,
        status: StatusCode,
        response: reqwest::Response,
    ) -> StorageError {
        let body = response.text().await.unwrap_or_default();

        // Try to parse as JSON error response
        let message = if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&body) {
            error_response.get_message()
        } else {
            body
        };

        match status {
            StatusCode::UNAUTHORIZED => StorageError::Unauthorized(message),
            StatusCode::FORBIDDEN => StorageError::Forbidden(message),
            StatusCode::NOT_FOUND => StorageError::NotFound(message),
            StatusCode::CONFLICT => StorageError::Conflict(message),
            StatusCode::PAYLOAD_TOO_LARGE => StorageError::QuotaExceeded(message),
            StatusCode::UNPROCESSABLE_ENTITY => StorageError::ValidationError(message),
            StatusCode::TOO_MANY_REQUESTS => StorageError::RateLimited(message),
            StatusCode::BAD_REQUEST => {
                // Check if it's a timestamp error
                if message.to_lowercase().contains("timestamp") {
                    StorageError::InvalidTimestamp(message)
                } else if message.to_lowercase().contains("hash") {
                    StorageError::InvalidHash(message)
                } else {
                    StorageError::Api {
                        status: status.as_u16(),
                        message,
                    }
                }
            }
            _ => StorageError::Api {
                status: status.as_u16(),
                message,
            },
        }
    }
}
