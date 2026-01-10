//! # Database Schema
//!
//! Defines the SQLite database schema for local data persistence.
//!
//! ## Tables
//!
//! | Table | Purpose |
//! |-------|---------|
//! | `profiles` | User login sessions (email, tokens, active flag) |
//! | `keys` | User encryption/signing keys (per identity) |
//! | `batches` | Document edit batches (synced and pending) |
//! | `snapshots` | Point-in-time block snapshots for key rotation |
//! | `blob_cache` | Cached encrypted media blobs metadata |
//! | `workspaces` | UI workspace configurations (Dockview layouts) |
//! | `document_keys` | Document encryption keys (synced from STDB, stored decrypted) |
//! | `permissions` | Document permissions (synced from STDB) |
//! | `documents` | Document metadata (synced from STDB) |
//! | `document_metadata` | Document path/tags (synced from STDB, merge strategy) |
//! | `pending_sync_conflicts` | Active conflicts awaiting UI resolution |
//! | `sync_conflict_history` | Insert-only archive of lost batches |
//!
//! ## Index Strategy
//!
//! Indexes are optimized for common query patterns:
//! - Lookup by `doc_id` for document queries
//! - Lookup by `doc_id + block_id` for block reconstruction
//! - Lookup by `doc_id + timestamp` for time-travel queries
//! - Partial indexes on `is_initial = 1` for efficient snapshot finding
//!
//! ## Timestamps
//!
//! Timestamps are stored as 16-byte big-endian blobs (`BLOB NOT NULL`)
//! using the `SqlU128` helper. This preserves sort order in SQL.

use rusqlite::{Connection, Result};

/// Initialize all database tables and indexes.
///
/// Called once when the database is first opened. Uses `CREATE IF NOT EXISTS`
/// to be idempotent on subsequent runs.
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

    // Create blob_cache table for tracking cached encrypted media blobs
    conn.execute(
        "CREATE TABLE IF NOT EXISTS blob_cache (
            hash TEXT PRIMARY KEY,
            doc_id TEXT NOT NULL,
            size_bytes INTEGER NOT NULL,
            cached_at INTEGER NOT NULL
        )",
        [],
    )?;

    // Index for looking up blobs by document (for deletion on doc delete)
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_blob_cache_doc ON blob_cache(doc_id)",
        [],
    )?;

    // Create workspaces table for storing Dockview UI layouts
    conn.execute(
        "CREATE TABLE IF NOT EXISTS workspaces (
            id TEXT PRIMARY KEY,
            config TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    // ========================================
    // STDB Sync Tables (for offline support)
    // ========================================

    // Document keys table (synced from STDB, stored DECRYPTED)
    // identity_id = which user account this data belongs to (for multi-account support)
    // Keys are decrypted during sync and stored in plaintext (SQLCipher encrypts the entire DB)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS document_keys (
            key_id TEXT PRIMARY KEY,
            identity_id BLOB NOT NULL,
            doc_id TEXT NOT NULL,
            user_id BLOB NOT NULL,
            key_timestamp BLOB NOT NULL,
            encryption_key BLOB NOT NULL,
            signing_private_key BLOB NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_document_keys_identity ON document_keys(identity_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_document_keys_doc_id ON document_keys(identity_id, doc_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_document_keys_doc_ts ON document_keys(identity_id, doc_id, key_timestamp DESC)",
        [],
    )?;

    // Permissions table (synced from STDB)
    // identity_id = which user account this data belongs to
    conn.execute(
        "CREATE TABLE IF NOT EXISTS permissions (
            permission_id TEXT PRIMARY KEY,
            identity_id BLOB NOT NULL,
            doc_id TEXT NOT NULL,
            user_id BLOB NOT NULL,
            role TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_permissions_identity ON permissions(identity_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_permissions_doc_id ON permissions(identity_id, doc_id)",
        [],
    )?;

    // Documents table (synced from STDB)
    // identity_id = which user account this data belongs to
    conn.execute(
        "CREATE TABLE IF NOT EXISTS documents (
            doc_id TEXT NOT NULL,
            identity_id BLOB NOT NULL,
            owner_id BLOB NOT NULL,
            current_public_signing_key BLOB NOT NULL,
            key_timestamp BLOB NOT NULL,
            PRIMARY KEY (doc_id, identity_id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_documents_identity ON documents(identity_id)",
        [],
    )?;

    // Document metadata table (synced from STDB, stored DECRYPTED)
    // identity_id = which user account this data belongs to (for multi-account support)
    // Metadata is decrypted during sync and stored in plaintext (SQLCipher encrypts the entire DB)
    // Uses merge sync strategy: tags are merged (union), path uses server-wins
    conn.execute(
        "CREATE TABLE IF NOT EXISTS document_metadata (
            meta_id TEXT PRIMARY KEY,
            identity_id BLOB NOT NULL,
            doc_id TEXT NOT NULL,
            path TEXT NOT NULL,
            tags TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_document_metadata_identity ON document_metadata(identity_id)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_document_metadata_doc_id ON document_metadata(identity_id, doc_id)",
        [],
    )?;

    // ========================================
    // Sync Conflict Tables
    // ========================================

    // Pending conflicts awaiting user resolution
    // Stores user and server block states for UI conflict resolution
    conn.execute(
        "CREATE TABLE IF NOT EXISTS pending_sync_conflicts (
            conflict_id TEXT PRIMARY KEY,
            doc_id TEXT NOT NULL,
            block_id INTEGER NOT NULL,
            user_state TEXT NOT NULL,
            server_state TEXT NOT NULL,
            timestamp BLOB NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_pending_conflicts_doc_block
         ON pending_sync_conflicts(doc_id, block_id)",
        [],
    )?;

    // Insert-only archive of lost batches (for recovery and branch visualization)
    // Never updated or deleted - preserves complete conflict history
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sync_conflict_history (
            history_id TEXT PRIMARY KEY,
            doc_id TEXT NOT NULL,
            block_id INTEGER NOT NULL,
            lost_batches BLOB NOT NULL,
            timestamp BLOB NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_conflict_history_doc_block
         ON sync_conflict_history(doc_id, block_id)",
        [],
    )?;

    Ok(())
}
