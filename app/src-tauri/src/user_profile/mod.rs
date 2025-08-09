pub(crate) mod auth;

use rand::Rng;
use crate::user_profile::auth::manager::{AuthManager, AuthorizationCompleteResponse, StartNewAuthFlowResponse};

pub struct UserProfile {
    pub user_profile_id: u16, // A random id to identify the profile
    user_id: Option<String>, // An identifier that is derived from the token
    auth_manager: AuthManager
}

impl UserProfile {
    pub fn new() -> Self {
        Self {
            user_profile_id: generate_user_profile_id(),
            user_id: None,
            auth_manager: AuthManager::new()
        }
    }

    pub async fn start_new_auth_flow(&mut self) -> StartNewAuthFlowResponse {
        self.auth_manager.start_new_auth_flow().await
    }

    pub async fn wait_for_authorization(&mut self) -> AuthorizationCompleteResponse {
        let res = self.auth_manager.wait_for_authorization().await;
        self.user_id = self.auth_manager.user_id.clone();
        res
    }
    
    pub async fn refresh_tokens(&mut self) -> bool {
        self.auth_manager.refresh_tokens().await
    }
}

fn generate_user_profile_id() -> u16 {
    let mut rng = rand::rng();
    rng.random()
}