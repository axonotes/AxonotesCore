mod api;
mod auth;
mod config;
mod db;
mod middleware;
mod openapi;
mod quota;
mod storage;
mod utils;

use anyhow::Result;
use axum::{
    Router, middleware as axum_middleware,
    routing::{delete, get, post, put},
};
use config::{Config, hot_reload::ConfigWatcher};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.toml".to_string());
    let config = Config::from_file(&config_path)?;

    // Setup logging
    setup_logging(&config)?;

    info!(
        "Starting axonotes Storage Service v{}",
        env!("CARGO_PKG_VERSION")
    );
    info!("Loading configuration from: {}", config_path);

    // Initialize database
    let db = db::Database::new(&config.database.path)?;
    info!("Database initialized at: {}", config.database.path);

    // Initialize S3 storage
    let storage = storage::StorageClient::new(&config.storage).await;
    info!(
        "S3 client initialized: endpoint={}",
        config.storage.endpoint
    );

    // Initialize JWT authenticator
    let authenticator = Arc::new(auth::Authenticator::new(
        config.jwt.issuers.clone(),
        config.jwt.dev_mode,
    ));
    info!(
        "JWT authenticator initialized with {} allowed issuers (dev_mode: {})",
        config.jwt.issuers.len(),
        config.jwt.dev_mode
    );

    // Initialize quota evaluator
    let quota_evaluator = Arc::new(RwLock::new(quota::QuotaEvaluator::from_config(&config)?));
    info!(
        "Quota evaluator initialized with {} rules",
        config.quota_rules.rule.len()
    );

    // Initialize rate limiters from config
    let rate_limiters = Arc::new(middleware::RateLimiters::from_config(&config.rate_limits));
    info!(
        "Rate limiters initialized (upload: {}/min, download: {}/min, quota: {}/min)",
        config.rate_limits.upload_per_minute,
        config.rate_limits.download_per_minute,
        config.rate_limits.quota_per_minute
    );

    // Setup config hot-reload with quota evaluator refresh
    let config_watcher = ConfigWatcher::new(config.clone(), PathBuf::from(&config_path));
    let config_handle = config_watcher.config();

    // Spawn config watcher with quota evaluator refresh
    let quota_eval_clone = quota_evaluator.clone();
    tokio::spawn(async move {
        if let Err(e) = config_watcher.start().await {
            error!("Config watcher error: {}", e);
        }
    });

    // Watch for config changes and refresh quota evaluator
    let config_clone = config_handle.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        let mut last_config = config_clone.read().await.quota_rules.rule.len();

        loop {
            interval.tick().await;
            let current_config = config_clone.read().await;
            let current_rules = current_config.quota_rules.rule.len();

            // Check if config changed
            if current_rules != last_config {
                match quota::QuotaEvaluator::from_config(&current_config) {
                    Ok(new_evaluator) => {
                        let mut evaluator = quota_eval_clone.write().await;
                        *evaluator = new_evaluator;
                        info!("Quota evaluator refreshed after config change");
                        last_config = current_rules;
                    }
                    Err(e) => {
                        warn!("Failed to refresh quota evaluator: {}", e);
                    }
                }
            }
        }
    });

    info!("Config hot-reload watcher started");

    // Spawn periodic cleanup tasks
    spawn_cleanup_tasks(
        authenticator.clone(),
        rate_limiters.clone(),
        db.clone(),
        storage.clone(),
    );

    // Build application state
    let upload_state = Arc::new(api::AppState {
        db: db.clone(),
        storage: storage.clone(),
        quota_evaluator: quota_evaluator.clone(),
        signature_max_age_secs: config.jwt.signature_max_age_secs,
        max_blob_size_bytes: config.storage.max_blob_size_mb * 1024 * 1024,
    });

    let quota_state = Arc::new(api::QuotaState {
        db: db.clone(),
        quota_evaluator: quota_evaluator.clone(),
    });

    let document_state = Arc::new(api::DocumentState {
        db: db.clone(),
        storage: storage.clone(),
    });

    let download_storage = Arc::new(storage.clone());

    // Build router with auth-protected routes
    let protected_routes = Router::new()
        // Blob upload endpoint (protected + rate limited)
        .route("/blobs", post(api::upload_blob))
        .with_state(upload_state)
        .layer(axum_middleware::from_fn_with_state(
            rate_limiters.clone(),
            middleware::upload_rate_limit,
        ))
        // Blob delete endpoint (protected)
        .route("/blobs/{hash}", delete(api::delete_blob))
        .with_state(document_state.clone())
        // Quota endpoint (protected + rate limited)
        .route("/quota", get(api::get_quota))
        .with_state(quota_state)
        .layer(axum_middleware::from_fn_with_state(
            rate_limiters.clone(),
            middleware::quota_rate_limit,
        ))
        // Document endpoints (protected)
        .route("/documents", post(api::create_document))
        .with_state(document_state.clone())
        .route("/documents/{document_id}", delete(api::delete_document))
        .with_state(document_state.clone())
        .route(
            "/documents/{document_id}/public-key",
            put(api::update_public_key),
        )
        .with_state(document_state)
        // Add auth middleware to all protected routes
        .layer(axum_middleware::from_fn_with_state(
            authenticator.clone(),
            middleware::auth_middleware,
        ));

    // Public routes (no auth required, but rate limited if authenticated)
    let public_routes = Router::new()
        .route("/blobs/{hash}", get(api::download_blob))
        .with_state(download_storage)
        .layer(axum_middleware::from_fn_with_state(
            rate_limiters.clone(),
            middleware::download_rate_limit,
        ));

    // Combine all routes
    let app = Router::new()
        .merge(protected_routes)
        .merge(public_routes)
        // Swagger UI
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
        )
        // Global middleware
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = "0.0.0.0:8080";
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Setup logging with file rotation
fn setup_logging(config: &Config) -> Result<()> {
    // Create log directory
    if let Some(parent) = std::path::Path::new(&config.logging.file).parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Setup file appender with rotation
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .max_log_files(config.logging.max_age_days as usize)
        .filename_prefix("app")
        .filename_suffix("log")
        .build(
            std::path::Path::new(&config.logging.file)
                .parent()
                .unwrap_or_else(|| std::path::Path::new("./logs")),
        )?;

    // Build subscriber
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_writer(file_appender))
        .with(fmt::layer().with_writer(std::io::stdout))
        .init();

    Ok(())
}

/// Spawn background cleanup tasks
fn spawn_cleanup_tasks(
    authenticator: Arc<auth::Authenticator>,
    rate_limiters: Arc<middleware::RateLimiters>,
    db: db::Database,
    storage: storage::StorageClient,
) {
    // JWKS cache cleanup (every 10 minutes)
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(600));
        loop {
            interval.tick().await;
            authenticator.jwks_cache().cleanup_expired();
        }
    });

    // Rate limiter cleanup (every 5 minutes)
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            rate_limiters.cleanup_all();
        }
    });

    // S3 ghost file cleanup (every 6 hours)
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(6 * 3600));
        // Wait a bit before first run
        tokio::time::sleep(Duration::from_secs(60)).await;

        loop {
            interval.tick().await;
            info!("Starting S3 ghost file cleanup");

            if let Err(e) = cleanup_ghost_files(&db, &storage).await {
                error!("S3 ghost file cleanup failed: {}", e);
            } else {
                info!("S3 ghost file cleanup completed");
            }
        }
    });

    info!("Background cleanup tasks spawned");
}

/// Cleanup orphaned blobs from S3 that don't exist in database
async fn cleanup_ghost_files(db: &db::Database, storage: &storage::StorageClient) -> Result<()> {
    // List all objects in S3 bucket (excluding temp/)
    let mut continuation_token: Option<String> = None;
    let mut total_deleted = 0;

    loop {
        let mut list_request = storage
            .client()
            .list_objects_v2()
            .bucket(storage.bucket())
            .max_keys(1000);

        if let Some(token) = &continuation_token {
            list_request = list_request.continuation_token(token);
        }

        let response = list_request.send().await?;

        if let Some(ref contents) = response.contents {
            for object in contents {
                if let Some(key) = object.key() {
                    // Skip temp files
                    if key.starts_with("temp/") {
                        continue;
                    }

                    // Extract hash from path (format: ab/cd/abcd123...)
                    let parts: Vec<&str> = key.split('/').collect();
                    if parts.len() == 3 {
                        let hash = parts[2];

                        // Check if blob exists in database
                        let exists = {
                            let conn = db.conn();
                            db::queries::blob_exists(&conn, hash)?
                        };

                        if !exists {
                            // Delete orphaned blob
                            warn!("Deleting ghost file from S3: {}", key);
                            storage
                                .client()
                                .delete_object()
                                .bucket(storage.bucket())
                                .key(key)
                                .send()
                                .await?;
                            total_deleted += 1;
                        }
                    }
                }
            }
        }

        // Check if there are more objects
        if response.is_truncated() == Some(true) {
            continuation_token = response.next_continuation_token;
        } else {
            break;
        }
    }

    if total_deleted > 0 {
        info!("Deleted {} ghost files from S3", total_deleted);
    }

    Ok(())
}
