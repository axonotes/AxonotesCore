use spacetimedb::{reducer, ReducerContext, Table};

// Module declarations
mod tables;
mod reducers;
mod utils;

// Re-export important items for easy access
pub use tables::*;
pub use reducers::*;
pub use utils::*;

pub use tables::user::user;

// Client connected reducer needs to be in lib.rs for SpacetimeDB to find it
#[reducer(client_connected)]
pub fn client_connected(ctx: &ReducerContext) {
    if let Some(_user) = ctx.db.user().identity().find(ctx.sender) {
        // User already exists, do nothing
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