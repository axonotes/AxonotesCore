pub(crate) mod auth;

use rand::Rng;
use crate::user_profile::auth::manager::{AuthManager, AuthorizationCompleteResponse, StartNewAuthFlowResponse};

pub struct UserProfile {
    pub user_profile_id: u16, // A random id to identify the profile
    user_id: Option<String>, // An identifier that is derived from the token
    auth_manager: AuthManager
}

impl UserProfile {
    /// Creates a new `UserProfile` with a generated profile ID and default state.
    ///
    /// The new profile will have a randomly generated `user_profile_id`, `user_id` set to `None`,
    /// and an initialized `AuthManager`.
    ///
    /// # Examples
    ///
    /// ```
    /// let profile = UserProfile::new();
    /// assert!(profile.user_profile_id > 0);
    /// assert!(profile.user_id.is_none());
    /// ```
    pub fn new() -> Self {
        Self {
            user_profile_id: generate_user_profile_id(),
            user_id: None,
            auth_manager: AuthManager::new()
        }
    }

    /// Starts a new authentication flow for this user profile.
    ///
    /// Delegates to the internal `AuthManager` to initiate the authentication process and
    /// returns the manager's `StartNewAuthFlowResponse`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn run() {
    /// let mut profile = UserProfile::new();
    /// let resp = profile.start_new_auth_flow().await;
    /// // handle `resp` (e.g., open a URL or display a code)
    /// # }
    /// ```
    pub async fn start_new_auth_flow(&mut self) -> StartNewAuthFlowResponse {
        self.auth_manager.start_new_auth_flow().await
    }

    /// Waits for the authentication flow to complete and returns the result.
    ///
    /// This method awaits the underlying `AuthManager`'s `wait_for_authorization`
    /// future, updates `self.user_id` from the manager's `user_id` (if present),
    /// and returns the `AuthorizationCompleteResponse` produced by the manager.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::user_profile::UserProfile;
    /// # async fn example() {
    /// let mut profile = UserProfile::new();
    /// let result = profile.wait_for_authorization().await;
    /// // `profile.user_id` has been synchronized from the auth manager.
    /// # drop(result);
    /// # }
    /// ```
    pub async fn wait_for_authorization(&mut self) -> AuthorizationCompleteResponse {
        let res = self.auth_manager.wait_for_authorization().await;
        self.user_id = self.auth_manager.user_id.clone();
        res
    }
    
    /// Refreshes the authentication tokens for this user profile.
    ///
    /// Returns `true` if the tokens were successfully refreshed, or `false` if the refresh failed.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// # use crate::user_profile::UserProfile;
    /// # async fn example() {
    /// let mut profile = UserProfile::new();
    /// if profile.refresh_tokens().await {
    ///     // tokens refreshed
    /// } else {
    ///     // refresh failed
    /// }
    /// # }
    /// ```
    pub async fn refresh_tokens(&mut self) -> bool {
        self.auth_manager.refresh_tokens().await
    }
}

/// Generates a random `u16` value to use as a user profile identifier.
///
/// The value is produced by the crate's RNG and may be any `u16`.
///
/// # Examples
///
/// ```
/// let id = generate_user_profile_id();
/// assert!(id <= u16::MAX);
/// ```
fn generate_user_profile_id() -> u16 {
    let mut rng = rand::rng();
    rng.random()
}