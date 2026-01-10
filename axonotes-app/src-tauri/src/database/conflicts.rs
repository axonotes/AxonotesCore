//! # Sync Conflict Storage
//!
//! Stores sync conflicts for user resolution and history archival.
//!
//! ## Tables
//!
//! | Table | Purpose |
//! |-------|---------|
//! | `pending_sync_conflicts` | Active conflicts awaiting UI resolution |
//! | `sync_conflict_history` | Insert-only archive of lost batches |
//!
//! ## Conflict Flow
//!
//! 1. Server batch arrives for block with local pending batches
//! 2. User's block state is reconstructed and saved to `pending_sync_conflicts`
//! 3. Lost batches are archived to `sync_conflict_history` (never deleted)
//! 4. Frontend shows conflict UI for user resolution

use crate::database::helpers::SqlU128;
use crate::encryption::batch::DecryptedBatch;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

// ==========================================
// Types
// ==========================================

/// Active conflict awaiting user resolution.
///
/// Stores both user and server block states as JSON for UI display.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingSyncConflict {
    pub conflict_id: String,
    pub doc_id: String,
    pub block_id: u64,
    /// JSON serialized Block (user's version before server sync)
    pub user_state: String,
    /// JSON serialized Block (server's version)
    pub server_state: String,
    /// When the conflict was detected (milliseconds since epoch)
    pub timestamp: u128,
}

/// Archived conflict history entry (insert-only).
///
/// Preserves lost batches for potential recovery and future branch visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflictHistory {
    pub history_id: String,
    pub doc_id: String,
    pub block_id: u64,
    /// The batches that were lost due to server sync
    pub lost_batches: Vec<DecryptedBatch>,
    /// Timestamp of first lost batch (for ordering)
    pub timestamp: u128,
}

// ==========================================
// Pending Conflicts (for UI resolution)
// ==========================================

/// Save a pending conflict for user resolution.
///
/// Uses INSERT OR REPLACE to update if conflict already exists for same conflict_id.
pub fn save_pending_conflict(conn: &Connection, conflict: &PendingSyncConflict) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO pending_sync_conflicts
         (conflict_id, doc_id, block_id, user_state, server_state, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            conflict.conflict_id,
            conflict.doc_id,
            conflict.block_id as i64,
            conflict.user_state,
            conflict.server_state,
            SqlU128(conflict.timestamp),
        ],
    )?;
    Ok(())
}

/// Get all pending conflicts for a document.
pub fn get_pending_conflicts_by_doc(
    conn: &Connection,
    doc_id: &str,
) -> Result<Vec<PendingSyncConflict>> {
    let mut stmt = conn.prepare(
        "SELECT conflict_id, doc_id, block_id, user_state, server_state, timestamp
         FROM pending_sync_conflicts
         WHERE doc_id = ?1
         ORDER BY timestamp DESC",
    )?;
    let rows = stmt.query_map([doc_id], |row| {
        Ok(PendingSyncConflict {
            conflict_id: row.get(0)?,
            doc_id: row.get(1)?,
            block_id: row.get::<_, i64>(2)? as u64,
            user_state: row.get(3)?,
            server_state: row.get(4)?,
            timestamp: row.get::<_, SqlU128>(5)?.0,
        })
    })?;
    rows.collect()
}

/// Get all pending conflicts across all documents.
pub fn get_all_pending_conflicts(conn: &Connection) -> Result<Vec<PendingSyncConflict>> {
    let mut stmt = conn.prepare(
        "SELECT conflict_id, doc_id, block_id, user_state, server_state, timestamp
         FROM pending_sync_conflicts
         ORDER BY timestamp DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(PendingSyncConflict {
            conflict_id: row.get(0)?,
            doc_id: row.get(1)?,
            block_id: row.get::<_, i64>(2)? as u64,
            user_state: row.get(3)?,
            server_state: row.get(4)?,
            timestamp: row.get::<_, SqlU128>(5)?.0,
        })
    })?;
    rows.collect()
}

/// Get a specific pending conflict by ID.
pub fn get_pending_conflict(
    conn: &Connection,
    conflict_id: &str,
) -> Result<Option<PendingSyncConflict>> {
    let mut stmt = conn.prepare(
        "SELECT conflict_id, doc_id, block_id, user_state, server_state, timestamp
         FROM pending_sync_conflicts
         WHERE conflict_id = ?1",
    )?;
    let mut rows = stmt.query([conflict_id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(PendingSyncConflict {
            conflict_id: row.get(0)?,
            doc_id: row.get(1)?,
            block_id: row.get::<_, i64>(2)? as u64,
            user_state: row.get(3)?,
            server_state: row.get(4)?,
            timestamp: row.get::<_, SqlU128>(5)?.0,
        }))
    } else {
        Ok(None)
    }
}

/// Delete a pending conflict after user resolves it.
pub fn delete_pending_conflict(conn: &Connection, conflict_id: &str) -> Result<()> {
    conn.execute(
        "DELETE FROM pending_sync_conflicts WHERE conflict_id = ?1",
        [conflict_id],
    )?;
    Ok(())
}

/// Delete pending conflict by doc and block (when new conflict supersedes old).
pub fn delete_pending_conflict_by_block(
    conn: &Connection,
    doc_id: &str,
    block_id: u64,
) -> Result<()> {
    conn.execute(
        "DELETE FROM pending_sync_conflicts WHERE doc_id = ?1 AND block_id = ?2",
        params![doc_id, block_id as i64],
    )?;
    Ok(())
}

// ==========================================
// Conflict History (insert-only archive)
// ==========================================

/// Archive lost batches to history.
///
/// This is INSERT ONLY - entries are never updated or deleted.
/// This preserves complete conflict history for recovery and visualization.
pub fn save_conflict_history(conn: &Connection, history: &SyncConflictHistory) -> Result<()> {
    let lost_batches_blob = postcard::to_allocvec(&history.lost_batches)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

    conn.execute(
        "INSERT INTO sync_conflict_history
         (history_id, doc_id, block_id, lost_batches, timestamp)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            history.history_id,
            history.doc_id,
            history.block_id as i64,
            lost_batches_blob,
            SqlU128(history.timestamp),
        ],
    )?;
    Ok(())
}

/// Get conflict history for a specific block.
///
/// Returns entries ordered by timestamp ascending (oldest first).
pub fn get_conflict_history_by_block(
    conn: &Connection,
    doc_id: &str,
    block_id: u64,
) -> Result<Vec<SyncConflictHistory>> {
    let mut stmt = conn.prepare(
        "SELECT history_id, doc_id, block_id, lost_batches, timestamp
         FROM sync_conflict_history
         WHERE doc_id = ?1 AND block_id = ?2
         ORDER BY timestamp ASC",
    )?;
    let rows = stmt.query_map(params![doc_id, block_id as i64], |row| {
        let blob: Vec<u8> = row.get(3)?;
        let lost_batches: Vec<DecryptedBatch> = postcard::from_bytes(&blob).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Blob, Box::new(e))
        })?;
        Ok(SyncConflictHistory {
            history_id: row.get(0)?,
            doc_id: row.get(1)?,
            block_id: row.get::<_, i64>(2)? as u64,
            lost_batches,
            timestamp: row.get::<_, SqlU128>(4)?.0,
        })
    })?;
    rows.collect()
}

/// Get all conflict history for a document.
///
/// Returns entries ordered by timestamp ascending (oldest first).
pub fn get_conflict_history_by_doc(
    conn: &Connection,
    doc_id: &str,
) -> Result<Vec<SyncConflictHistory>> {
    let mut stmt = conn.prepare(
        "SELECT history_id, doc_id, block_id, lost_batches, timestamp
         FROM sync_conflict_history
         WHERE doc_id = ?1
         ORDER BY timestamp ASC",
    )?;
    let rows = stmt.query_map([doc_id], |row| {
        let blob: Vec<u8> = row.get(3)?;
        let lost_batches: Vec<DecryptedBatch> = postcard::from_bytes(&blob).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Blob, Box::new(e))
        })?;
        Ok(SyncConflictHistory {
            history_id: row.get(0)?,
            doc_id: row.get(1)?,
            block_id: row.get::<_, i64>(2)? as u64,
            lost_batches,
            timestamp: row.get::<_, SqlU128>(4)?.0,
        })
    })?;
    rows.collect()
}
