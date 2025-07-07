use spacetimedb::{reducer, ReducerContext};
use crate::utils::verify_user_signature;
use crate::tables::User;
use crate::user;

/// Reducer to update a user's encrypted keys, e.g., after a password change.
/// Requires a valid signature to authorize the change.
#[reducer]
pub fn update_encryption_keys(
    ctx: &ReducerContext,
    new_encrypted_private_key: String,
    new_encrypted_private_signing_key: String,
    new_argon_salt: String,
    signature_base64: String,
) -> Result<(), String> {
    let message_to_verify = format!(
        "{}{}{}",
        new_encrypted_private_key, new_encrypted_private_signing_key, new_argon_salt
    );

    // Use the helper function to verify the signature
    verify_user_signature(ctx, &message_to_verify, &signature_base64)?;

    // If we get here, the signature was valid, so proceed with the update
    let user = ctx
        .db
        .user()
        .identity()
        .find(ctx.sender)
        .ok_or_else(|| "Cannot update keys for an unknown user.".to_string())?;

    ctx.db.user().identity().update(User {
        encrypted_private_key: Some(new_encrypted_private_key),
        encrypted_private_signing_key: Some(new_encrypted_private_signing_key),
        argon_salt: Some(new_argon_salt),
        ..user
    });

    Ok(())
}