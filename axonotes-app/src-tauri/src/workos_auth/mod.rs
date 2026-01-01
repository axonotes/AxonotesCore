use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use once_cell::sync::Lazy;
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tiny_http::{Response, Server};
use tokio::sync::Mutex;

use crate::config::OAuthConfig;

const SUCCESS_HTML: &str = include_str!("html/success.html");
const ERROR_HTML: &str = include_str!("html/error.html");
const STYLES_CSS: &str = include_str!("html/styles.css");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub email: String,
    pub name: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    user: UserInfo,
}

#[derive(Deserialize)]
struct UserInfo {
    id: String,
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
}

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
pub async fn start_auth_flow<F>(callback: F) -> Result<String, String>
where
    F: FnOnce(Result<Profile, String>) + Send + 'static,
{
    let config = OAuthConfig::new();
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Store the verifier
    {
        let mut state = AUTH_STATE.lock().await;
        state.code_verifier = Some(code_verifier);
    }

    // Build authorization URL
    let auth_url = config.get_auth_url(&code_challenge);

    // Start callback server in background
    let callback_port = config.callback_port;
    tokio::spawn(async move {
        if let Err(e) = run_callback_server(callback_port, callback).await {
            eprintln!("Callback server error: {}", e);
        }
    });

    Ok(auth_url)
}

/// Refresh an access token using a refresh token
pub async fn refresh_access_token(refresh_token: &str) -> Result<String, String> {
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
    }

    let refresh_response: RefreshResponse = response.json().await.map_err(|e| e.to_string())?;
    Ok(refresh_response.access_token)
}

async fn run_callback_server<F>(port: u16, callback: F) -> Result<(), String>
where
    F: FnOnce(Result<Profile, String>) + Send + 'static,
{
    let address = format!("127.0.0.1:{}", port);
    let server = Server::http(&address).map_err(|e| e.to_string())?;

    for request in server.incoming_requests() {
        let url = format!("http://localhost:{}{}", port, request.url());

        // Handle CSS file request
        if url.contains("/styles.css") {
            let response = Response::from_string(STYLES_CSS).with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/css; charset=utf-8"[..])
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
                let response = Response::from_string(SUCCESS_HTML).with_header(
                    tiny_http::Header::from_bytes(
                        &b"Content-Type"[..],
                        &b"text/html; charset=utf-8"[..],
                    )
                        .unwrap(),
                );
                let _ = request.respond(response);

                // Exchange code for token in background
                let code = code.clone();
                tokio::spawn(async move {
                    // Small delay to ensure the HTML page fully loads
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                    match exchange_code_for_token(code).await {
                        Ok(profile) => callback(Ok(profile)),
                        Err(error) => callback(Err(error)),
                    }
                });

                break;
            } else if let Some(error) = params.get("error") {
                let error_description = params
                    .get("error_description")
                    .map(|s| s.as_str())
                    .unwrap_or(error);

                let error_html = ERROR_HTML.replace("{{error}}", error_description);

                let response = Response::from_string(error_html).with_header(
                    tiny_http::Header::from_bytes(
                        &b"Content-Type"[..],
                        &b"text/html; charset=utf-8"[..],
                    )
                        .unwrap(),
                );
                let _ = request.respond(response);

                // Call callback with error
                let error_msg = error_description.to_string();
                tokio::spawn(async move {
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                    callback(Err(error_msg));
                });

                break;
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
        return Err(format!("Token exchange failed: {}", error_text));
    }

    let token_response: TokenResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse token response: {}", e))?;

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