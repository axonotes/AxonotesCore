use spacetimedb::ReducerContext;

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    // Called when the module is initially published
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(ctx: &ReducerContext) -> Result<(), String> {
    let auth_ctx = ctx.sender_auth();
    let (subject, issuer) = match auth_ctx.jwt() {
        Some(claims) => (claims.subject().to_string(), claims.issuer().to_string()),
        None => {
            return Err("Client connected without JWT".to_string());
        }
    };
    // TODO: for now just log, in the future we only allow our workos jwt's
    log::info!("sub: {}, iss: {}", subject, issuer);
    Ok(())
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(_ctx: &ReducerContext) {
    // Called everytime a client disconnects
}
