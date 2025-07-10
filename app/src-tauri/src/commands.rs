use crate::auth::AuthError;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthStateResponse {
    pub is_authenticated: bool,
    pub user_info: Option<crate::auth::UserInfo>,
    pub is_waiting_for_auth: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateAuthRequest {
    pub force_new_login: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InitiateAuthResponse {
    pub auth_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigInfo {
    pub config: crate::config::AppConfig,
    pub config_path: String,
}

#[tauri::command]
pub async fn get_initiate_auth_url(
    state: State<'_, AppState>,
    request: InitiateAuthRequest,
) -> Result<InitiateAuthResponse, String> {
    let force_new_login = request.force_new_login.unwrap_or(false);

    match state.auth_manager.initiate_auth(force_new_login).await {
        Ok(auth_url) => {
            Ok(InitiateAuthResponse { auth_url })
        }
        Err(e) => {
            log::error!("Failed to initiate auth: {}", e);
            Err(format!("Failed to initiate authentication: {}", e))
        }
    }
}

#[tauri::command]
pub async fn handle_auth_callback(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    log::info!("Handling auth callback for session: {}", session_id);

    match state.auth_manager.retrieve_tokens(&session_id).await {
        Ok(()) => {
            log::info!(
                "Auth callback successful for user",
            );
            
            Ok(())
        }
        Err(e) => {
            log::error!("Auth callback failed: {}", e);
            Err(format!("Authentication failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn get_config_info(
    state: State<'_, AppState>,
) -> Result<ConfigInfo, String> {
    Ok(ConfigInfo {
        config: state.config.clone(),
        config_path: crate::config::AppConfig::get_config_path_string(),
    })
}

#[tauri::command]
pub async fn update_config(
    state: State<'_, AppState>,
    config: crate::config::AppConfig,
) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    log::info!("Configuration updated");
    Ok(())
}
