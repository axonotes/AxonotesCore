use crate::db::models::{
    CreateDocumentRequest, QuotaResponse, UpdatePublicKeyRequest, UpdatePublicKeyResponse,
    UploadResponse,
};
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(),
    components(schemas(
        UploadResponse,
        QuotaResponse,
        CreateDocumentRequest,
        UpdatePublicKeyRequest,
        UpdatePublicKeyResponse,
    )),
    tags(
        (name = "blobs", description = "Blob storage operations"),
        (name = "quota", description = "Quota management"),
        (name = "documents", description = "Document management"),
    ),
    info(
        title = "Axonotes Storage Service",
        version = "0.1.0",
        description = "E2EE-compatible blob storage with JWT authentication, Ed25519 signatures, and flexible quota rules",
        contact(name = "Axonotes", url = "https://axonotes.com"),
    ),
    servers(
        (url = "http://localhost:8080", description = "Local development server"),
    )
)]
pub struct ApiDoc;
