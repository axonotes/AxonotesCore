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

#[derive(Debug, Serialize)]
struct TokenRefreshBodyRequest {
    refresh_token: String,
}

pub struct OAuthClient {
    client: Client,
    base_url: String,
    client_id: String,
    code_verifier: String,
    code_challenge: String,
}

impl OAuthClient {
    /// Creates a new OAuthClient configured for the device authorization flow with PKCE.
    ///
    /// Generates a cryptographically random PKCE `code_verifier` and its corresponding
    /// `code_challenge`, builds an internal HTTP client, and stores the provided
    /// `base_url` and `client_id`.
    ///
    /// # Examples
    ///
    /// ```
    /// let client = OAuthClient::new("https://api.example.com".to_string(), "my-client-id".to_string());
    /// ```
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

    /// Initiates the OAuth 2.0 Device Authorization request (PKCE) and returns the device authorization response.
    ///
    /// Sends a POST to `{base_url}/oauth/device/authorize` with the client ID, PKCE code challenge, S256 method, and the `openid profile email` scope. On HTTP success the response is parsed as `DeviceAuthResponse`. On non-success the response body is returned as an error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use app::user_profile::auth::client::OAuthClient;
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let client = OAuthClient::new("https://auth.example.com".into(), "my-client-id".into());
    /// let auth = client.start_device_flow().await;
    /// match auth {
    ///     Ok(resp) => {
    ///         println!("User code: {}", resp.user_code);
    ///         println!("Visit: {}", resp.verification_uri_complete);
    ///     }
    ///     Err(e) => eprintln!("Device flow failed: {}", e),
    /// }
    /// # });
    /// ```
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

    /// Polls the device token endpoint to check whether the user has completed authorization for the given device code.
    ///
    /// Returns `Ok(true)` when the server reports the device flow status as `"complete"`. Returns `Ok(false)` when
    /// authorization is still pending or the server asks the client to slow down (in which case the function delays 2s
    /// before returning). Returns `Err` for terminal errors such as an expired device code or other HTTP/JSON-reported errors.
    ///
    /// # Parameters
    ///
    /// - `device_code` — the device_code received from the device authorization response; this ties the poll request to the
    ///   in-progress device authorization session.
    ///
    /// # Errors
    ///
    /// - Returns an error when the server responds with `expired_token`.
    /// - Returns an error for non-400 HTTP failures or when the server reports an error other than `authorization_pending` or `slow_down`.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn run_example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = OAuthClient::new("https://example.com".to_string(), "client_id".to_string());
    /// // `device_code` would normally come from start_device_flow()
    /// let authorized = client.poll_for_authorization("example_device_code").await?;
    /// if authorized {
    ///     // proceed to exchange_for_tokens(...)
    /// }
    /// # Ok(()) }
    /// ```
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

    /// Exchanges a device authorization code for OAuth 2.0 tokens.
    ///
    /// Sends a form-encoded POST to the client's `{base_url}/oauth/device/token` endpoint
    /// using the device-code grant type with the stored PKCE `code_verifier`. On success
    /// returns a parsed `TokenResponse`. If the endpoint responds with an error status,
    /// the server `error_description` is returned as an Err.
    ///
    /// # Parameters
    ///
    /// - `device_code`: the device code previously obtained from `start_device_flow`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use app::user_profile::auth::client::OAuthClient;
    /// # tokio::runtime::Runtime::new().unwrap().block_on(async {
    /// let client = OAuthClient::new("https://auth.example.com".into(), "my-client-id".into());
    /// let result = client.exchange_for_tokens("DEVICE_CODE_VALUE").await;
    /// match result {
    ///     Ok(tokens) => println!("access_token={}", tokens.access_token),
    ///     Err(e) => eprintln!("exchange failed: {}", e),
    /// }
    /// # });
    /// ```
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

    /// Refreshes OAuth tokens using a refresh token.
    ///
    /// Sends the refresh token to the backend refresh endpoint (`/api/auth/refresh`)
    /// and returns a new TokenResponse on success.
    ///
    /// # Returns
    /// A `TokenResponse` containing fresh access and refresh tokens on success.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use tokio;
    /// # #[tokio::test]
    /// # async fn example() -> Result<(), Box<dyn Error>> {
    /// let client = OAuthClient::new("https://auth.example.com".to_string(), "client-id".to_string());
    /// let refreshed = client.refresh_tokens("existing-refresh-token".to_string()).await?;
    /// assert!(!refreshed.access_token.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn refresh_tokens(&self, refresh_token: String) -> Result<TokenResponse, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/api/auth/refresh", self.base_url);

        let request_body = TokenRefreshBodyRequest {
            refresh_token
        };

        let response = self.client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        Ok(response.json().await?)
    }
}

// PKCE helper functions
/// Generates a cryptographically random PKCE code verifier.
///
/// The verifier is a 128-character string drawn from the unreserved characters
/// allowed by the PKCE specification: ASCII letters, digits, and "-._~".
///
/// # Examples
///
/// ```
/// let verifier = generate_code_verifier();
/// assert_eq!(verifier.len(), 128);
/// let allowed = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~";
/// assert!(verifier.chars().all(|c| allowed.contains(c)));
/// ```
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

/// Produce a PKCE "S256" code challenge from a code verifier.
///
/// The function computes the SHA-256 digest of `verifier` and returns the
/// URL-safe base64 (no padding) encoding of that digest, suitable for use as
/// the `code_challenge` in the OAuth 2.0 PKCE S256 flow.
///
/// # Examples
///
/// ```
/// let verifier = "test";
/// let challenge = generate_code_challenge(verifier);
/// assert_eq!(challenge, "n4bQgYhx9leK-qoMVrQFaO_TxssLgi0V1sFbDwCgg");
/// ```
fn generate_code_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(digest)
}
