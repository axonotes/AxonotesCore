use base64::engine::general_purpose;
use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use spacetimedb::{reducer, table, Identity, ReducerContext, Table};

#[table(name = user, public)]
pub struct User {
    #[primary_key]
    identity: Identity,

    // Encryption Keys (RSA-OAEP)
    public_key: Option<String>,
    encrypted_private_key: Option<String>,
    encrypted_backup_key: Option<String>,

    // Signing Keys (Ed25519)
    public_signing_key: Option<String>,
    encrypted_private_signing_key: Option<String>,
    encrypted_private_backup_signing_key: Option<String>,

    // Password Hashing
    argon_salt: Option<String>,
}

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
    let user =
        ctx.db.user().identity().find(ctx.sender).ok_or_else(|| {
            "Cannot update keys for an unknown user.".to_string()
        })?;

    let public_signing_key_str = user.public_signing_key
        .as_ref()
        .ok_or_else(|| "User has no public signing key to verify with. Has the account been initialized?".to_string())?;

    let public_key_vec: Vec<u8> = general_purpose::STANDARD
        .decode(public_signing_key_str)
        .map_err(|e| format!("Failed to decode public signing key: {}", e))?;
    let signature_bytes: Vec<u8> = general_purpose::STANDARD
        .decode(signature_base64)
        .map_err(|e| format!("Failed to decode signature: {}", e))?;

    let public_key_array: [u8; 32] =
        public_key_vec.try_into().map_err(|_| {
            "Public signing key is not the correct length (expected 32 bytes)."
                .to_string()
        })?;

    let message_to_verify = format!(
        "{}{}{}",
        new_encrypted_private_key,
        new_encrypted_private_signing_key,
        new_argon_salt
    );

    let verifying_key = VerifyingKey::from_bytes(&public_key_array)
        .map_err(|e| format!("Invalid public signing key format: {}", e))?;

    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|e| format!("Invalid signature format: {}", e))?;

    verifying_key
        .verify_strict(message_to_verify.as_bytes(), &signature)
        .map_err(|_| {
            "Signature verification failed. Unauthorized.".to_string()
        })?;

    ctx.db.user().identity().update(User {
        encrypted_private_key: Some(new_encrypted_private_key),
        encrypted_private_signing_key: Some(new_encrypted_private_signing_key),
        argon_salt: Some(new_argon_salt),
        ..user
    });

    Ok(())
}

#[reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    if let Some(_user) = ctx.db.user().identity().find(ctx.sender) {
    } else {
        ctx.db.user().insert(User {
            identity: ctx.sender,
            public_key: None,
            encrypted_backup_key: None,
            encrypted_private_key: None,
            public_signing_key: None,
            encrypted_private_signing_key: None,
            encrypted_private_backup_signing_key: None,
            argon_salt: None,
        });
    }
}
