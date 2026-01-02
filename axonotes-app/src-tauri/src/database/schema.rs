use rusqlite::{Connection, Result};

/// Initialize all database tables and indexes
pub fn init_schema(conn: &Connection) -> Result<()> {
    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Create profiles table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS profiles (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL,
            name TEXT NOT NULL,
            access_token TEXT NOT NULL,
            refresh_token TEXT,
            is_active INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    // Create index for active profile lookup
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_profiles_active ON profiles(is_active) WHERE is_active = 1",
        [],
    )?;

    // Create index for email lookup
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_profiles_email ON profiles(email)",
        [],
    )?;

    Ok(())
}
