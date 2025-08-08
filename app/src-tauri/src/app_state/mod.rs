use crate::user_profile::auth::manager::{
    AuthorizationCompleteResponse, StartNewAuthFlowResponse,
};
use crate::user_profile::UserProfile;
use anyhow::Result;
use serde::{Serialize};
use tauri::State;
use tokio::sync::Mutex;

#[derive(Serialize)]
pub struct AddAndAuthNewUserResponse {
    start_new_auth_flow_response: StartNewAuthFlowResponse,
    user_profile_id: u16,
}

pub struct AppState {
    user_profiles: Mutex<Vec<UserProfile>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            user_profiles: Mutex::new(Vec::new()),
        }
    }
}

#[tauri::command]
pub async fn add_and_auth_new_user(
    state: State<'_, AppState>,
) -> Result<AddAndAuthNewUserResponse, String> {
    let mut new_user_profile = UserProfile::new();
    let res = new_user_profile.start_new_auth_flow().await;
    let id = new_user_profile.user_profile_id;

    let mut profiles = state.user_profiles.lock().await;
    profiles.push(new_user_profile);

    Ok(AddAndAuthNewUserResponse {
        start_new_auth_flow_response: res,
        user_profile_id: id,
    })
}

#[tauri::command(rename_all = "snake_case")]
pub async fn wait_for_user_auth(
    state: State<'_, AppState>,
    user_profile_id: u16,
) -> Result<AuthorizationCompleteResponse, String> {
    let mut profiles = state.user_profiles.lock().await;

    match profiles
        .iter_mut()
        .find(|profile| profile.user_profile_id == user_profile_id)
    {
        Some(profile) => Ok(profile.wait_for_authorization().await),
        None => Err("User profile not found".into()),
    }
}
