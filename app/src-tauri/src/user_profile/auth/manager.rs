use std::error::Error;
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
    /// Creates a new AuthManager with default configuration.
    ///
    /// The instance is initialized with:
    /// - `base_url` set to "http://localhost:5173"
    /// - an `OAuthClient` targeting the same base URL (client id set to `"unused"`)
    /// - `new_auth_flow`, `token`, and `user_id` set to `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mgr = AuthManager::new();
    /// assert_eq!(mgr.base_url, "http://localhost:5173");
    /// assert!(mgr.new_auth_flow.is_none());
    /// assert!(mgr.token.is_none());
    /// assert!(mgr.user_id.is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:5173".to_string(),
            oauth_client: OAuthClient::new("http://localhost:5173".to_string(), "unused".to_string()),
            new_auth_flow: None,
            token: None,
            user_id: None,
        }
    }

    /// Starts a new OAuth2 device authorization flow and stores the device-flow response on the manager.
    ///
    /// Returns a StartNewAuthFlowResponse containing the verification URL and user code on success,
    /// or an error_message when initiating the device flow fails. On success the device flow response
    /// is saved to `self.new_auth_flow` for subsequent polling.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tokio::runtime::Runtime;
    /// let mut mgr = AuthManager::new();
    /// let rt = Runtime::new().unwrap();
    /// let resp = rt.block_on(async { mgr.start_new_auth_flow().await });
    /// // `resp.complete_uri` and `resp.user_code` will be populated on success,
    /// // or `resp.error_message` will contain a human-readable error on failure.
    /// ```
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

    /// Polls the previously started device authorization flow until it completes, expires, or times out.
    ///
    /// This method requires that `start_new_auth_flow` was called successfully beforehand. It repeatedly
    /// polls the OAuth client for authorization, exchanges the device code for tokens on success,
    /// decodes the access token to derive and store `user_id`, and stores the received token response.
    /// Returns an `AuthorizationCompleteResponse` with `status = "complete"` on success or `status = "error"`
    /// with an explanatory `error_message` on failure (e.g., no flow started, device code expired, token
    /// exchange failed, or polling timed out).
    ///
    /// # Examples
    ///
    /// ```
    /// # use tokio::runtime::Runtime;
    /// # use app::user_profile::auth::manager::AuthManager;
    /// # let rt = Runtime::new().unwrap();
    /// # rt.block_on(async {
    /// let mut mgr = AuthManager::new();
    /// // Normally you would call `mgr.start_new_auth_flow().await` and show the user the code/URI.
    /// // Then wait for the user to authorize the device:
    /// let result = mgr.wait_for_authorization().await;
    /// match result.status.as_str() {
    ///     "complete" => println!("Authorization complete"),
    ///     "error" => eprintln!("Authorization failed: {:?}", result.error_message),
    ///     _ => {}
    /// }
    /// # });
    /// ```
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

    /// Attempts to refresh stored OAuth tokens using the current refresh token.
    ///
    /// If there is no token stored, this returns `false`. On success it replaces
    /// `self.token` with the newly obtained tokens and returns `true`. If the
    /// refresh request fails, `self.token` is left unchanged and `false` is
    /// returned.
    ///
    /// # Returns
    ///
    /// `true` if tokens were successfully refreshed and `self.token` was updated;
    /// `false` if no token was present or the refresh request failed.
    ///
    /// # Examples
    ///
    /// ```
    /// #[tokio::test]
    /// async fn refresh_tokens_example() {
    ///     let mut mgr = AuthManager::new();
    ///     // Without an existing token, refresh_tokens returns false.
    ///     assert!(!mgr.refresh_tokens().await);
    /// }
    /// ```
    pub async fn refresh_tokens(&mut self) -> bool {
        if self.token.is_none() {
            return false;
        }

        let old_refresh_token = self.token.clone().expect("Unexpected Error: Token should be something due to check.").refresh_token;

        let res = self.oauth_client.refresh_tokens(old_refresh_token).await;
        match res {
            Ok(new_tokens) => {
                self.token = new_tokens.into();
                true
            }
            Err(_) => {
                false
            }
        }
    }
}