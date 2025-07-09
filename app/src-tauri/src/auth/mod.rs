use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::commands::InitiateAuthResponse;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Session not found")]
    SessionNotFound,
    #[error("Invalid response format")]
    InvalidResponse,
    #[error("Authentication failed: {0}")]
    AuthFailed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: u64,
    pub user: UserInfo,
    #[serde(rename = "workosSessionId")]
    pub workos_session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenRequest {
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: u64,
    pub user: UserInfo,
}

pub struct AuthManager {
    client: Client,
    dashboard_url: String,
}

impl AuthManager {
    pub fn new(dashboard_url: String) -> Self {
        Self {
            client: Client::new(),
            dashboard_url,
        }
    }

    /// Returns the "Initiate authentication" URL
    pub async fn initiate_auth(
        &self,
        force_new_login: bool,
    ) -> Result<String, AuthError> {
        let url = if force_new_login {
            format!("{}/auth/desktop?force_new_login=true", self.dashboard_url)
        } else {
            format!("{}/auth/desktop", self.dashboard_url)
        };

        Ok(url)
    }

    /// Retrieve tokens using the session ID received from deep link
    pub async fn retrieve_tokens(
        &self,
        session_id: &str,
    ) -> Result<TokenResponse, AuthError> {
        let url =
            format!("{}/api/auth/desktop/{}", self.dashboard_url, session_id);

        log::info!("Retrieving tokens from: {}", url);

        let response = self.client.get(&url).send().await?;

        match response.status().as_u16() {
            200 => {
                let token_response: TokenResponse = response.json().await?;
                log::info!(
                    "Successfully retrieved tokens for user: {}",
                    token_response.user.email
                );
                Ok(token_response)
            }
            404 => Err(AuthError::SessionNotFound),
            400 => Err(AuthError::AuthFailed("Invalid request".to_string())),
            401 => Err(AuthError::AuthFailed(
                "Invalid or expired session".to_string(),
            )),
            500 => {
                Err(AuthError::AuthFailed("Internal server error".to_string()))
            }
            status => Err(AuthError::AuthFailed(format!(
                "Unexpected status: {}",
                status
            ))),
        }
    }

    /// Refresh an expired access token
    pub async fn refresh_token(
        &self,
        refresh_token: &str,
    ) -> Result<RefreshTokenResponse, AuthError> {
        let url = format!("{}/api/auth/desktop/refresh", self.dashboard_url);

        let request = RefreshTokenRequest {
            refresh_token: refresh_token.to_string(),
        };

        log::info!("Refreshing token at: {}", url);

        let response = self.client.post(&url).json(&request).send().await?;

        match response.status().as_u16() {
            200 => {
                let refresh_response: RefreshTokenResponse =
                    response.json().await?;
                log::info!(
                    "Successfully refreshed token for user: {}",
                    refresh_response.user.email
                );
                Ok(refresh_response)
            }
            400 => Err(AuthError::AuthFailed(
                "Invalid refresh token request".to_string(),
            )),
            401 => Err(AuthError::AuthFailed(
                "Invalid or expired refresh token".to_string(),
            )),
            500 => {
                Err(AuthError::AuthFailed("Internal server error".to_string()))
            }
            status => {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                Err(AuthError::AuthFailed(format!(
                    "Status {}: {}",
                    status, error_text
                )))
            }
        }
    }
}
