use crate::auth::{AuthManager, TokenResponse};
use crate::config::AppConfig;

pub struct AppState {
    pub config: AppConfig,
    pub auth_manager: AuthManager,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let auth_manager = AuthManager::new(config.dashboard_url.clone());
        
        Self {
            config,
            auth_manager,
        }
    }
}
