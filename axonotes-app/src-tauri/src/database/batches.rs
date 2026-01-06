#![allow(dead_code)]

use crate::database::helpers::SqlU128;
use crate::encryption::batch::{BatchData, DecryptedBatch, Patch};
use postcard::{from_bytes, to_allocvec};
use rusqlite::{params, Connection, Error, Result};

/// Save a synced batch (from server)
pub fn save(conn: &Connection, batch: &DecryptedBatch) -> Result<()> {
    save_internal(conn, batch, false)
}

/// Save a pending batch (local, not yet synced)
pub fn save_pending(conn: &Connection, batch: &DecryptedBatch) -> Result<()> {
    save_internal(conn, batch, true)
}

fn save_internal(conn: &Connection, batch: &DecryptedBatch, pending: bool) -> Result<()> {
    let patches_blob = to_allocvec(&batch.batch_data.patches)
        .map_err(|e| Error::ToSqlConversionFailure(Box::new(e)))?;

    conn.execute(
        "INSERT OR REPLACE INTO batches (batch_id, doc_id, timestamp, block_id, patches, pending, is_initial)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            &batch.batch_id,
            &batch.doc_id,
            SqlU128(batch.timestamp),
            batch.batch_data.block_id as i64,
            patches_blob,
            i32::from(pending),
            i32::from(batch.is_initial),
        ],
    )?;

    Ok(())
}

/// Save multiple synced batches in a transaction
pub fn save_all(conn: &Connection, batches: &[DecryptedBatch]) -> Result<()> {
    let tx = conn.unchecked_transaction()?;

    for batch in batches {
        save(&tx, batch)?;
    }

    tx.commit()
}

/// Get all batches for a document (both pending and synced)
pub fn get_by_doc_id(conn: &Connection, doc_id: &str) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
         FROM batches
         WHERE doc_id = ?1
         ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(params![doc_id], row_to_batch)?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get all batches for a specific block within a document
pub fn get_by_doc_and_block(
    conn: &Connection,
    doc_id: &str,
    block_id: u64,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
         FROM batches
         WHERE doc_id = ?1 AND block_id = ?2
         ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(params![doc_id, block_id as i64], row_to_batch)?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get batches up to (and including) a given timestamp
pub fn get_up_to_timestamp(
    conn: &Connection,
    doc_id: &str,
    up_to: u128,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
         FROM batches
         WHERE doc_id = ?1 AND timestamp <= ?2
         ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(params![doc_id, SqlU128(up_to)], row_to_batch)?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get batches for a specific block up to (and including) a given timestamp
pub fn get_by_doc_and_block_up_to_timestamp(
    conn: &Connection,
    doc_id: &str,
    block_id: u64,
    up_to: u128,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
         FROM batches
         WHERE doc_id = ?1 AND block_id = ?2 AND timestamp <= ?3
         ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(
            params![doc_id, block_id as i64, SqlU128(up_to)],
            row_to_batch,
        )?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get the timestamp of the latest initial batch or snapshot for a block up to a given timestamp.
/// Returns None if no initial exists.
pub fn get_latest_initial_timestamp(
    conn: &Connection,
    doc_id: &str,
    block_id: u64,
    up_to: u128,
) -> Result<Option<u128>> {
    // MAX() returns NULL if no rows match, so we need to handle Option<SqlU128>
    let result: Option<SqlU128> = conn.query_row(
        "SELECT MAX(ts) FROM (
            SELECT timestamp as ts FROM batches
            WHERE doc_id = ?1 AND block_id = ?2 AND is_initial = 1 AND timestamp <= ?3
            UNION ALL
            SELECT timestamp as ts FROM snapshots
            WHERE doc_id = ?1 AND block_id = ?2 AND timestamp <= ?3
        )",
        params![doc_id, block_id as i64, SqlU128(up_to)],
        |row| row.get(0),
    )?;

    Ok(result.map(|s| s.0))
}

/// Get batches from the most recent initial batch OR snapshot up to a given timestamp.
/// Returns batches across ALL blocks in the document.
/// If a snapshot exists at the starting timestamp, uses the snapshot instead of the batch.
pub fn get_from_latest_initial_up_to_timestamp(
    conn: &Connection,
    doc_id: &str,
    up_to: u128,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "WITH latest_initial AS (
            SELECT MAX(ts) as ts FROM (
                SELECT timestamp as ts FROM batches
                WHERE doc_id = ?1 AND is_initial = 1 AND timestamp <= ?2
                UNION ALL
                SELECT timestamp as ts FROM snapshots
                WHERE doc_id = ?1 AND timestamp <= ?2
            )
        ),
        start_ts AS (
            SELECT COALESCE((SELECT ts FROM latest_initial), 0) as ts
        )
        SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
        FROM (
            -- Snapshots at start_ts (preferred)
            SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
            FROM snapshots
            WHERE doc_id = ?1 AND timestamp = (SELECT ts FROM start_ts)

            UNION ALL

            -- Batches at start_ts that DON'T have a snapshot at same position
            SELECT b.batch_id, b.doc_id, b.timestamp, b.block_id, b.patches, b.pending, b.is_initial
            FROM batches b
            WHERE b.doc_id = ?1
              AND b.timestamp = (SELECT ts FROM start_ts)
              AND NOT EXISTS (
                  SELECT 1 FROM snapshots s 
                  WHERE s.doc_id = b.doc_id 
                    AND s.block_id = b.block_id 
                    AND s.timestamp = b.timestamp
              )

            UNION ALL

            -- All batches after starting point
            SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
            FROM batches
            WHERE doc_id = ?1
              AND timestamp > (SELECT ts FROM start_ts)
              AND timestamp <= ?2
        )
        ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(params![doc_id, SqlU128(up_to)], row_to_batch)?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get batches for a specific block from its most recent initial batch OR snapshot up to a given timestamp.
/// If a snapshot exists at the starting timestamp, uses the snapshot instead of the batch.
pub fn get_block_from_latest_initial_up_to_timestamp(
    conn: &Connection,
    doc_id: &str,
    block_id: u64,
    up_to: u128,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "WITH latest_initial AS (
            SELECT MAX(ts) as ts FROM (
                SELECT timestamp as ts FROM batches
                WHERE doc_id = ?1 AND block_id = ?2 AND is_initial = 1 AND timestamp <= ?3
                UNION ALL
                SELECT timestamp as ts FROM snapshots
                WHERE doc_id = ?1 AND block_id = ?2 AND timestamp <= ?3
            )
        ),
        start_ts AS (
            SELECT COALESCE((SELECT ts FROM latest_initial), 0) as ts
        )
        SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
        FROM (
            -- Snapshot at start_ts (preferred)
            SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
            FROM snapshots
            WHERE doc_id = ?1 AND block_id = ?2 AND timestamp = (SELECT ts FROM start_ts)

            UNION ALL

            -- Batch at start_ts if no snapshot exists at same position
            SELECT b.batch_id, b.doc_id, b.timestamp, b.block_id, b.patches, b.pending, b.is_initial
            FROM batches b
            WHERE b.doc_id = ?1 AND b.block_id = ?2
              AND b.timestamp = (SELECT ts FROM start_ts)
              AND NOT EXISTS (
                  SELECT 1 FROM snapshots s 
                  WHERE s.doc_id = b.doc_id 
                    AND s.block_id = b.block_id 
                    AND s.timestamp = b.timestamp
              )

            UNION ALL

            -- All batches after starting point
            SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
            FROM batches
            WHERE doc_id = ?1 AND block_id = ?2
              AND timestamp > (SELECT ts FROM start_ts)
              AND timestamp <= ?3
        )
        ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(
            params![doc_id, block_id as i64, SqlU128(up_to)],
            row_to_batch,
        )?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get batches AND snapshots for a block from a starting timestamp up to target timestamp.
/// Used for incremental reconstruction from a cached state.
///
/// This query:
/// 1. Finds the batch containing `from_ts`
/// 2. Returns all batches/snapshots from there to `to_ts`
/// 3. Prefers snapshots over batches at the same (doc, block, timestamp)
pub fn get_block_batches_in_range(
    conn: &Connection,
    doc_id: &str,
    block_id: u64,
    from_ts: u128,
    to_ts: u128,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "WITH containing_batch AS (
            -- Find the batch that contains our cached timestamp
            SELECT MAX(timestamp) as ts
            FROM batches
            WHERE doc_id = ?1 AND block_id = ?2 AND timestamp <= ?3
        ),
        start_ts AS (
            SELECT COALESCE((SELECT ts FROM containing_batch), 0) as ts
        )
        SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
        FROM (
            -- Snapshots in range (preferred over batches)
            SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
            FROM snapshots
            WHERE doc_id = ?1 AND block_id = ?2
              AND timestamp >= (SELECT ts FROM start_ts)
              AND timestamp <= ?4

            UNION ALL

            -- Batches in range that don't have a snapshot at same position
            SELECT b.batch_id, b.doc_id, b.timestamp, b.block_id, b.patches, b.pending, b.is_initial
            FROM batches b
            WHERE b.doc_id = ?1 AND b.block_id = ?2
              AND b.timestamp >= (SELECT ts FROM start_ts)
              AND b.timestamp <= ?4
              AND NOT EXISTS (
                  SELECT 1 FROM snapshots s 
                  WHERE s.doc_id = b.doc_id 
                    AND s.block_id = b.block_id 
                    AND s.timestamp = b.timestamp
              )
        )
        ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(
            params![doc_id, block_id as i64, SqlU128(from_ts), SqlU128(to_ts)],
            row_to_batch,
        )?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get pending batches for a specific block (for conflict detection)
pub fn get_pending_by_doc_and_block(
    conn: &Connection,
    doc_id: &str,
    block_id: u64,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
         FROM batches
         WHERE doc_id = ?1 AND block_id = ?2 AND pending = 1
         ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(params![doc_id, block_id as i64], row_to_batch)?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get all pending batches for a document
pub fn get_pending_by_doc(conn: &Connection, doc_id: &str) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
         FROM batches
         WHERE doc_id = ?1 AND pending = 1
         ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(params![doc_id], row_to_batch)?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get all pending batches (for upload queue)
pub fn get_all_pending(conn: &Connection) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
         FROM batches
         WHERE pending = 1
         ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map([], row_to_batch)?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Mark a pending batch as synced
pub fn mark_synced(conn: &Connection, batch_id: &str) -> Result<usize> {
    conn.execute(
        "UPDATE batches SET pending = 0 WHERE batch_id = ?1",
        params![batch_id],
    )
}

/// Delete all batches for a document
pub fn delete_by_doc_id(conn: &Connection, doc_id: &str) -> Result<usize> {
    conn.execute("DELETE FROM batches WHERE doc_id = ?1", params![doc_id])
}

/// Delete a specific batch
pub fn delete(conn: &Connection, batch_id: &str) -> Result<usize> {
    conn.execute("DELETE FROM batches WHERE batch_id = ?1", params![batch_id])
}

/// Count batches for a document
pub fn count_by_doc(conn: &Connection, doc_id: &str) -> Result<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM batches WHERE doc_id = ?1",
        params![doc_id],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}

/// Count pending batches (useful for sync status UI)
pub fn count_pending(conn: &Connection) -> Result<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM batches WHERE pending = 1",
        [],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}

// ========================================
// Internal Helpers
// ========================================

fn row_to_batch(row: &rusqlite::Row) -> Result<DecryptedBatch> {
    let patches_blob: Vec<u8> = row.get(4)?;
    let patches: Vec<Patch> = from_bytes(patches_blob.as_slice()).map_err(|e| {
        Error::FromSqlConversionFailure(4, rusqlite::types::Type::Blob, Box::new(e))
    })?;

    Ok(DecryptedBatch {
        batch_id: row.get(0)?,
        doc_id: row.get(1)?,
        timestamp: row.get::<_, SqlU128>(2)?.0,
        batch_data: BatchData {
            block_id: row.get::<_, i64>(3)? as u64,
            patches,
        },
        is_initial: row.get::<_, i32>(6)? != 0,
    })
}
