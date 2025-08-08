use std::time::Duration;
use tokio::time::sleep;
use serde::Serialize;
use crate::user_profile::auth::client::{DeviceAuthResponse, OAuthClient, TokenResponse};
use crate::user_profile::auth::jwk::{decode_jwt_safely, generate_user_id};

#[derive(Serialize)]
pub struct StartNewAuthFlowResponse {
    complete_uri: String,
    user_code: String,
    error_message: Option<String>,
}

#[derive(Serialize)]
pub struct AuthorizationCompleteResponse {
    status: String,
    error_message: Option<String>,
}

pub struct AuthManager {
    base_url: String,
    oauth_client: OAuthClient,
    new_auth_flow: Option<DeviceAuthResponse>,
    token: Option<TokenResponse>,
    pub user_id: Option<String>,
}

impl AuthManager {
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:5173".to_string(),
            oauth_client: OAuthClient::new("http://localhost:5173".to_string(), "unused".to_string()),
            new_auth_flow: None,
            token: None,
            user_id: None,
        }
    }

    pub async fn start_new_auth_flow(&mut self) -> StartNewAuthFlowResponse {
        let new_auth_flow_result = self.oauth_client.start_device_flow().await;

        if new_auth_flow_result.is_err() {
            return StartNewAuthFlowResponse {
                complete_uri: "".to_string(),
                user_code: "".to_string(),
                error_message: "Failed to initiate auth flow".to_string().into(),
            }
        }

        let new_auth_flow = new_auth_flow_result.expect("Unexpected Error: Failed to initiate auth flow. This error should never happen.");

        self.new_auth_flow = new_auth_flow.clone().into();

        StartNewAuthFlowResponse {
            complete_uri: new_auth_flow.verification_uri_complete,
            user_code: new_auth_flow.user_code,
            error_message: None,
        }
    }

    pub async fn wait_for_authorization(&mut self) -> AuthorizationCompleteResponse {
        let auth_response = self.new_auth_flow.clone();
        if auth_response.is_none() {
            return AuthorizationCompleteResponse {
                status: "error".to_string(),
                error_message: "Can't wait for authorization if none was started.".to_string().into(),
            }
        }

        let auth_response = auth_response.expect("Unexpected Error: Failed to get DeviceAuthResponse. This error should never happen.");
        let poll_interval = Duration::from_secs(auth_response.interval as u64);
        let max_attempts = (auth_response.expires_in / auth_response.interval) + 5;

        for attempt in 1..=max_attempts {
            sleep(poll_interval).await;

            match self.oauth_client.poll_for_authorization(&auth_response.device_code).await {
                Ok(true) => {
                    let exchange_response = self.oauth_client.exchange_for_tokens(&auth_response.device_code).await;
                    if exchange_response.is_err() {
                        return AuthorizationCompleteResponse {
                            status: "error".to_string(),
                            error_message: "Failed to exchange device code for token".to_string().into(),
                        }
                    }
                    let exchange_response = exchange_response.expect("Unexpected Error: Failed to exchange device code for token. This error should never happen.");

                    self.token = exchange_response.clone().into();

                    let token_claims_result = decode_jwt_safely(&exchange_response.access_token, &self.base_url).await;
                    if token_claims_result.is_err() {
                        return AuthorizationCompleteResponse {
                            status: "error".to_string(),
                            error_message: "Failed to get token claims".to_string().into(),
                        }
                    }
                    let token_claims = token_claims_result.expect("Unexpected Error: Failed to get token claims. This error should never happen.");
                    self.user_id = generate_user_id(&token_claims).into();

                    return AuthorizationCompleteResponse {
                        status: "complete".to_string(),
                        error_message: None,
                    }
                }
                Ok(false) => {
                    // Still waiting
                }
                Err(e) => {
                    if e.to_string().contains("expired") {
                        return AuthorizationCompleteResponse {
                            status: "error".to_string(),
                            error_message: "Device code expired".to_string().into(),
                        }
                    }

                    // For other errors continue polling
                }
            }
        }

        AuthorizationCompleteResponse {
            status: "error".to_string(),
            error_message: "Authentication timed out".to_string().into(),
        }
    }
}