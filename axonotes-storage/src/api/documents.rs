use crate::auth::AuthUser;
use crate::db::{
    Database,
    models::{CreateDocumentRequest, UpdatePublicKeyRequest, UpdatePublicKeyResponse},
    queries,
};
use crate::storage::{StorageClient, streaming};
use crate::utils::{AppError, Result, hash::validate_hash};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use std::sync::Arc;
use tracing::{info, warn};

pub struct DocumentState {
    pub db: Database,
    pub storage: StorageClient,
}

/// POST /documents - Create new document
pub async fn create_document(
    State(state): State<Arc<DocumentState>>,
    user: AuthUser,
    Json(payload): Json<CreateDocumentRequest>,
) -> Result<StatusCode> {
    // Decode public key from hex
    let public_key_bytes = hex::decode(&payload.public_key)
        .map_err(|_| AppError::BadRequest("Invalid public key hex".to_string()))?;

    if public_key_bytes.len() != 32 {
        return Err(AppError::BadRequest(
            "Public key must be 32 bytes".to_string(),
        ));
    }

    // Create document
    {
        let conn = state.db.conn();

        // Ensure user exists
        queries::get_or_create_user(&conn, &user.user_id_bytes)?;

        // Create document
        queries::create_document(
            &conn,
            &payload.document_id,
            &user.user_id_bytes,
            &public_key_bytes,
        )?;
    }

    info!(
        "Document created: id={} owner={}",
        payload.document_id, user.user_id
    );

    Ok(StatusCode::CREATED)
}

/// DELETE /documents/:document_id - Delete document and all associated blobs (owner only)
pub async fn delete_document(
    State(state): State<Arc<DocumentState>>,
    user: AuthUser,
    Path(document_id): Path<String>,
) -> Result<StatusCode> {
    // Get document and verify ownership
    let document = {
        let conn = state.db.conn();
        let doc = queries::get_document(&conn, &document_id)?
            .ok_or_else(|| AppError::NotFound(format!("Document not found: {}", document_id)))?;

        // Verify ownership (only owner can delete the entire document)
        if doc.owner_id != user.user_id_bytes {
            return Err(AppError::Forbidden(
                "Only document owner can delete".to_string(),
            ));
        }

        doc
    };

    // Get all blobs for this document
    let blobs = {
        let conn = state.db.conn();
        queries::get_blobs_for_document(&conn, &document_id)?
    };

    // Delete blobs from S3
    for blob in &blobs {
        if let Err(e) =
            streaming::delete_blob(state.storage.client(), state.storage.bucket(), &blob.hash).await
        {
            warn!("Failed to delete blob from S3: {} - {}", blob.hash, e);
            // Continue deleting other blobs even if one fails
        }
    }

    // Delete from database (this also updates user quota via the returned total_size)
    let total_size = {
        let conn = state.db.conn();
        let size = queries::delete_document(&conn, &document_id)?;

        // Update user quota (subtract deleted size)
        queries::update_user_quota(&conn, &document.owner_id, -(size as i64))?;

        size
    };

    info!(
        "Document deleted: id={} blobs_count={} total_size={}",
        document_id,
        blobs.len(),
        total_size
    );

    Ok(StatusCode::NO_CONTENT)
}

/// PUT /documents/:document_id/public-key - Update document's public key (anyone with old private key)
pub async fn update_public_key(
    State(state): State<Arc<DocumentState>>,
    Path(document_id): Path<String>,
    Json(payload): Json<UpdatePublicKeyRequest>,
) -> Result<Json<UpdatePublicKeyResponse>> {
    // No ownership check - signature verification is the authorization

    // Get document
    let document = {
        let conn = state.db.conn();
        queries::get_document(&conn, &document_id)?
            .ok_or_else(|| AppError::NotFound(format!("Document not found: {}", document_id)))?
    };

    // Decode new public key
    let new_public_key_bytes = hex::decode(&payload.new_public_key)
        .map_err(|_| AppError::BadRequest("Invalid new public key hex".to_string()))?;

    if new_public_key_bytes.len() != 32 {
        return Err(AppError::BadRequest(
            "Public key must be 32 bytes".to_string(),
        ));
    }

    // Verify signature: sign(new_public_key) with old private key
    // This proves the caller has access to the document's current private key
    verify_key_update_signature(
        &document.public_key,
        &payload.signature,
        &new_public_key_bytes,
    )?;

    // Update public key in database
    {
        let conn = state.db.conn();
        queries::update_document_public_key(&conn, &document_id, &new_public_key_bytes)?;
    }

    info!("Document public key updated: id={}", document_id);

    Ok(Json(UpdatePublicKeyResponse {
        document_id,
        public_key_updated: true,
    }))
}

/// DELETE /blobs/:hash - Delete individual blob (owner only)
pub async fn delete_blob(
    State(state): State<Arc<DocumentState>>,
    user: AuthUser,
    Path(hash): Path<String>,
) -> Result<StatusCode> {
    // Validate hash format
    validate_hash(&hash)?;

    // Get blob ownership to verify user is owner
    let blob = {
        let conn = state.db.conn();

        // Find blob ownership for this hash and user
        let mut stmt = conn
            .prepare("SELECT document_id, size_bytes FROM blob_ownership WHERE hash = ?1 AND user_id = ?2")
            .map_err(AppError::Database)?;

        stmt.query_row((&hash, &user.user_id_bytes), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|e| {
            if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
                AppError::Forbidden("Blob not found or you don't own it".to_string())
            } else {
                AppError::Database(e)
            }
        })?
    };

    let (document_id, size_bytes) = blob;

    // Delete from S3
    streaming::delete_blob(state.storage.client(), state.storage.bucket(), &hash).await?;

    // Delete from database and update quota
    {
        let conn = state.db.conn();

        // Delete blob ownership record
        conn.execute(
            "DELETE FROM blob_ownership WHERE hash = ?1 AND user_id = ?2",
            (&hash, &user.user_id_bytes),
        )
        .map_err(AppError::Database)?;

        // Update user quota (subtract deleted size)
        queries::update_user_quota(&conn, &user.user_id_bytes, -size_bytes)?;
    }

    info!(
        "Blob deleted: hash={} document={} size={} owner={}",
        hash, document_id, size_bytes, user.user_id
    );

    Ok(StatusCode::NO_CONTENT)
}

fn verify_key_update_signature(
    old_public_key: &[u8],
    signature_hex: &str,
    new_public_key: &[u8],
) -> Result<()> {
    if old_public_key.len() != 32 {
        return Err(AppError::InvalidSignature);
    }

    let old_key_array: [u8; 32] = old_public_key
        .try_into()
        .map_err(|_| AppError::InvalidSignature)?;

    let verifying_key =
        VerifyingKey::from_bytes(&old_key_array).map_err(|_| AppError::InvalidSignature)?;

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
        .verify(new_public_key, &signature)
        .map_err(|_| AppError::InvalidSignature)?;

    Ok(())
}
