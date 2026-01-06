use crate::crypto::bip39::get_mnemonic;
use crate::database::keys::Keys;
use crate::encryption::user::{
    decrypted_to_encrypted, generate_decrypted_keys, mnemonic_encrypted_to_decrypted,
    pwd_encrypted_to_decrypted, reencrypt_with_new_mnemonic, reencrypt_with_new_password,
    sign_encryption_update, UserKeysDecrypted, UserKeysEncrypted,
};
use crate::stdb_bindings::User;
use crate::{database, stdb};

/// This function can only be called once per user since it creates a new profile on the stdb server
#[tauri::command]
pub async fn create_stdb_user(password: String) -> Result<String, String> {
    let mnemonic = get_mnemonic().map_err(|e| format!("Failed to generate mnemonic: {e}"))?;
    let decrypted_keys = generate_decrypted_keys();
    let encrypted_keys =
        decrypted_to_encrypted(decrypted_keys.clone(), password, mnemonic.clone())?;

    // Store in local db
    database::save_active_user_keys(Keys {
        user_id: "".to_string(), // This is allowed to be empty since 'save_active_user_keys' overwrites with active user id
        public_encryption_key: decrypted_keys.public_encryption_key,
        private_encryption_key: decrypted_keys.private_encryption_key,
        public_signing_key: decrypted_keys.public_signing_key,
        private_signing_key: decrypted_keys.private_signing_key,
    })
    .await?;

    // Store on stdb
    stdb::active_profile().create_user(encrypted_keys).await?;

    Ok(mnemonic)
}

#[tauri::command]
pub async fn does_stdb_user_exist() -> Result<bool, String> {
    let user = stdb::active_profile().get_cached_user().await?;
    Ok(user.is_some())
}

#[tauri::command]
pub async fn do_stdb_keys_need_sync() -> Result<bool, String> {
    let keys = database::get_active_user_keys().await?;
    Ok(keys.is_none())
}

async fn get_user_or_error() -> Result<User, String> {
    let user = stdb::active_profile().get_cached_user().await?;
    if let Some(user) = user {
        return Ok(user);
    }

    if stdb::active_profile().is_connected().await.unwrap_or(false) {
        Err("No User found. Create a user first.".to_string())
    } else {
        Err("Not Connected with STDB server.".to_string())
    }
}

#[tauri::command]
pub async fn sync_stdb_keys_with_pwd(password: String) -> Result<(), String> {
    let user = get_user_or_error().await?;
    let decrypted_keys = pwd_encrypted_to_decrypted(user.into(), password)?;

    // Store in local db
    database::save_active_user_keys(Keys {
        user_id: "".to_string(), // This is allowed to be empty since 'save_active_user_keys' overwrites with active user id
        public_encryption_key: decrypted_keys.public_encryption_key,
        private_encryption_key: decrypted_keys.private_encryption_key,
        public_signing_key: decrypted_keys.public_signing_key,
        private_signing_key: decrypted_keys.private_signing_key,
    })
    .await?;

    Ok(())
}

#[tauri::command]
pub async fn sync_stdb_keys_with_mnemonic(mnemonic: String) -> Result<(), String> {
    let user = get_user_or_error().await?;
    let decrypted_keys = mnemonic_encrypted_to_decrypted(user.into(), mnemonic)?;

    // Store in local db
    database::save_active_user_keys(Keys {
        user_id: "".to_string(), // This is allowed to be empty since 'save_active_user_keys' overwrites with active user id
        public_encryption_key: decrypted_keys.public_encryption_key,
        private_encryption_key: decrypted_keys.private_encryption_key,
        public_signing_key: decrypted_keys.public_signing_key,
        private_signing_key: decrypted_keys.private_signing_key,
    })
    .await?;

    Ok(())
}

async fn update_encryption(
    user_keys_decrypted: UserKeysDecrypted,
    new_password: String,
    new_mnemonic: String,
) -> Result<(), String> {
    // Re-encrypt with new password and new mnemonic
    let encrypted_keys =
        decrypted_to_encrypted(user_keys_decrypted.clone(), new_password, new_mnemonic)?;

    // Sign the update
    let signature = sign_encryption_update(&user_keys_decrypted, &encrypted_keys)?;

    // Update encryption
    stdb::active_profile()
        .update_encryption_keys(encrypted_keys, signature)
        .await?;

    Ok(())
}

/// Returns a new mnemonic, old one is now Invalid
#[tauri::command]
pub async fn update_pwd_from_mnemonic(
    old_mnemonic: String,
    new_password: String,
) -> Result<String, String> {
    let user = get_user_or_error().await?;
    // Get old keys
    let decrypted_keys = mnemonic_encrypted_to_decrypted(user.into(), old_mnemonic)?;

    // Generate new mnemonic
    let new_mnemonic = get_mnemonic().map_err(|e| format!("Failed to generate mnemonic: {e}"))?;

    // Update stdb
    update_encryption(decrypted_keys, new_password, new_mnemonic.clone()).await?;

    Ok(new_mnemonic)
}

/// Returns a new mnemonic, old one is now Invalid
#[tauri::command]
pub async fn update_mnemonic_from_pwd(password: String) -> Result<String, String> {
    let user = get_user_or_error().await?;
    // Get old keys
    let decrypted_keys = pwd_encrypted_to_decrypted(user.into(), password.clone())?;

    // Generate new mnemonic
    let new_mnemonic = get_mnemonic().map_err(|e| format!("Failed to generate mnemonic: {e}"))?;

    // Update stdb
    update_encryption(decrypted_keys, password, new_mnemonic.clone()).await?;

    Ok(new_mnemonic)
}

/// Does not update mnemonic
#[tauri::command]
pub async fn update_pwd_from_pwd(old_password: String, new_password: String) -> Result<(), String> {
    let user = get_user_or_error().await?;
    let old_encrypted: UserKeysEncrypted = user.into();

    // Decrypt with old password
    let decrypted_keys = pwd_encrypted_to_decrypted(old_encrypted.clone(), old_password)?;

    // Re-encrypt only password-protected keys
    let encrypted_keys =
        reencrypt_with_new_password(decrypted_keys.clone(), old_encrypted, new_password)?;

    // Sign the update
    let signature = sign_encryption_update(&decrypted_keys, &encrypted_keys)?;

    // Update encryption
    stdb::active_profile()
        .update_encryption_keys(encrypted_keys, signature)
        .await?;

    Ok(())
}

/// Does not update password
#[tauri::command]
pub async fn update_mnemonic_from_mnemonic(old_mnemonic: String) -> Result<String, String> {
    let user = get_user_or_error().await?;
    let old_encrypted: UserKeysEncrypted = user.into();

    // Decrypt with old mnemonic
    let decrypted_keys = mnemonic_encrypted_to_decrypted(old_encrypted.clone(), old_mnemonic)?;

    // Generate new mnemonic
    let new_mnemonic = get_mnemonic().map_err(|e| format!("Failed to generate mnemonic: {e}"))?;

    // Re-encrypt only mnemonic-protected keys
    let encrypted_keys =
        reencrypt_with_new_mnemonic(decrypted_keys.clone(), old_encrypted, new_mnemonic.clone())?;

    // Sign the update
    let signature = sign_encryption_update(&decrypted_keys, &encrypted_keys)?;

    // Update encryption
    stdb::active_profile()
        .update_encryption_keys(encrypted_keys, signature)
        .await?;

    Ok(new_mnemonic)
}
