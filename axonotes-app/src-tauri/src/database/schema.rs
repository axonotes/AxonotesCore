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
            updated_at INTEGER NOT NULL,
            last_batch_sync BLOB
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

    // Create keys table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS keys (
            user_id TEXT PRIMARY KEY,
            public_encryption_key BLOB,
            private_encryption_key BLOB,
            public_signing_key BLOB,
            private_signing_key BLOB
        )",
        [],
    )?;

    // Create batch table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS batches (
            batch_id TEXT PRIMARY KEY,
            doc_id TEXT NOT NULL,
            timestamp BLOB NOT NULL,
            block_id INTEGER NOT NULL,
            patches BLOB NOT NULL,
            pending INTEGER NOT NULL DEFAULT 0,
            is_initial INTEGER NOT NULL DEFAULT 0
        )",
        [],
    )?;

    // Create index for doc_id lookup
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_doc_id ON batches(doc_id)",
        [],
    )?;

    // Create index for block on doc_id lookup
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_batches_doc_block ON batches(doc_id, block_id)",
        [],
    )?;

    // Create index for block on doc_id lookup after timestamp
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_batches_doc_block_ts ON batches(doc_id, block_id, timestamp)",
        [],
    )?;

    // Create index for pending blocks on doc_id
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_batches_doc_block_pending ON batches(doc_id, block_id, pending)",
        [],
    )?;

    // Create index for finding the latest initial batch of a doc efficiently
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_batches_doc_initial_ts ON batches(doc_id, is_initial, timestamp DESC) WHERE is_initial = 1",
        [],
    )?;

    // Create index for finding the latest initial batch of a block efficiently
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_batches_doc_block_initial_ts ON batches(doc_id, block_id, timestamp DESC) WHERE is_initial = 1",
        [],
    )?;

    // Create snapshots table (same structure as batches, all entries are is_initial=1)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS snapshots (
        batch_id TEXT PRIMARY KEY,
        doc_id TEXT NOT NULL,
        timestamp BLOB NOT NULL,
        block_id INTEGER NOT NULL,
        patches BLOB NOT NULL,
        pending INTEGER NOT NULL DEFAULT 0,
        is_initial INTEGER NOT NULL DEFAULT 1
    )",
        [],
    )?;

    // Index for block-specific snapshot lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_snapshots_doc_block_ts ON snapshots(doc_id, block_id, timestamp DESC)",
        [],
    )?;

    // Index for doc-wide snapshot lookups
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_snapshots_doc_ts ON snapshots(doc_id, timestamp DESC)",
        [],
    )?;

    Ok(())
}
