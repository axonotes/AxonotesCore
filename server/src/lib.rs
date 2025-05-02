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
    pub created_at: u64,
    pub updated_at: u64,
    pub profile_picture_url: Option<String>,
}

#[derive(Table, Serialize, Deserialize)]
pub struct Course {
    #[primarykey]
    pub course_id: u64,
    pub title: String,
    pub description: String,
    #[foreignkey(User)]
    pub instructor_id: u64,
    pub created_at: u64,
    pub updated_at: u64,
    pub status: String,
}

#[derive(Table, Serialize, Deserialize)]
pub struct Module {
    #[primarykey]
    pub module_id: u64,
    #[foreignkey(Course)]
    pub course_id: u64,
    pub title: String,
    pub description: String,
    pub order: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Table, Serialize, Deserialize)]
pub struct Lesson {
    #[primarykey]
    pub lesson_id: u64,
    #[foreignkey(Module)]
    pub module_id: u64,
    pub title: String,
    pub content: String,
    pub lesson_type: String, //text/video/quiz/assignment - maybe change to Enum
    pub order: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Table, Serialize, Deserialize)]
pub struct Enrollment {
    #[primarykey]
    pub enrollment_id: u64,
    #[foreignkey(User)]
    pub user_id: u64,
    #[foreignkey(Course)]
    pub course_id: u64,
    pub enrolled_at: u64,
    pub completion_status: String, // not_started/in_progress/completed
    pub completed_at: Option<u64>,
}

#[derive(Table, Serialize, Deserialize)]
pub struct UserProgress {
    #[primarykey]
    pub user_progress_id: u64,
    #[foreignkey(User)]
    pub user_id: u64,
    #[foreignkey(Lesson)]
    pub lesson_id: u64,
    pub status: String, // not_started/started/completed
    pub started_at: u64,
    pub completed_at: Option<u64>,
}
