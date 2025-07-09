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
pub async fn get_auth_state(
    state: State<'_, AppState>,
) -> Result<AuthStateResponse, String> {
    let auth_state = state.get_auth_state().await;

    Ok(AuthStateResponse {
        is_authenticated: auth_state.is_authenticated,
        user_info: auth_state.user_info,
        is_waiting_for_auth: auth_state.is_waiting_for_auth,
    })
}

#[tauri::command]
pub async fn get_initiate_auth_url(
    state: State<'_, AppState>,
    request: InitiateAuthRequest,
) -> Result<InitiateAuthResponse, String> {
    let force_new_login = request.force_new_login.unwrap_or(false);

    // Set waiting state
    state.set_waiting_for_auth(true).await;

    match state.auth_manager.initiate_auth(force_new_login).await {
        Ok(auth_url) => {
            Ok(InitiateAuthResponse { auth_url })
        }
        Err(e) => {
            log::error!("Failed to initiate auth: {}", e);
            state.set_waiting_for_auth(false).await;
            Err(format!("Failed to initiate authentication: {}", e))
        }
    }
}

#[tauri::command]
pub async fn handle_auth_callback(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<crate::auth::TokenResponse, String> {
    log::info!("Handling auth callback for session: {}", session_id);

    match state.auth_manager.retrieve_tokens(&session_id).await {
        Ok(token_response) => {
            log::info!(
                "Auth callback successful for user: {}",
                token_response.user.email
            );
            state.set_authenticated(token_response.clone()).await;
            Ok(token_response)
        }
        Err(e) => {
            log::error!("Auth callback failed: {}", e);
            state.set_waiting_for_auth(false).await;
            Err(format!("Authentication failed: {}", e))
        }
    }
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), String> {
    log::info!("User logged out");
    state.clear_auth().await;
    Ok(())
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

#[tauri::command]
pub async fn refresh_token(
    state: State<'_, AppState>,
    refresh_token: String,
) -> Result<crate::auth::RefreshTokenResponse, String> {
    match state.auth_manager.refresh_token(&refresh_token).await {
        Ok(refresh_response) => {
            log::info!("Token refreshed successfully");
            Ok(refresh_response)
        }
        Err(e) => {
            log::error!("Token refresh failed: {}", e);
            Err(format!("Failed to refresh token: {}", e))
        }
    }
}
