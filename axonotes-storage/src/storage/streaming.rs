use crate::utils::{AppError, Result, hash::new_hasher};
use aws_sdk_s3::{
    Client,
    primitives::ByteStream,
    types::{CompletedMultipartUpload, CompletedPart},
};
use bytes::Bytes;
use futures::Stream;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Minimum part size for S3 multipart upload (5 MB)
const MIN_PART_SIZE: usize = 5 * 1024 * 1024;

/// Stream upload result
pub struct StreamUploadResult {
    pub hash: String,
    pub size: u64,
    pub temp_key: String,
}

/// Upload stream to S3 using multipart upload while computing BLAKE3 hash
///
/// For files < 5MB: uses single PUT
/// For files >= 5MB: uses multipart upload with streaming parts
///
/// This prevents loading entire file into memory.
pub async fn stream_upload<S, E>(
    client: &Client,
    bucket: &str,
    mut stream: S,
    max_size: u64,
) -> Result<StreamUploadResult>
where
    S: Stream<Item = std::result::Result<Bytes, E>> + Unpin,
    E: std::error::Error + Send + Sync + 'static,
{
    use futures::StreamExt;

    let mut hasher = new_hasher();
    let mut bytes_received = 0u64;
    let mut buffer = Vec::with_capacity(MIN_PART_SIZE);

    // Generate temporary key for upload
    let temp_key = format!("temp/{}", Uuid::new_v4());

    // Multipart upload state (initialized on demand)
    let mut upload_id: Option<String> = None;
    let mut completed_parts: Vec<CompletedPart> = Vec::new();
    let mut part_number = 1i32;

    // Collect chunks while hashing, upload parts when buffer reaches threshold
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| AppError::Internal(e.to_string()))?;

        bytes_received += chunk.len() as u64;

        // Check quota during streaming
        if bytes_received > max_size {
            // Abort multipart upload if in progress
            if let Some(ref uid) = upload_id {
                let _ = abort_multipart_upload(client, bucket, &temp_key, uid).await;
            }
            return Err(AppError::PayloadTooLarge(format!(
                "Upload size {} exceeds quota {}",
                bytes_received, max_size
            )));
        }

        // Update hash incrementally
        hasher.update(&chunk);

        // Add to buffer
        buffer.extend_from_slice(&chunk);

        // If buffer >= 5MB, upload as a part
        if buffer.len() >= MIN_PART_SIZE {
            // Initialize multipart upload if not started
            if upload_id.is_none() {
                upload_id = Some(create_multipart_upload(client, bucket, &temp_key).await?);
                debug!("Started multipart upload: {:?}", upload_id);
            }

            // Upload part
            let part_data = std::mem::take(&mut buffer);
            buffer = Vec::with_capacity(MIN_PART_SIZE);

            let etag = upload_part(
                client,
                bucket,
                &temp_key,
                upload_id.as_ref().unwrap(),
                part_number,
                part_data,
            )
            .await
            .map_err(|e| {
                // Best effort abort on failure
                let uid = upload_id.clone();
                let c = client.clone();
                let b = bucket.to_string();
                let tk = temp_key.clone();
                tokio::spawn(async move {
                    if let Some(uid) = uid {
                        let _ = abort_multipart_upload(&c, &b, &tk, &uid).await;
                    }
                });
                e
            })?;

            completed_parts.push(
                CompletedPart::builder()
                    .e_tag(etag)
                    .part_number(part_number)
                    .build(),
            );
            part_number += 1;

            debug!(
                "Uploaded part {} ({} bytes so far)",
                part_number - 1,
                bytes_received
            );
        }
    }

    debug!("Upload stream complete: {} bytes received", bytes_received);

    // Finalize hash
    let hash = hasher.finalize().to_hex().to_string();
    debug!("Computed BLAKE3 hash: {}", hash);

    // Handle remaining buffer
    if let Some(ref uid) = upload_id {
        // Multipart upload in progress - upload final part and complete
        if !buffer.is_empty() {
            let etag = upload_part(client, bucket, &temp_key, uid, part_number, buffer)
                .await
                .map_err(|e| {
                    let uid = upload_id.clone();
                    let c = client.clone();
                    let b = bucket.to_string();
                    let tk = temp_key.clone();
                    tokio::spawn(async move {
                        if let Some(uid) = uid {
                            let _ = abort_multipart_upload(&c, &b, &tk, &uid).await;
                        }
                    });
                    e
                })?;

            completed_parts.push(
                CompletedPart::builder()
                    .e_tag(etag)
                    .part_number(part_number)
                    .build(),
            );
        }

        // Complete the multipart upload
        complete_multipart_upload(client, bucket, &temp_key, uid, completed_parts).await?;
        info!(
            "Completed multipart upload to S3: {} ({} parts)",
            temp_key, part_number
        );
    } else {
        // Small file - use single PUT (buffer contains entire file)
        let byte_stream = ByteStream::from(buffer);

        if let Err(e) = client
            .put_object()
            .bucket(bucket)
            .key(&temp_key)
            .body(byte_stream)
            .send()
            .await
        {
            return Err(AppError::S3Error(format!("Failed to upload to S3: {}", e)));
        }

        info!("Uploaded to S3 with single PUT: {}", temp_key);
    }

    Ok(StreamUploadResult {
        hash,
        size: bytes_received,
        temp_key,
    })
}

/// Create a multipart upload and return the upload ID
async fn create_multipart_upload(client: &Client, bucket: &str, key: &str) -> Result<String> {
    let response = client
        .create_multipart_upload()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .map_err(|e| AppError::S3Error(format!("Failed to create multipart upload: {}", e)))?;

    response
        .upload_id()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::S3Error("No upload ID returned".to_string()))
}

/// Upload a single part of a multipart upload, returns ETag
async fn upload_part(
    client: &Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
    part_number: i32,
    data: Vec<u8>,
) -> Result<String> {
    let response = client
        .upload_part()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .part_number(part_number)
        .body(ByteStream::from(data))
        .send()
        .await
        .map_err(|e| AppError::S3Error(format!("Failed to upload part {}: {}", part_number, e)))?;

    response
        .e_tag()
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::S3Error("No ETag returned for part".to_string()))
}

/// Complete a multipart upload
async fn complete_multipart_upload(
    client: &Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
    parts: Vec<CompletedPart>,
) -> Result<()> {
    let completed_upload = CompletedMultipartUpload::builder()
        .set_parts(Some(parts))
        .build();

    client
        .complete_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .multipart_upload(completed_upload)
        .send()
        .await
        .map_err(|e| AppError::S3Error(format!("Failed to complete multipart upload: {}", e)))?;

    Ok(())
}

/// Abort a multipart upload (cleanup on failure)
async fn abort_multipart_upload(
    client: &Client,
    bucket: &str,
    key: &str,
    upload_id: &str,
) -> Result<()> {
    warn!(
        "Aborting multipart upload: {} (upload_id: {})",
        key, upload_id
    );

    client
        .abort_multipart_upload()
        .bucket(bucket)
        .key(key)
        .upload_id(upload_id)
        .send()
        .await
        .map_err(|e| AppError::S3Error(format!("Failed to abort multipart upload: {}", e)))?;

    Ok(())
}

/// Rename S3 object from temp key to final hash-based key
pub async fn finalize_upload(
    client: &Client,
    bucket: &str,
    temp_key: &str,
    hash: &str,
) -> Result<()> {
    use crate::utils::hash::hash_to_path;

    let final_key = hash_to_path(hash);

    // Copy object to final location
    // S3 CopyObject requires format: /bucket-name/key
    // Note: path separators (/) should NOT be encoded, only special chars in key segments
    let copy_source = format!("/{}/{}", bucket, temp_key);
    client
        .copy_object()
        .bucket(bucket)
        .key(&final_key)
        .copy_source(&copy_source)
        .send()
        .await
        .map_err(|e| AppError::S3Error(format!("Failed to copy S3 object: {}", e)))?;

    // Delete temp object
    client
        .delete_object()
        .bucket(bucket)
        .key(temp_key)
        .send()
        .await
        .map_err(|e| AppError::S3Error(format!("Failed to delete temp S3 object: {}", e)))?;

    info!("Finalized upload: {} -> {}", temp_key, final_key);
    Ok(())
}

/// Delete temp object on failure
pub async fn cleanup_temp_upload(client: &Client, bucket: &str, temp_key: &str) -> Result<()> {
    client
        .delete_object()
        .bucket(bucket)
        .key(temp_key)
        .send()
        .await
        .map_err(|e| AppError::S3Error(format!("Failed to cleanup temp object: {}", e)))?;

    debug!("Cleaned up temp upload: {}", temp_key);
    Ok(())
}

/// Download blob from S3 as stream
pub async fn download_blob(client: &Client, bucket: &str, hash: &str) -> Result<ByteStream> {
    use crate::utils::hash::hash_to_path;
    use tracing::error;

    let key = hash_to_path(hash);

    let response = client
        .get_object()
        .bucket(bucket)
        .key(&key)
        .send()
        .await
        .map_err(|e| {
            let error_str = e.to_string();
            let error_debug = format!("{:?}", e);
            error!(
                "S3 get_object error: {} | debug: {}",
                error_str, error_debug
            );
            // Handle various S3/MinIO error formats for missing objects
            if error_str.contains("NoSuchKey")
                || error_str.contains("NotFound")
                || error_str.contains("not found")
                || error_str.contains("404")
                || error_debug.contains("NoSuchKey")
            {
                AppError::NotFound(format!("Blob not found: {}", hash))
            } else {
                AppError::S3Error(format!("Failed to download from S3: {}", e))
            }
        })?;

    Ok(response.body)
}

/// Delete blob from S3
pub async fn delete_blob(client: &Client, bucket: &str, hash: &str) -> Result<()> {
    use crate::utils::hash::hash_to_path;

    let key = hash_to_path(hash);

    client
        .delete_object()
        .bucket(bucket)
        .key(&key)
        .send()
        .await
        .map_err(|e| AppError::S3Error(format!("Failed to delete from S3: {}", e)))?;

    debug!("Deleted blob from S3: {}", hash);
    Ok(())
}

/// Check if blob exists in S3
#[allow(dead_code)]
pub async fn blob_exists_in_s3(client: &Client, bucket: &str, hash: &str) -> Result<bool> {
    use crate::utils::hash::hash_to_path;

    let key = hash_to_path(hash);

    match client.head_object().bucket(bucket).key(&key).send().await {
        Ok(_) => Ok(true),
        Err(e) => {
            if e.to_string().contains("NotFound") || e.to_string().contains("NoSuchKey") {
                Ok(false)
            } else {
                Err(AppError::S3Error(format!(
                    "Failed to check S3 object: {}",
                    e
                )))
            }
        }
    }
}
