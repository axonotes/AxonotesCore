//! # Documents Storage
//!
//! Handles storage and retrieval of document metadata in the local database.
//! Synced from STDB for offline access.
//!
//! ## Multi-Account Support
//!
//! All operations are scoped by `identity_id` to support multiple user accounts
//! on the same device.

use super::helpers::SqlU128;
use crate::stdb_bindings::Document;
use rusqlite::{params, Connection, Result};
use spacetimedb_sdk::Identity;
use std::collections::HashSet;

/// Saves a single document, replacing if it already exists.
pub fn save(conn: &Connection, identity_id: &Identity, document: &Document) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO documents
        (doc_id, identity_id, owner_id, current_public_signing_key, key_timestamp)
        VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            document.doc_id,
            identity_id.to_byte_array().as_slice(),
            document.owner_id.to_byte_array().as_slice(),
            document.current_public_signing_key,
            SqlU128(document.key_timestamp),
        ],
    )?;
    Ok(())
}

/// Gets all document IDs for a specific identity (for sync comparison).
fn get_all_ids_for_identity(conn: &Connection, identity_id: &Identity) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT doc_id FROM documents WHERE identity_id = ?1")?;
    let rows = stmt.query_map(params![identity_id.to_byte_array().as_slice()], |row| {
        row.get::<_, String>(0)
    })?;
    rows.collect()
}

/// Syncs documents for a specific identity.
///
/// Strategy to avoid race conditions with real-time callbacks:
/// 1. Collect local document IDs and server document IDs
/// 2. Delete documents that are NOT in server set
/// 3. Upsert all server documents
///
/// This preserves documents inserted by real-time callbacks during sync.
pub fn sync(
    conn: &Connection,
    identity_id: &Identity,
    server_documents: &[Document],
) -> Result<()> {
    // Collect IDs
    let local_ids = get_all_ids_for_identity(conn, identity_id)?;
    let server_ids: HashSet<String> = server_documents.iter().map(|d| d.doc_id.clone()).collect();

    // Delete documents that are not on server
    for local_id in &local_ids {
        if !server_ids.contains(local_id) {
            conn.execute(
                "DELETE FROM documents WHERE identity_id = ?1 AND doc_id = ?2",
                params![identity_id.to_byte_array().as_slice(), local_id],
            )?;
        }
    }

    // Upsert all server documents
    for document in server_documents {
        save(conn, identity_id, document)?;
    }

    Ok(())
}

/// Deletes a specific document by doc_id.
pub fn delete(conn: &Connection, identity_id: &Identity, doc_id: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM documents WHERE identity_id = ?1 AND doc_id = ?2",
        params![identity_id.to_byte_array().as_slice(), doc_id],
    )
}

/// Deletes all documents for a specific identity.
pub fn delete_for_identity(conn: &Connection, identity_id: &Identity) -> Result<usize> {
    conn.execute(
        "DELETE FROM documents WHERE identity_id = ?1",
        params![identity_id.to_byte_array().as_slice()],
    )
}

/// Gets all documents for a specific identity.
pub fn get_all_for_identity(conn: &Connection, identity_id: &Identity) -> Result<Vec<Document>> {
    let mut stmt = conn.prepare(
        "SELECT doc_id, owner_id, current_public_signing_key, key_timestamp
         FROM documents
         WHERE identity_id = ?1",
    )?;

    let rows = stmt.query_map(params![identity_id.to_byte_array().as_slice()], |row| {
        let owner_id_bytes: Vec<u8> = row.get(1)?;
        let owner_id_arr: [u8; 32] = owner_id_bytes.try_into().map_err(|_| {
            rusqlite::Error::InvalidColumnType(1, "owner_id".into(), rusqlite::types::Type::Blob)
        })?;

        let key_timestamp: SqlU128 = row.get(3)?;

        Ok(Document {
            doc_id: row.get(0)?,
            owner_id: Identity::from_byte_array(owner_id_arr),
            current_public_signing_key: row.get(2)?,
            key_timestamp: key_timestamp.0,
        })
    })?;

    rows.collect()
}

/// Gets a specific document by doc_id and identity.
pub fn get_by_id(
    conn: &Connection,
    identity_id: &Identity,
    doc_id: &str,
) -> Result<Option<Document>> {
    let mut stmt = conn.prepare(
        "SELECT doc_id, owner_id, current_public_signing_key, key_timestamp
         FROM documents
         WHERE identity_id = ?1 AND doc_id = ?2",
    )?;

    let mut rows = stmt.query(params![identity_id.to_byte_array().as_slice(), doc_id])?;

    if let Some(row) = rows.next()? {
        let owner_id_bytes: Vec<u8> = row.get(1)?;
        let owner_id_arr: [u8; 32] = owner_id_bytes.try_into().map_err(|_| {
            rusqlite::Error::InvalidColumnType(1, "owner_id".into(), rusqlite::types::Type::Blob)
        })?;

        let key_timestamp: SqlU128 = row.get(3)?;

        Ok(Some(Document {
            doc_id: row.get(0)?,
            owner_id: Identity::from_byte_array(owner_id_arr),
            current_public_signing_key: row.get(2)?,
            key_timestamp: key_timestamp.0,
        }))
    } else {
        Ok(None)
    }
}
