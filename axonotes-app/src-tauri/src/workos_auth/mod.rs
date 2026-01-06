//! # WorkOS Authentication Module
//!
//! Implements OAuth 2.0 + PKCE authentication flow using WorkOS as the identity provider.
//!
//! ## Flow Overview
//!
//! 1. **Generate PKCE pair**: Code verifier (random) + code challenge (SHA-256 hash)
//! 2. **Open browser**: Redirect to WorkOS authorization URL with challenge
//! 3. **Local callback**: Tiny HTTP server listens for OAuth redirect
//! 4. **Token exchange**: Exchange authorization code + verifier for tokens
//! 5. **Profile creation**: Extract user info and store profile locally
//!
//! ## Security Features
//!
//! - **PKCE**: Proof Key for Code Exchange prevents authorization code interception
//! - **Localhost callback**: No secrets transmitted over network
//! - **Short-lived tokens**: Access tokens expire; refresh tokens for renewal
//!
//! ## Usage
//!
//! ```ignore
//! // Start auth flow (returns URL to open in browser)
//! let auth_url = start_auth_flow(dark_mode, |result| {
//!     match result {
//!         Ok(profile) => save_profile(profile),
//!         Err(e) => show_error(e),
//!     }
//! }).await?;
//!
//! // Open auth_url in user's browser
//! open::that(auth_url)?;
//!
//! // Refresh expired tokens
//! let new_token = refresh_access_token(&refresh_token).await?;
//! ```

use crate::config::OAuthConfig;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use once_cell::sync::Lazy;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tiny_http::{Response, Server};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

const SUCCESS_HTML: &str = include_str!("html/success.html");
const ERROR_HTML: &str = include_str!("html/error.html");
const STYLES_CSS: &str = include_str!("html/styles.css");

/// User profile information returned from WorkOS authentication.
///
/// Contains identity information and tokens for API access.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Unique user identifier from WorkOS
    pub id: String,
    /// User's email address
    pub email: String,
    /// User's display name (first + last name)
    pub name: String,
    /// JWT access token for SpacetimeDB authentication
    pub access_token: String,
    /// Optional refresh token for obtaining new access tokens
    pub refresh_token: Option<String>,
}

/// Response from WorkOS token exchange endpoint.
#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    user: UserInfo,
}

/// User information included in token response.
#[derive(Deserialize)]
struct UserInfo {
    id: String,
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
}

/// Global state for tracking PKCE code verifier during auth flow.
struct AuthState {
    code_verifier: Option<String>,
}

static AUTH_STATE: Lazy<Arc<Mutex<AuthState>>> = Lazy::new(|| {
    Arc::new(Mutex::new(AuthState {
        code_verifier: None,
    }))
});

/// Generate PKCE code verifier (base64url encoded random string)
fn generate_code_verifier() -> String {
    let random_bytes: Vec<u8> = (0..32).map(|_| rand::rng().random::<u8>()).collect();
    URL_SAFE_NO_PAD.encode(random_bytes)
}

/// Generate PKCE code challenge (SHA256 hash of verifier)
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

/// Start OAuth flow with a callback that will be invoked when authentication completes.
/// Returns the authorization URL that should be opened in a browser.
///
/// The callback will be called with Ok(Profile) on success or Err(String) on failure.
/// The callback server will automatically shut down after handling the OAuth response.
pub async fn start_auth_flow<F>(dark_mode: bool, callback: F) -> Result<String, String>
where
    F: FnOnce(Result<Profile, String>) + Send + 'static,
{
    let config = OAuthConfig::new();
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(code_verifier.as_str());

    // Store the verifier
    {
        let mut state = AUTH_STATE.lock().await;
        state.code_verifier = Some(code_verifier);
    }

    // Build authorization URL
    let auth_url = config.get_auth_url(code_challenge.as_str());
    let callback_port = config.callback_port;

    // Use spawn_blocking for the synchronous server
    tokio::task::spawn_blocking(move || {
        // Run the blocking server in a dedicated thread
        tokio::runtime::Handle::current().block_on(async move {
            if let Err(e) = run_callback_server(dark_mode, callback_port, callback).await {
                eprintln!("Callback server error: {e}");
            }
        });
    });

    Ok(auth_url)
}

/// Response from token refresh endpoint
#[derive(Debug, Clone)]
pub struct RefreshResult {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

/// Refresh an access token using a refresh token.
/// Returns both the new access token and optionally a new refresh token
/// (if the provider rotates refresh tokens).
pub async fn refresh_access_token(refresh_token: &str) -> Result<RefreshResult, String> {
    let config = OAuthConfig::new();
    let client = reqwest::Client::new();
    let params = [
        ("client_id", config.client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
    ];

    let response = client
        .post(config.token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err("Token refresh failed".to_string());
    }

    #[derive(Deserialize)]
    struct RefreshResponse {
        access_token: String,
        refresh_token: Option<String>,
    }

    let refresh_response: RefreshResponse = response.json().await.map_err(|e| e.to_string())?;
    Ok(RefreshResult {
        access_token: refresh_response.access_token,
        refresh_token: refresh_response.refresh_token,
    })
}

async fn run_callback_server<F>(dark_mode: bool, port: u16, callback: F) -> Result<(), String>
where
    F: FnOnce(Result<Profile, String>) + Send + 'static,
{
    let address = format!("127.0.0.1:{port}");
    let server = Server::http(&address).map_err(|e| e.to_string())?;

    // Prepare dark mode class
    let dark_class = if dark_mode { "dark" } else { "" };

    let callback = Arc::new(Mutex::new(Some(callback)));

    // Spawn server handling in a separate task
    for request in server.incoming_requests() {
        let url = format!("http://localhost:{}{}", port, request.url());

        // Handle shutdown
        if url.contains("/shutdown") {
            // Just respond and break
            let _ = request.respond(Response::from_string(""));
            break;
        }

        // Handle CSS file request
        if url.contains("/styles.css") {
            let response = Response::from_string(STYLES_CSS).with_header(
                tiny_http::Header::from_bytes(
                    &b"Content-Type"[..],
                    &b"text/css; charset=utf-8"[..],
                )
                .unwrap(),
            );
            let _ = request.respond(response);
            continue;
        }

        if url.contains("/callback") {
            // Parse query parameters
            let parsed_url = url::Url::parse(&url).map_err(|e| e.to_string())?;
            let params: std::collections::HashMap<_, _> =
                parsed_url.query_pairs().into_owned().collect();

            if let Some(code) = params.get("code") {
                // Send success response to browser
                let success_html = SUCCESS_HTML.replace("{{dark_class}}", dark_class);
                let response = Response::from_string(success_html).with_header(
                    tiny_http::Header::from_bytes(
                        &b"Content-Type"[..],
                        &b"text/html; charset=utf-8"[..],
                    )
                    .unwrap(),
                );
                let _ = request.respond(response);

                // Exchange code for token
                let callback_fn = callback.lock().await.take();
                if let Some(cb) = callback_fn {
                    let code = code.clone();
                    match exchange_code_for_token(code).await {
                        Ok(profile) => cb(Ok(profile)),
                        Err(error) => cb(Err(error)),
                    }
                }

                // Shutdown server after 3 seconds
                let shutdown_port = port;
                tokio::spawn(async move {
                    sleep(Duration::from_secs(3)).await;
                    // Make a dummy request to unblock the server
                    let _ =
                        reqwest::get(format!("http://127.0.0.1:{shutdown_port}/shutdown")).await;
                });
            } else if let Some(error) = params.get("error") {
                #[allow(clippy::map_unwrap_or)] // Type coercion: &String -> &str via unwrap_or
                let error_description = params
                    .get("error_description")
                    .map(|s| s.as_str())
                    .unwrap_or(error);

                let error_html = ERROR_HTML
                    .replace("{{dark_class}}", dark_class)
                    .as_str()
                    .replace("{{error}}", error_description);

                let response = Response::from_string(error_html).with_header(
                    tiny_http::Header::from_bytes(
                        &b"Content-Type"[..],
                        &b"text/html; charset=utf-8"[..],
                    )
                    .unwrap(),
                );
                let _ = request.respond(response);

                // Call callback with error
                let callback_fn = callback.lock().await.take();
                if let Some(cb) = callback_fn {
                    cb(Err(error_description.to_string()));
                }

                // Shutdown server after 3 seconds
                let shutdown_port = port;
                tokio::spawn(async move {
                    sleep(Duration::from_secs(3)).await;
                    // Make a dummy request to unblock the server
                    let _ =
                        reqwest::get(format!("http://127.0.0.1:{shutdown_port}/shutdown")).await;
                });
            }
        }
    }

    Ok(())
}

async fn exchange_code_for_token(code: String) -> Result<Profile, String> {
    let config = OAuthConfig::new();

    // Get and clear the code verifier
    let code_verifier = {
        let mut state = AUTH_STATE.lock().await;
        state.code_verifier.take()
    }
    .ok_or("Code verifier not found")?;

    let client = reqwest::Client::new();
    let params = [
        ("client_id", config.client_id),
        ("code", code.as_str()),
        ("code_verifier", code_verifier.as_str()),
        ("grant_type", "authorization_code"),
    ];

    let response = client
        .post(config.token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("Token exchange failed: {error_text}"));
    }

    let token_response: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {e}"))?;

    let full_name = format!(
        "{} {}",
        token_response.user.first_name.unwrap_or_default(),
        token_response.user.last_name.unwrap_or_default()
    )
    .trim()
    .to_string();

    Ok(Profile {
        id: token_response.user.id,
        email: token_response.user.email,
        name: if full_name.is_empty() {
            "User".to_string()
        } else {
            full_name
        },
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
    })
}

// ============================================================================
// Token Refresh Manager
// ============================================================================

/// How often to check if token needs refresh (in seconds)
const REFRESH_CHECK_INTERVAL_SECS: u64 = 60;

/// Refresh token this many seconds before it expires
const REFRESH_BUFFER_SECS: i64 = 300; // 5 minutes

/// Handle to the background token refresh task
static TOKEN_REFRESH_HANDLE: Lazy<Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

/// JWT claims structure for decoding expiration time
#[derive(Debug, Deserialize)]
struct JwtClaims {
    exp: i64,
}

/// Decode a JWT token and extract the expiration timestamp.
/// Returns None if the token is invalid or doesn't contain an exp claim.
fn decode_token_expiry(token: &str) -> Option<i64> {
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

    // Use insecure decoding - we only need the exp claim, not validation
    let mut validation = Validation::new(Algorithm::RS256);
    validation.insecure_disable_signature_validation();
    validation.validate_exp = false;
    validation.validate_aud = false;

    // Try to decode with a dummy key (signature validation is disabled)
    let dummy_key = DecodingKey::from_secret(&[]);

    match decode::<JwtClaims>(token, &dummy_key, &validation) {
        Ok(token_data) => Some(token_data.claims.exp),
        Err(_) => None,
    }
}

/// Check if a token should be refreshed (expires within `REFRESH_BUFFER_SECS`)
fn should_refresh_token(token: &str) -> bool {
    let Some(exp) = decode_token_expiry(token) else {
        return false;
    };

    let now = chrono::Utc::now().timestamp();
    let time_until_expiry = exp - now;

    time_until_expiry <= REFRESH_BUFFER_SECS
}

/// Start the background token refresh task.
/// This task periodically checks if the active profile's token needs refreshing
/// and automatically refreshes it before expiration.
pub async fn start_token_refresh_task() {
    // Cancel any existing task first
    stop_token_refresh_task().await;

    let handle = tokio::spawn(async move {
        eprintln!("[token-refresh] Background task started");

        loop {
            sleep(Duration::from_secs(REFRESH_CHECK_INTERVAL_SECS)).await;

            if let Err(e) = check_and_refresh_token().await {
                eprintln!("[token-refresh] Check failed: {e}");
            }
        }
    });

    // Store the handle
    let mut guard = TOKEN_REFRESH_HANDLE.lock().await;
    *guard = Some(handle);
}

/// Stop the background token refresh task if it's running
pub async fn stop_token_refresh_task() {
    let mut guard = TOKEN_REFRESH_HANDLE.lock().await;
    if let Some(handle) = guard.take() {
        handle.abort();
        eprintln!("[token-refresh] Background task stopped");
    }
}

/// Check if the active profile's token needs refreshing and refresh if needed
async fn check_and_refresh_token() -> Result<(), String> {
    // Get the active profile
    let Some(profile) = crate::database::get_active_profile().await? else {
        return Ok(()); // No active profile, nothing to do
    };

    // Check if the token needs refreshing
    if !should_refresh_token(&profile.access_token) {
        return Ok(());
    }

    eprintln!("[token-refresh] Access token expiring soon, refreshing...");

    // Get refresh token
    let refresh_token = profile
        .refresh_token
        .as_ref()
        .ok_or("No refresh token available")?;

    // Refresh the token
    let result = refresh_access_token(refresh_token).await?;

    // Update the database with both tokens
    crate::database::update_profile_tokens(
        &profile.id,
        &result.access_token,
        result.refresh_token.as_deref(),
    )
    .await?;

    eprintln!("[token-refresh] Access token refreshed, reconnecting SpacetimeDB...");

    // Reconnect SpacetimeDB with the new token
    crate::stdb::reconnect_active_profile().await?;

    eprintln!("[token-refresh] SpacetimeDB reconnected with new token");

    Ok(())
}
