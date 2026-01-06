use rand::RngCore;
use spacetimedb::ReducerContext;

const SHARE_CODE_CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const SHARE_CODE_LENGTH: usize = 8;

/// Generate a random 8-character share code (0-9, A-Z)
pub fn generate_share_code(ctx: &ReducerContext) -> String {
    let mut rng = ctx.rng();
    let mut code = String::with_capacity(SHARE_CODE_LENGTH);
    for _ in 0..SHARE_CODE_LENGTH {
        let idx = (rng.next_u32() as usize) % SHARE_CODE_CHARS.len();
        code.push(SHARE_CODE_CHARS[idx] as char);
    }
    code
}
