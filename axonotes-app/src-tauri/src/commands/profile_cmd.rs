use serde::{Deserialize, Serialize};
use crate::{database, workos_auth};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendProfile {
    pub id: String,
    pub email: String,
    pub name: String,
}

impl From<workos_auth::Profile> for FrontendProfile {
    fn from(p: workos_auth::Profile) -> Self {
        FrontendProfile {
            id: p.id,
            email: p.email,
            name: p.name,
        }
    }
}

#[tauri::command]
pub async fn get_active_profile() -> Result<Option<FrontendProfile>, String> {
    database::get_active_profile().await
        .map_err(|e| e.to_string())
        .map(|opt| opt.map(|workos_profile| workos_profile.into()))
}

#[tauri::command]
pub async fn get_all_profiles() -> Result<Vec<FrontendProfile>, String> {
    database::get_all_profiles().await
        .map_err(|e| e.to_string())
        .map(|profiles| profiles.into_iter().map(Into::into).collect())
}

#[tauri::command]
pub async fn switch_profile(profile_id: String) -> Result<(), String> {
    database::set_active_profile(profile_id).await
}

#[tauri::command]
pub async fn refresh_token() -> Result<String, String> {
    database::refresh_active_profile_token().await
}