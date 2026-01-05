use crate::storage::{StorageClient, streaming};
use crate::utils::{AppError, Result, hash::validate_hash};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use tracing::debug;

/// GET /blobs/:hash - Download blob by hash
pub async fn download_blob(
    State(storage): State<Arc<StorageClient>>,
    Path(hash): Path<String>,
) -> Result<Response> {
    // Validate hash format
    validate_hash(&hash)?;

    debug!("Downloading blob: {}", hash);

    // Download from S3
    let byte_stream = streaming::download_blob(storage.client(), storage.bucket(), &hash).await?;

    // Collect bytes from ByteStream
    let bytes = byte_stream
        .collect()
        .await
        .map_err(|e| AppError::S3Error(format!("Failed to read S3 stream: {}", e)))?
        .into_bytes();

    // Build response with appropriate headers
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", hash)
            .parse()
            .unwrap(),
    );

    Ok((StatusCode::OK, headers, bytes).into_response())
}
