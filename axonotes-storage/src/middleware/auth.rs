use crate::auth::{AuthUser, Authenticator};
use crate::utils::AppError;
use axum::{
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, request::Parts},
    middleware::Next,
    response::Response,
};
use std::{future::Future, sync::Arc};

/// Extract JWT from Authorization header and verify it
pub async fn auth_middleware(
    State(auth): State<Arc<Authenticator>>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Extract token from Authorization header
    let token = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .ok_or_else(|| {
            AppError::Unauthorized("Missing or invalid Authorization header".to_string())
        })?;

    // Verify token
    let user = auth.verify_token(token).await?;

    // Store user in request extensions
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}

/// Extractor for authenticated user
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    #[allow(clippy::manual_async_fn)]
    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = std::result::Result<Self, Self::Rejection>> + Send {
        async move {
            parts
                .extensions
                .get::<AuthUser>()
                .cloned()
                .ok_or_else(|| AppError::Unauthorized("Not authenticated".to_string()))
        }
    }
}
