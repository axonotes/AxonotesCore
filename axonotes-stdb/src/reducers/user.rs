use crate::tables::{private_user, User};
use crate::utils::auth::verify_message;
use spacetimedb::{ReducerContext, Table};

#[spacetimedb::reducer]
pub fn create_user(
    ctx: &ReducerContext,

    // Asymmetric Encryption
    public_encryption_key: Vec<u8>,
    pwd_encrypted_private_encryption_key: Vec<u8>,
    mnemonic_encrypted_private_encryption_key: Vec<u8>,

    // Signing
    public_signing_key: Vec<u8>,
    pwd_encrypted_private_signing_key: Vec<u8>,
    mnemonic_encrypted_private_signing_key: Vec<u8>,
) {
    // Check if user already exists
    if ctx.db.private_user().identity().find(ctx.sender).is_some() {
        return;
    }

    ctx.db.private_user().insert(User {
        identity: ctx.sender,
        public_encryption_key,
        pwd_encrypted_private_encryption_key,
        mnemonic_encrypted_private_encryption_key,
        public_signing_key,
        pwd_encrypted_private_signing_key,
        mnemonic_encrypted_private_signing_key,
    });
}

#[spacetimedb::reducer]
pub fn set_encryption_keys(
    ctx: &ReducerContext,

    // Asymmetric Encryption
    public_encryption_key: Vec<u8>,
    pwd_encrypted_private_encryption_key: Vec<u8>,
    mnemonic_encrypted_private_encryption_key: Vec<u8>,

    // Signing
    public_signing_key: Vec<u8>,
    pwd_encrypted_private_signing_key: Vec<u8>,
    mnemonic_encrypted_private_signing_key: Vec<u8>,

    // Signature
    signature: Vec<u8>,
) {
    let user = ctx
        .db
        .private_user()
        .identity()
        .find(ctx.sender)
        .expect("User not found. Use 'create_user' first.");

    let message: Vec<u8> = [
        &public_encryption_key[..],
        &pwd_encrypted_private_encryption_key[..],
        &mnemonic_encrypted_private_encryption_key[..],
        &public_signing_key[..],
        &pwd_encrypted_private_signing_key[..],
        &mnemonic_encrypted_private_signing_key[..],
    ]
    .concat();

    let signature_array: &[u8; 64] = signature
        .as_slice()
        .try_into()
        .expect("Signature must be exactly 64 bytes");

    match verify_message(ctx, message.as_slice(), signature_array) {
        Ok(true) => {
            ctx.db.private_user().identity().update(User {
                public_encryption_key,
                pwd_encrypted_private_encryption_key,
                mnemonic_encrypted_private_encryption_key,
                public_signing_key,
                pwd_encrypted_private_signing_key,
                mnemonic_encrypted_private_signing_key,
                ..user
            });
        }
        Ok(false) => panic!("Unauthorized: Signature verification failed"),
        Err(e) => panic!("Error verifying signature: {}", e),
    }
}
