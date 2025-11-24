use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State, Window};
use tauri_plugin_store::StoreExt;
use tiny_http::{Response, Server};

use crate::config::OAuthConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub email: String,
    pub name: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilesState {
    pub profiles: Vec<Profile>,
    pub active_profile_id: Option<String>,
}

#[derive(Default)]
pub struct OAuthState {
    code_verifier: Arc<Mutex<Option<String>>>,
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

// Generate PKCE code verifier (base64url encoded random string)
fn generate_code_verifier() -> String {
    let random_bytes: Vec<u8> = (0..32).map(|_| rand::thread_rng().gen::<u8>()).collect();
    URL_SAFE_NO_PAD.encode(random_bytes)
}

// Generate PKCE code challenge (SHA256 hash of verifier)
fn generate_code_challenge(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let hash = hasher.finalize();
    URL_SAFE_NO_PAD.encode(hash)
}

#[tauri::command]
pub async fn start_oauth_flow(
    window: Window,
    oauth_state: State<'_, OAuthState>,
) -> Result<String, String> {
    let config = OAuthConfig::new();
    let code_verifier = generate_code_verifier();
    let code_challenge = generate_code_challenge(&code_verifier);

    // Store the verifier for later use
    *oauth_state.code_verifier.lock().unwrap() = Some(code_verifier.clone());

    // Build authorization URL using config
    let auth_url = config.get_auth_url(&code_challenge);

    // Start localhost server to capture callback
    let callback_port = config.callback_port;
    tokio::spawn(async move {
        if let Err(e) = start_callback_server(window, callback_port).await {
            eprintln!("Callback server error: {}", e);
        }
    });

    Ok(auth_url)
}

async fn start_callback_server(window: Window, port: u16) -> Result<(), String> {
    let address = format!("127.0.0.1:{}", port);
    let server = Server::http(&address).map_err(|e| e.to_string())?;

    for request in server.incoming_requests() {
        let url = format!("http://localhost:{}{}", port, request.url());

        if url.contains("/callback") {
            // Parse query parameters
            let parsed_url = url::Url::parse(&url).map_err(|e| e.to_string())?;
            let params: std::collections::HashMap<_, _> =
                parsed_url.query_pairs().into_owned().collect();

            if let Some(code) = params.get("code") {
                // Send success response to browser
                let response = Response::from_string(
                    "<html><body><h1>Authentication successful!</h1><p>You can close this window.</p></body></html>"
                ).with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..]).unwrap()
                );
                let _ = request.respond(response);

                // Emit event with the authorization code
                window.emit("oauth_callback", code.to_string()).unwrap();
                break;
            } else if let Some(error) = params.get("error") {
                let response = Response::from_string(format!(
                    "<html><body><h1>Authentication failed</h1><p>{}</p></body></html>",
                    error
                ))
                .with_header(
                    tiny_http::Header::from_bytes(
                        &b"Content-Type"[..],
                        &b"text/html; charset=utf-8"[..],
                    )
                    .unwrap(),
                );
                let _ = request.respond(response);
                window.emit("oauth_error", error.to_string()).unwrap();
                break;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub async fn exchange_code_for_token(
    code: String,
    oauth_state: State<'_, OAuthState>,
) -> Result<Profile, String> {
    let config = OAuthConfig::new();
    let code_verifier = oauth_state
        .code_verifier
        .lock()
        .unwrap()
        .take()
        .ok_or("Code verifier not found")?;

    let client = reqwest::Client::new();
    let params = [
        ("client_id", config.client_id),
        ("code", &code),
        ("code_verifier", &code_verifier),
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

#[tauri::command]
pub async fn refresh_access_token(
    profile_id: String,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let config = OAuthConfig::new();
    let store = app.store("profiles.json").map_err(|e| e.to_string())?;

    let profiles_state: ProfilesState = store
        .get("profiles_state")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_else(|| ProfilesState {
            profiles: vec![],
            active_profile_id: None,
        });

    let profile = profiles_state
        .profiles
        .iter()
        .find(|p| p.id == profile_id)
        .ok_or("Profile not found")?;

    let refresh_token = profile
        .refresh_token
        .as_ref()
        .ok_or("No refresh token available")?;

    let client = reqwest::Client::new();
    let params = [
        ("client_id", config.client_id),
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token.as_str()),
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
