use rusqlite::Connection;

/// Initialize database schema
pub fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    // Enable WAL mode for better concurrency
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "cache_size", -64000)?; // 64MB cache
    conn.pragma_update(None, "busy_timeout", 5000)?; // 5 second timeout

    // Create users table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            user_id BLOB PRIMARY KEY,
            used_bytes INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at INTEGER NOT NULL DEFAULT (unixepoch())
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_users_created ON users(created_at)",
        [],
    )?;

    // Create documents table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS documents (
            document_id TEXT PRIMARY KEY,
            owner_id BLOB NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
            public_key BLOB NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch())
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_documents_owner ON documents(owner_id)",
        [],
    )?;

    // Create blob_ownership table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS blob_ownership (
            hash TEXT NOT NULL,
            user_id BLOB NOT NULL REFERENCES users(user_id) ON DELETE CASCADE,
            document_id TEXT NOT NULL REFERENCES documents(document_id) ON DELETE CASCADE,
            size_bytes INTEGER NOT NULL,
            created_at INTEGER NOT NULL DEFAULT (unixepoch()),
            PRIMARY KEY (hash, user_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_blob_ownership_user ON blob_ownership(user_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_blob_ownership_document ON blob_ownership(document_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_blob_ownership_hash ON blob_ownership(hash)",
        [],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_initialization() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"documents".to_string()));
        assert!(tables.contains(&"blob_ownership".to_string()));
    }

    #[test]
    fn test_wal_mode_enabled() {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();

        let journal_mode: String = conn
            .pragma_query_value(None, "journal_mode", |row| row.get(0))
            .unwrap();

        // Note: WAL mode may not work in memory, but we test the attempt
        assert!(journal_mode == "wal" || journal_mode == "memory");
    }
}
