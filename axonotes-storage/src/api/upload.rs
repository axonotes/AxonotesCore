use crate::auth::AuthUser;
use crate::db::{Database, models::UploadResponse, queries};
use crate::quota::QuotaEvaluator;
use crate::storage::{StorageClient, streaming};
use crate::utils::{AppError, Result};
use axum::{
    Json,
    extract::{Query, Request, State},
    http::StatusCode,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tracing::{info, warn};

#[derive(Deserialize)]
pub struct UploadMetadata {
    pub document_id: String,
    pub signature: String, // Hex-encoded Ed25519 signature (128 chars)
    pub timestamp: i64,
}

pub struct AppState {
    pub db: Database,
    pub storage: StorageClient,
    pub quota_evaluator: Arc<RwLock<QuotaEvaluator>>,
    pub signature_max_age_secs: u64,
    pub max_blob_size_bytes: u64,
}

/// POST /blobs - Upload encrypted blob with streaming
pub async fn upload_blob(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Query(metadata): Query<UploadMetadata>,
    request: Request,
) -> Result<(StatusCode, Json<UploadResponse>)> {
    // Validate signature timestamp is not too old (prevents replay attacks)
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::Internal(format!("System time error: {}", e)))?
        .as_secs() as i64;

    let timestamp_age = now - metadata.timestamp;
    if timestamp_age < 0 {
        warn!(
            "Upload rejected: signature timestamp is in the future by {} seconds",
            -timestamp_age
        );
        return Err(AppError::BadRequest(
            "Signature timestamp is in the future".to_string(),
        ));
    }
    if timestamp_age > state.signature_max_age_secs as i64 {
        warn!(
            "Upload rejected: signature timestamp is {} seconds old (max: {})",
            timestamp_age, state.signature_max_age_secs
        );
        return Err(AppError::BadRequest(format!(
            "Signature timestamp is too old ({} seconds, max: {})",
            timestamp_age, state.signature_max_age_secs
        )));
    }

    // Get document and verify ownership
    let document = {
        let conn = state.db.conn();
        queries::get_document(&conn, &metadata.document_id)?.ok_or_else(|| {
            AppError::NotFound(format!("Document not found: {}", metadata.document_id))
        })?
    };

    // Get quota for the owner
    let (quota_bytes, _matched_rule) = {
        let evaluator = state.quota_evaluator.read().await;
        evaluator.evaluate(&user.claims)?
    };

    // Get current usage
    let owner = {
        let conn = state.db.conn();
        queries::get_or_create_user(&conn, &document.owner_id)?
    };

    // Calculate available quota, also respect max blob size limit
    let available = quota_bytes
        .saturating_sub(owner.used_bytes as u64)
        .min(state.max_blob_size_bytes);

    // Stream upload and compute hash
    let body_stream = request.into_body().into_data_stream();

    let upload_result = streaming::stream_upload(
        state.storage.client(),
        state.storage.bucket(),
        body_stream,
        available,
    )
    .await?;

    let hash = upload_result.hash;
    let size = upload_result.size;
    let temp_key = upload_result.temp_key;

    // Verify signature: sign(hash || document_id || timestamp)
    let message = format!("{}{}{}", hash, metadata.document_id, metadata.timestamp);
    if let Err(e) = verify_upload_signature(
        &document.public_key,
        &metadata.signature,
        message.as_bytes(),
    ) {
        // Cleanup temp upload on signature verification failure
        let _ = streaming::cleanup_temp_upload(
            state.storage.client(),
            state.storage.bucket(),
            &temp_key,
        )
        .await;
        return Err(e);
    }

    // Finalize upload (rename from temp to final hash)
    streaming::finalize_upload(
        state.storage.client(),
        state.storage.bucket(),
        &temp_key,
        &hash,
    )
    .await?;

    // Update database
    {
        let conn = state.db.conn();

        // Create blob ownership record
        queries::create_blob_ownership(
            &conn,
            &hash,
            &document.owner_id,
            &metadata.document_id,
            size as i64,
        )?;

        // Update owner's quota
        queries::update_user_quota(&conn, &document.owner_id, size as i64)?;
    }

    info!(
        "Blob uploaded: hash={} size={} document={} owner={}",
        hash,
        size,
        metadata.document_id,
        hex::encode(&document.owner_id)
    );

    Ok((
        StatusCode::CREATED,
        Json(UploadResponse {
            hash,
            size,
            uploaded_at: chrono::Utc::now().timestamp(),
        }),
    ))
}

fn verify_upload_signature(public_key: &[u8], signature_hex: &str, message: &[u8]) -> Result<()> {
    if public_key.len() != 32 {
        return Err(AppError::InvalidSignature);
    }

    let public_key_array: [u8; 32] = public_key
        .try_into()
        .map_err(|_| AppError::InvalidSignature)?;

    let verifying_key =
        VerifyingKey::from_bytes(&public_key_array).map_err(|_| AppError::InvalidSignature)?;

    // Decode hex signature
    let signature_bytes = hex::decode(signature_hex)
        .map_err(|_| AppError::BadRequest("Invalid signature hex".to_string()))?;

    if signature_bytes.len() != 64 {
        return Err(AppError::BadRequest(
            "Signature must be 64 bytes".to_string(),
        ));
    }

    let signature_array: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| AppError::InvalidSignature)?;

    let signature = Signature::from_bytes(&signature_array);

    verifying_key
        .verify(message, &signature)
        .map_err(|_| AppError::InvalidSignature)?;

    Ok(())
}
