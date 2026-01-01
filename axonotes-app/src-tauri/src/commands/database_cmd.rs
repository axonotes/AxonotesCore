use crate::database;

#[tauri::command]
pub async fn get_unlock_mode() -> Result<String, String> {
    database::get_unlock_mode()
}

#[tauri::command]
pub async fn switch_unlock_mode(new_mode: String) -> Result<(), String> {
    database::switch_unlock_mode_ui(new_mode)
}

#[tauri::command]
pub async fn unlock_database(password: String) -> Result<(), String> {
    // Convert empty string to None
    let pwd = if password.is_empty() {
        None
    } else {
        Some(password)
    };
    database::unlock_db(pwd).await
}

#[tauri::command]
pub async fn is_database_unlocked() -> Result<bool, String> {
    Ok(database::is_unlocked().await)
}

#[tauri::command]
pub async fn wipe_database() -> Result<(), String> {
    database::wipe_db().await
}

#[tauri::command]
pub async fn set_database_encryption(
    new_password: String,
    mode: String,
) -> Result<(), String> {
    database::set_encryption(new_password, mode).await
}

#[tauri::command]
pub async fn remove_database_encryption() -> Result<(), String> {
    database::remove_encryption().await
}