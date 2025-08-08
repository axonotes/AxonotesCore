use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Serialize)]
struct DeviceAuthRequest {
    client_id: String,
    code_challenge: String,
    code_challenge_method: String,
    scope: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u32,
    pub interval: u32,
}

#[derive(Debug, Serialize)]
struct TokenExchangeRequest {
    grant_type: String,
    device_code: String,
    code_verifier: String,
    client_id: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u32,
    pub scope: String,
}

#[derive(Debug, Deserialize)]
struct PollResponse {
    status: Option<String>,
    message: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
    error_description: String,
}

pub struct OAuthClient {
    client: Client,
    base_url: String,
    client_id: String,
    code_verifier: String,
    code_challenge: String,
}

impl OAuthClient {
    pub fn new(base_url: String, client_id: String) -> Self {
        let code_verifier = generate_code_verifier();
        let code_challenge = generate_code_challenge(&code_verifier);

        Self {
            client: Client::new(),
            base_url,
            client_id,
            code_verifier,
            code_challenge,
        }
    }

    pub async fn start_device_flow(
        &self,
    ) -> Result<DeviceAuthResponse, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/oauth/device/authorize", self.base_url);

        let params = [
            ("client_id", self.client_id.as_str()),
            ("code_challenge", self.code_challenge.as_str()),
            ("code_challenge_method", "S256"),
            ("scope", "openid profile email"),
        ];

        let response = self.client.post(&url).form(&params).send().await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Device auth failed: {}", error_text).into());
        }

        let auth_response: DeviceAuthResponse = response.json().await?;
        Ok(auth_response)
    }

    pub async fn poll_for_authorization(
        &self,
        device_code: &str,
    ) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/oauth/device/token", self.base_url);

        let response = self
            .client
            .get(&url)
            .query(&[("device_code", device_code)])
            .send()
            .await?;

        if response.status().is_success() {
            let poll_response: PollResponse = response.json().await?;

            // Check if authorization is complete
            if let Some(status) = poll_response.status {
                return Ok(status == "complete");
            }

            Ok(false)
        } else if response.status() == 400 {
            // Parse error response
            let error_response: ErrorResponse = response.json().await?;

            match error_response.error.as_str() {
                "authorization_pending" => Ok(false),
                "slow_down" => {
                    // Server wants us to slow down polling
                    sleep(Duration::from_secs(2)).await;
                    Ok(false)
                }
                "expired_token" => Err("Device code expired".into()),
                _ => Err(format!(
                    "Polling error: {}",
                    error_response.error_description
                )
                .into()),
            }
        } else {
            Err(format!("HTTP error: {}", response.status()).into())
        }
    }

    pub async fn exchange_for_tokens(
        &self,
        device_code: &str,
    ) -> Result<TokenResponse, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/oauth/device/token", self.base_url);

        // Send as form data, not JSON
        let params = [
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
            ("code_verifier", self.code_verifier.as_str()),
            ("client_id", self.client_id.as_str()),
        ];

        let response = self.client.post(&url).form(&params).send().await?;

        if !response.status().is_success() {
            let error_response: ErrorResponse = response.json().await?;
            return Err(format!(
                "Token exchange failed: {}",
                error_response.error_description
            )
            .into());
        }

        let token_response: TokenResponse = response.json().await?;
        Ok(token_response)
    }
}

// PKCE helper functions
fn generate_code_verifier() -> String {
    use rand::Rng;
    const CHARSET: &[u8] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
    let mut rng = rand::rng();

    (0..128)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn generate_code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(digest)
}
