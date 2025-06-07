use spacetimedb::{reducer, table, Identity, ReducerContext, Table};

#[table(name = user, public)]
pub struct User {
    #[primary_key]
    identity: Identity,

    // Encryption
    public_key: Option<String>,
    encrypted_private_key: Option<String>,
    encrypted_backup_key: Option<String>,
    argon_salt: Option<String>,
}

#[reducer]
pub fn set_encryption(
    ctx: &ReducerContext,
    public_key: String,
    encrypted_private_key: String,
    encrypted_backup_key: String,
    argon_salt: String,
) -> Result<(), String> {
    if let Some(user) = ctx.db.user().identity().find(ctx.sender) {
        ctx.db.user().identity().update(User {
            public_key: Some(public_key),
            encrypted_private_key: Some(encrypted_private_key),
            encrypted_backup_key: Some(encrypted_backup_key),
            argon_salt: Some(argon_salt),
            ..user
        });
        Ok(())
    } else {
        Err("Cannot set encryption for unknown user".to_string())
    }
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
            argon_salt: None,
        });
    }
}
