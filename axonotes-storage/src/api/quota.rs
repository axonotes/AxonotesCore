use crate::auth::AuthUser;
use crate::db::{Database, models::QuotaResponse, queries};
use crate::quota::QuotaEvaluator;
use crate::utils::Result;
use axum::{Json, extract::State};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::debug;

pub struct QuotaState {
    pub db: Database,
    pub quota_evaluator: Arc<RwLock<QuotaEvaluator>>,
}

/// GET /quota - Get current quota information for authenticated user
pub async fn get_quota(
    State(state): State<Arc<QuotaState>>,
    user: AuthUser,
) -> Result<Json<QuotaResponse>> {
    debug!("Getting quota for user: {}", user.user_id);

    // Evaluate quota rules
    let (quota_bytes, matched_rule) = {
        let evaluator = state.quota_evaluator.read().await;
        evaluator.evaluate(&user.claims)?
    };

    // Get current usage
    let db_user = {
        let conn = state.db.conn();
        queries::get_or_create_user(&conn, &user.user_id_bytes)?
    };

    let used_bytes = db_user.used_bytes as u64;
    let available_bytes = quota_bytes.saturating_sub(used_bytes);

    // Extract plan from claims (optional)
    let plan = user
        .claims
        .get("plan")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(Json(QuotaResponse {
        user_id: user.user_id,
        plan,
        quota_bytes,
        used_bytes,
        available_bytes,
        matched_rule,
    }))
}
