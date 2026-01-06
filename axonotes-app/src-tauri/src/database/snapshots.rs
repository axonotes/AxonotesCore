use crate::database::helpers::SqlU128;
use crate::encryption::batch::DecryptedBatch;
use postcard::to_allocvec;
use rusqlite::{params, Connection, Error, Result};

/// Save a snapshot (always stored as is_initial = 1)
/// The batch should contain the full serialized state in patches, not deltas.
pub fn save(conn: &Connection, batch: &DecryptedBatch) -> Result<()> {
    let patches_blob = to_allocvec(&batch.batch_data.patches)
        .map_err(|e| Error::ToSqlConversionFailure(Box::new(e)))?;

    conn.execute(
        "INSERT OR REPLACE INTO snapshots (batch_id, doc_id, timestamp, block_id, patches, pending, is_initial)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, 1)",
        params![
            &batch.batch_id,
            &batch.doc_id,
            SqlU128(batch.timestamp),
            batch.batch_data.block_id as i64,
            patches_blob,
        ],
    )?;

    Ok(())
}

/// Delete all snapshots for a document
pub fn delete_by_doc_id(conn: &Connection, doc_id: &str) -> Result<usize> {
    conn.execute("DELETE FROM snapshots WHERE doc_id = ?1", params![doc_id])
}

/// Delete a specific snapshot
pub fn delete(conn: &Connection, batch_id: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM snapshots WHERE batch_id = ?1",
        params![batch_id],
    )
}

/// Count snapshots for a document (useful for testing)
pub fn count_by_doc(conn: &Connection, doc_id: &str) -> Result<u64> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM snapshots WHERE doc_id = ?1",
        params![doc_id],
        |row| row.get(0),
    )?;
    Ok(count as u64)
}
