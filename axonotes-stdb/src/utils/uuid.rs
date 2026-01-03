use rand::RngCore;
use spacetimedb::ReducerContext;
use uuid::{Builder, Uuid};

/// Generate a random UUID v4 using SpacetimeDB's deterministic RNG
pub fn generate_uuid(ctx: &ReducerContext) -> Uuid {
    let mut rng = ctx.rng();
    let mut random_bytes = [0u8; 16];
    rng.fill_bytes(&mut random_bytes);
    Builder::from_random_bytes(random_bytes).into_uuid()
}
