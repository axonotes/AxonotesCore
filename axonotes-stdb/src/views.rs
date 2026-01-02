use crate::tables::{private_user__view, User};
use spacetimedb::rt::IntoVec;
use spacetimedb::ViewContext;

#[spacetimedb::view(name = user, public)]
pub fn user_view(ctx: &ViewContext) -> Vec<User> {
    ctx.db.private_user().identity().find(ctx.sender).into_vec()
}
