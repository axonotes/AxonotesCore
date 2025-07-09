use crate::auth::{AuthManager, TokenResponse};
use crate::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthState {
    pub is_authenticated: bool,
    pub user_info: Option<crate::auth::UserInfo>,
    pub is_waiting_for_auth: bool,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            is_authenticated: false,
            user_info: None,
            is_waiting_for_auth: false,
        }
    }
}

pub struct AppState {
    pub config: AppConfig,
    pub auth_manager: AuthManager,
    pub auth_state: Arc<RwLock<AuthState>>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let auth_manager = AuthManager::new(config.dashboard_url.clone());

        Self {
            config,
            auth_manager,
            auth_state: Arc::new(RwLock::new(AuthState::default())),
        }
    }

    pub async fn set_authenticated(&self, token_response: TokenResponse) {
        let mut auth_state = self.auth_state.write().await;
        auth_state.is_authenticated = true;
        auth_state.user_info = Some(token_response.user);
        auth_state.is_waiting_for_auth = false;
    }

    pub async fn set_waiting_for_auth(&self, waiting: bool) {
        let mut auth_state = self.auth_state.write().await;
        auth_state.is_waiting_for_auth = waiting;
    }

    pub async fn clear_auth(&self) {
        let mut auth_state = self.auth_state.write().await;
        *auth_state = AuthState::default();
    }

    pub async fn get_auth_state(&self) -> AuthState {
        self.auth_state.read().await.clone()
    }
}
