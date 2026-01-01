use crate::oauth::{Profile, ProfilesState};
use serde_json::json;
use tauri_plugin_store::StoreExt;

#[tauri::command]
pub async fn get_profiles(app: tauri::AppHandle) -> Result<ProfilesState, String> {
    let store = app.store("profiles.json").map_err(|e| e.to_string())?;

    let profiles_state: ProfilesState = store
        .get("profiles_state")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_else(|| ProfilesState {
            profiles: vec![],
            active_profile_id: None,
        });

    Ok(profiles_state)
}

#[tauri::command]
pub async fn add_profile(profile: Profile, app: tauri::AppHandle) -> Result<ProfilesState, String> {
    let store = app.store("profiles.json").map_err(|e| e.to_string())?;

    let mut profiles_state: ProfilesState = store
        .get("profiles_state")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_else(|| ProfilesState {
            profiles: vec![],
            active_profile_id: None,
        });

    // Check if profile already exists
    if let Some(existing_profile) = profiles_state
        .profiles
        .iter_mut()
        .find(|p| p.id == profile.id)
    {
        // Update existing profile
        *existing_profile = profile.clone();
    } else {
        // Add new profile
        profiles_state.profiles.push(profile.clone());
    }

    // Set as active if it's the only profile
    if profiles_state.profiles.len() == 1 {
        profiles_state.active_profile_id = Some(profile.id.clone());
    }

    // set() doesn't return a Result, it returns ()
    store.set("profiles_state".to_string(), json!(profiles_state));

    // save() returns a Result
    store.save().map_err(|e| e.to_string())?;

    Ok(profiles_state)
}

#[tauri::command]
pub async fn remove_profile(
    profile_id: String,
    app: tauri::AppHandle,
) -> Result<ProfilesState, String> {
    let store = app.store("profiles.json").map_err(|e| e.to_string())?;

    let mut profiles_state: ProfilesState = store
        .get("profiles_state")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_else(|| ProfilesState {
            profiles: vec![],
            active_profile_id: None,
        });

    profiles_state.profiles.retain(|p| p.id != profile_id);

    // If the removed profile was active, set a new active profile
    if profiles_state.active_profile_id.as_ref() == Some(&profile_id) {
        profiles_state.active_profile_id = profiles_state.profiles.first().map(|p| p.id.clone());
    }

    store.set("profiles_state".to_string(), json!(profiles_state));
    store.save().map_err(|e| e.to_string())?;

    Ok(profiles_state)
}

#[tauri::command]
pub async fn set_active_profile(
    profile_id: String,
    app: tauri::AppHandle,
) -> Result<ProfilesState, String> {
    let store = app.store("profiles.json").map_err(|e| e.to_string())?;

    let mut profiles_state: ProfilesState = store
        .get("profiles_state")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_else(|| ProfilesState {
            profiles: vec![],
            active_profile_id: None,
        });

    // Verify profile exists
    if profiles_state.profiles.iter().any(|p| p.id == profile_id) {
        profiles_state.active_profile_id = Some(profile_id);

        store.set("profiles_state".to_string(), json!(profiles_state));
        store.save().map_err(|e| e.to_string())?;

        Ok(profiles_state)
    } else {
        Err("Profile not found".to_string())
    }
}

#[tauri::command]
pub async fn get_active_profile(app: tauri::AppHandle) -> Result<Option<Profile>, String> {
    let store = app.store("profiles.json").map_err(|e| e.to_string())?;

    let profiles_state: ProfilesState = store
        .get("profiles_state")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_else(|| ProfilesState {
            profiles: vec![],
            active_profile_id: None,
        });

    if let Some(active_id) = profiles_state.active_profile_id {
        Ok(profiles_state
            .profiles
            .into_iter()
            .find(|p| p.id == active_id))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn update_profile_token(
    profile_id: String,
    access_token: String,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let store = app.store("profiles.json").map_err(|e| e.to_string())?;

    let mut profiles_state: ProfilesState = store
        .get("profiles_state")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_else(|| ProfilesState {
            profiles: vec![],
            active_profile_id: None,
        });

    if let Some(profile) = profiles_state
        .profiles
        .iter_mut()
        .find(|p| p.id == profile_id)
    {
        profile.access_token = access_token;

        store.set("profiles_state".to_string(), json!(profiles_state));
        store.save().map_err(|e| e.to_string())?;

        Ok(())
    } else {
        Err("Profile not found".to_string())
    }
}
