use crate::crypto::ed25519::verify_signature;
use crate::tables::private_user;
use spacetimedb::ReducerContext;

pub fn verify_message(
    ctx: &ReducerContext,
    message: &[u8],
    signature: &[u8; 64],
) -> Result<bool, Box<dyn std::error::Error>> {
    let user = ctx
        .db
        .private_user()
        .identity()
        .find(ctx.sender)
        .ok_or("User not found")?;

    let public_signing_key_array: &[u8; 32] = user
        .public_signing_key
        .as_slice()
        .try_into()
        .expect("Public signing key must be exactly 32 bytes");

    verify_signature(public_signing_key_array, message, signature)
}
