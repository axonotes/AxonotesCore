use crate::{database, workos_auth};
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub async fn start_login(app: AppHandle, dark_mode: bool) -> Result<String, String> {
    let auth_url = workos_auth::start_auth_flow(dark_mode, move |result| {
        match result {
            Ok(profile) => {
                // Save profile to database and set as active
                let app_clone = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = database::save_profile(profile.clone(), true).await {
                        eprintln!("Failed to save profile: {}", e);
                        return;
                    }

                    // Emit event to frontend
                    let _ = app_clone.emit("login-success", &profile);
                });
            }
            Err(error) => {
                eprintln!("Login failed: {}", error);
                let _ = app.emit("login-error", &error);
            }
        }
    })
    .await?;

    Ok(auth_url)
}

#[tauri::command]
pub async fn logout(profile_id: String) -> Result<(), String> {
    database::delete_profile(profile_id).await
}
