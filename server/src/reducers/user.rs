use spacetimedb::{reducer, ReducerContext};
use crate::tables::User;
use crate::user;

/// Reducer for the initial setup of a user's encryption and signing keys.
/// This should only be called once when the user has no keys yet.
#[reducer]
pub fn init_encryption_and_signing(
    ctx: &ReducerContext,
    public_key: String,
    encrypted_private_key: String,
    encrypted_backup_key: String,
    public_signing_key: String,
    encrypted_private_signing_key: String,
    encrypted_private_backup_signing_key: String,
    argon_salt: String,
) -> Result<(), String> {
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        // Prevent overwriting existing keys with this reducer
        if user.public_key.is_some() || user.public_signing_key.is_some() {
            return Err("User keys are already initialized. Use update_encryption_keys instead.".to_string());
        }

        ctx.db.user().identity().update(User {
            public_key: Some(public_key),
            encrypted_private_key: Some(encrypted_private_key),
            encrypted_backup_key: Some(encrypted_backup_key),
            public_signing_key: Some(public_signing_key),
            encrypted_private_signing_key: Some(encrypted_private_signing_key),
            encrypted_private_backup_signing_key: Some(
                encrypted_private_backup_signing_key,
            ),
            argon_salt: Some(argon_salt),
            ..user
        });
        Ok(())
    } else {
        Err("Cannot set keys for an unknown user.".to_string())
    }
}