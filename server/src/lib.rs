use spacetimedb::{Deserialize, Identity, ReducerContext, Serialize, Table, Timestamp};

#[spacetimedb::table(name = person)]
pub struct Person {
    name: String,
}

#[spacetimedb::reducer(init)]
pub fn init(_ctx: &ReducerContext) {
    // Called when the module is initially published
}

#[spacetimedb::reducer(client_connected)]
pub fn identity_connected(_ctx: &ReducerContext) {
    // Called everytime a new client connects
}

#[spacetimedb::reducer(client_disconnected)]
pub fn identity_disconnected(_ctx: &ReducerContext) {
    // Called everytime a client disconnects
}

#[spacetimedb::reducer]
pub fn add(ctx: &ReducerContext, name: String) {
    ctx.db.person().insert(Person { name });
}

#[spacetimedb::reducer]
pub fn say_hello(ctx: &ReducerContext) {
    for person in ctx.db.person().iter() {
        log::info!("Hello, {}!", person.name);
    }
    log::info!("Hello, World!");
}

#[derive(Table, Serialize, Deserialize)]
pub struct User {
    #[primarykey]
    pub user_id: u64,
    #[unique]
    pub username: String,
    #[unique]
    pub email: String,
    pub role: String, //this should be changed to an enum or fk to a different "Role" table
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub profile_picture_url: Option<String>,
}
