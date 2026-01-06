use crate::crypto::ed25519::sign_message;
use crate::database::get_active_user_keys;
use crate::database::keys::Keys;
use crate::share::key_rotation::rotate_keys_for_share;
use crate::share::sync::{
    emit_share_closed, get_share_code_for_doc, register_share_session, unregister_share_session,
};
use crate::stdb;
use crate::stdb_bindings::Role;
use crate::utils::vec_array::ByteArrayConversion;
use spacetimedb_sdk::Identity;

/// Parse role from string
fn parse_role(role_str: &str) -> Result<Role, String> {
    match role_str.to_lowercase().as_str() {
        "editor" => Ok(Role::Editor),
        "reader" => Ok(Role::Reader),
        "owner" => Err("Cannot share document with Owner role".to_string()),
        _ => Err(format!(
            "Invalid role: {}. Expected 'Editor' or 'Reader'",
            role_str
        )),
    }
}

/// Create a share session for a document
///
/// This initiates a share by:
/// 1. Registering a local share session
/// 2. Calling the create_pending_share reducer
/// 3. The share code will be emitted via "share-code-ready" event when received from server
///
/// # Arguments
/// * `doc_id` - The document to share
/// * `role` - The role to grant to joiners ("Editor" or "Reader")
/// * `full_history` - If true, joiners get all historical keys; if false, only the latest key
#[tauri::command]
pub async fn create_share(doc_id: String, role: String, full_history: bool) -> Result<(), String> {
    // Parse and validate role
    let role = parse_role(&role)?;

    // Get user keys for signing
    let user_keys: Keys = get_active_user_keys().await?.ok_or("No active user keys")?;

    let private_signing_key = user_keys.private_signing_key.as_array()?;

    // Sign the create_pending_share message
    let message = [b"create_pending_share", doc_id.as_bytes()].concat();
    let signature = sign_message(private_signing_key, &message)
        .map_err(|e| format!("Failed to sign message: {}", e))?;

    // Register local share session (before calling reducer so subscription can pick it up)
    register_share_session(doc_id.clone(), role, full_history).await;

    // Call the reducer
    match stdb::active_profile()
        .create_pending_share(doc_id.clone(), signature.to_vec())
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            // Cleanup on failure
            unregister_share_session(&doc_id).await;
            Err(e)
        }
    }
}

/// Join an existing share using a share code
///
/// This calls the join_pending_share reducer which:
/// 1. Auto-capitalizes the share code
/// 2. Adds the joiner's public key to the share request
/// 3. The share creator will auto-accept via their subscription
///
/// The joiner will receive access via the normal accessible_documents subscription
#[tauri::command]
pub async fn join_share(share_code: String) -> Result<(), String> {
    stdb::active_profile().join_pending_share(share_code).await
}

/// Close an active share session
///
/// This:
/// 1. Finds the share code for the document
/// 2. Calls close_pending_share reducer to cleanup server-side
/// 3. Unregisters the local share session
/// 4. Emits "share-closed" event
#[tauri::command]
pub async fn close_share(doc_id: String) -> Result<(), String> {
    // Get the share code for this document
    let share_code = get_share_code_for_doc(&doc_id)
        .await
        .ok_or("No active share session for this document")?;

    // Get user keys for signing
    let user_keys: Keys = get_active_user_keys().await?.ok_or("No active user keys")?;

    let private_signing_key = user_keys.private_signing_key.as_array()?;

    // Sign the close_pending_share message
    let message = [b"close_pending_share", share_code.as_bytes()].concat();
    let signature = sign_message(private_signing_key, &message)
        .map_err(|e| format!("Failed to sign message: {}", e))?;

    // Call the reducer
    stdb::active_profile()
        .close_pending_share(share_code.clone(), signature.to_vec())
        .await?;

    // Unregister local session
    unregister_share_session(&doc_id).await;

    // Emit closed event
    emit_share_closed(doc_id, share_code);

    Ok(())
}

/// Leave a share that you've joined (as a joiner, not creator)
///
/// This withdraws your share request before the creator accepts it
#[tauri::command]
pub async fn leave_share(share_code: String) -> Result<(), String> {
    stdb::active_profile().leave_pending_share(share_code).await
}

/// Update a user's role on a document
///
/// Can change between Editor and Reader roles.
/// Cannot change Owner role (use transfer_ownership instead).
///
/// # Arguments
/// * `doc_id` - The document ID
/// * `user_id` - The user's identity as hex string
/// * `role` - The new role ("editor" or "reader")
#[tauri::command]
pub async fn update_user_role(doc_id: String, user_id: String, role: String) -> Result<(), String> {
    // Parse role
    let new_role = parse_role(&role)?;

    // Parse user_id from hex string to Identity
    let user_identity =
        Identity::from_hex(&user_id).map_err(|e| format!("Invalid user ID: {}", e))?;

    // Get user keys for signing
    let user_keys: Keys = get_active_user_keys().await?.ok_or("No active user keys")?;
    let private_signing_key = user_keys.private_signing_key.as_array()?;

    // Convert role to byte for signature (must match server)
    let role_byte: u8 = match new_role {
        Role::Owner => 0,
        Role::Editor => 1,
        Role::Reader => 2,
    };

    // Sign the change_role message (must match server format)
    let message = [
        b"change_role".as_slice(),
        doc_id.as_bytes(),
        user_identity.to_byte_array().as_slice(),
        &[role_byte],
    ]
    .concat();

    let signature = sign_message(private_signing_key, &message)
        .map_err(|e| format!("Failed to sign message: {}", e))?;

    stdb::active_profile()
        .change_user_role(doc_id, user_identity, new_role, signature.to_vec())
        .await
}

/// Transfer document ownership to another user
///
/// Only the current owner can transfer ownership.
/// The old owner becomes an Editor.
///
/// # Arguments
/// * `doc_id` - The document ID
/// * `new_owner_id` - The new owner's identity as hex string
#[tauri::command]
pub async fn transfer_ownership(doc_id: String, new_owner_id: String) -> Result<(), String> {
    // Parse new_owner_id from hex string to Identity
    let new_owner_identity =
        Identity::from_hex(&new_owner_id).map_err(|e| format!("Invalid user ID: {}", e))?;

    // Get user keys for signing
    let user_keys: Keys = get_active_user_keys().await?.ok_or("No active user keys")?;
    let private_signing_key = user_keys.private_signing_key.as_array()?;

    // Sign the transfer_ownership message (must match server format)
    let message = [
        b"transfer_ownership".as_slice(),
        doc_id.as_bytes(),
        new_owner_identity.to_byte_array().as_slice(),
    ]
    .concat();

    let signature = sign_message(private_signing_key, &message)
        .map_err(|e| format!("Failed to sign message: {}", e))?;

    stdb::active_profile()
        .transfer_ownership(doc_id, new_owner_identity, signature.to_vec())
        .await
}

/// Remove a user from a document
///
/// This:
/// 1. Removes the user's permission and keys from the server
/// 2. Rotates document keys so the removed user can't decrypt new content
///
/// # Arguments
/// * `doc_id` - The document ID
/// * `user_id` - The user's identity as hex string
#[tauri::command]
pub async fn remove_user(doc_id: String, user_id: String) -> Result<(), String> {
    // Parse user_id from hex string to Identity
    let user_identity =
        Identity::from_hex(&user_id).map_err(|e| format!("Invalid user ID: {}", e))?;

    // Get user keys for signing
    let user_keys: Keys = get_active_user_keys().await?.ok_or("No active user keys")?;
    let private_signing_key = user_keys.private_signing_key.as_array()?;

    // Sign the remove_user message (must match server format)
    let message = [
        b"remove_user".as_slice(),
        doc_id.as_bytes(),
        user_identity.to_byte_array().as_slice(),
    ]
    .concat();

    let signature = sign_message(private_signing_key, &message)
        .map_err(|e| format!("Failed to sign message: {}", e))?;

    // Step 1: Remove user from document
    stdb::active_profile()
        .remove_user_from_document(doc_id.clone(), user_identity, signature.to_vec())
        .await?;

    // Step 2: Rotate keys so removed user can't decrypt new content
    rotate_keys_for_share(&doc_id).await?;

    Ok(())
}
