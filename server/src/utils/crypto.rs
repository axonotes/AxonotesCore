use base64::engine::general_purpose;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use spacetimedb::ReducerContext;
use crate::user;

/// Helper function to verify a signature for the current user
/// Returns Ok(()) if signature is valid, Err(String) if invalid or any error occurs
pub fn verify_user_signature(
    ctx: &ReducerContext,
    message: &str,
    signature_base64: &str,
) -> Result<(), String> {
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender)
        .ok_or_else(|| "Cannot verify signature for unknown user.".to_string())?;

    let public_signing_key_str = user.public_signing_key.as_ref().ok_or_else(|| {
        "User has no public signing key to verify with. Has the account been initialized?"
            .to_string()
    })?;

    let public_key_vec: Vec<u8> = general_purpose::STANDARD
        .decode(public_signing_key_str)
        .map_err(|e| format!("Failed to decode public signing key: {}", e))?;

    let signature_bytes: Vec<u8> = general_purpose::STANDARD
        .decode(signature_base64)
        .map_err(|e| format!("Failed to decode signature: {}", e))?;

    let public_key_array: [u8; 32] = public_key_vec.try_into().map_err(|_| {
        "Public signing key is not the correct length (expected 32 bytes).".to_string()
    })?;

    let verifying_key = VerifyingKey::from_bytes(&public_key_array)
        .map_err(|e| format!("Invalid public signing key format: {}", e))?;

    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|e| format!("Invalid signature format: {}", e))?;

    verifying_key
        .verify_strict(message.as_bytes(), &signature)
        .map_err(|_| "Signature verification failed. Unauthorized.".to_string())?;

    Ok(())
}