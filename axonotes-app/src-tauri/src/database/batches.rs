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
            pending as i32,
            batch.is_initial as i32,
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
    after: u128,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
         FROM batches
         WHERE doc_id = ?1 AND timestamp <= ?2
         ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(params![doc_id, SqlU128(after)], row_to_batch)?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get batches for a specific block up to (and including) a given timestamp
pub fn get_by_doc_and_block_up_to_timestamp(
    conn: &Connection,
    doc_id: &str,
    block_id: u64,
    after: u128,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
         FROM batches
         WHERE doc_id = ?1 AND block_id = ?2 AND timestamp <= ?3
         ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(
            params![doc_id, block_id as i64, SqlU128(after)],
            row_to_batch,
        )?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get batches from the most recent initial batch up to (and including) a given timestamp.
/// Returns batches across ALL blocks in the document.
///
/// Example: For batches [t0(initial), t100, t200(initial), t250, t300] with target t270:
/// Returns [t200(initial), t250]
pub fn get_from_latest_initial_up_to_timestamp(
    conn: &Connection,
    doc_id: &str,
    up_to: u128,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "WITH latest_initial AS (
            SELECT timestamp
            FROM batches
            WHERE doc_id = ?1 AND is_initial = 1 AND timestamp <= ?2
            ORDER BY timestamp DESC
            LIMIT 1
        )
        SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
        FROM batches
        WHERE doc_id = ?1
          AND timestamp >= COALESCE((SELECT timestamp FROM latest_initial), 0)
          AND timestamp <= ?2
        ORDER BY timestamp ASC",
    )?;

    let batches = stmt
        .query_map(params![doc_id, SqlU128(up_to)], row_to_batch)?
        .collect::<Result<Vec<_>>>()?;

    Ok(batches)
}

/// Get batches for a specific block from its most recent initial batch up to a given timestamp.
/// Only considers initial batches for the specified block.
///
/// Example: For block 5 with batches [t0(initial), t100, t200(initial), t250, t300] and target t270:
/// Returns [t200(initial), t250]
pub fn get_block_from_latest_initial_up_to_timestamp(
    conn: &Connection,
    doc_id: &str,
    block_id: u64,
    up_to: u128,
) -> Result<Vec<DecryptedBatch>> {
    let mut stmt = conn.prepare(
        "WITH latest_initial AS (
            SELECT timestamp
            FROM batches
            WHERE doc_id = ?1 AND block_id = ?2 AND is_initial = 1 AND timestamp <= ?3
            ORDER BY timestamp DESC
            LIMIT 1
        )
        SELECT batch_id, doc_id, timestamp, block_id, patches, pending, is_initial
        FROM batches
        WHERE doc_id = ?1
          AND block_id = ?2
          AND timestamp >= COALESCE((SELECT timestamp FROM latest_initial), 0)
          AND timestamp <= ?3
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
