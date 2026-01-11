//! # Version Tags Storage
//!
//! Handles storage and retrieval of document version tags in the local database.
//! Synced from STDB for offline access.
//!
//! ## Sync Strategy
//!
//! - **Pending tags** (created offline): Preserved during sync, uploaded when online
//! - **Server tags**: Upserted with pending=0, old non-pending tags deleted
//!
//! This approach:
//! 1. Preserves offline-created tags (pending=1)
//! 2. Avoids race conditions with real-time callbacks
//!
//! ## Multi-Account Support
//!
//! All operations are scoped by `identity_id` to support multiple user accounts
//! on the same device.

use super::helpers::SqlU128;
use crate::encryption::version_tag::DecryptedVersionTag;
use rusqlite::{params, Connection, Result};
use spacetimedb_sdk::Identity;
use std::collections::HashSet;

/// Saves a single decrypted version tag from server (pending=0).
pub fn save(conn: &Connection, identity_id: &Identity, tag: &DecryptedVersionTag) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO version_tags
        (tag_id, identity_id, doc_id, tag_name, timestamp, created_by, created_at, pending)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
        params![
            tag.tag_id,
            identity_id.to_byte_array().as_slice(),
            tag.doc_id,
            tag.data.tag_name,
            SqlU128(tag.data.timestamp),
            tag.data.created_by.to_byte_array().as_slice(),
            SqlU128(tag.data.created_at),
        ],
    )?;
    Ok(())
}

/// Saves a version tag created offline (pending=1).
/// Will be uploaded to server when connection is available.
pub fn save_pending(
    conn: &Connection,
    identity_id: &Identity,
    tag: &DecryptedVersionTag,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO version_tags
        (tag_id, identity_id, doc_id, tag_name, timestamp, created_by, created_at, pending)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1)",
        params![
            tag.tag_id,
            identity_id.to_byte_array().as_slice(),
            tag.doc_id,
            tag.data.tag_name,
            SqlU128(tag.data.timestamp),
            tag.data.created_by.to_byte_array().as_slice(),
            SqlU128(tag.data.created_at),
        ],
    )?;
    Ok(())
}

/// Gets all tag IDs for a specific identity (for sync comparison).
fn get_all_ids_for_identity(conn: &Connection, identity_id: &Identity) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT tag_id FROM version_tags WHERE identity_id = ?1")?;
    let rows = stmt.query_map(params![identity_id.to_byte_array().as_slice()], |row| {
        row.get::<_, String>(0)
    })?;
    rows.collect()
}

/// Syncs version tags for a specific identity.
///
/// Strategy to avoid race conditions and preserve offline data:
/// 1. Collect local tag IDs and server tag IDs
/// 2. Delete non-pending tags that are NOT in server set
/// 3. Upsert all server tags (with pending=0)
///
/// This preserves:
/// - Pending tags (created offline, not yet uploaded)
/// - Tags inserted by real-time callbacks during sync
pub fn sync(
    conn: &Connection,
    identity_id: &Identity,
    server_tags: &[DecryptedVersionTag],
) -> Result<()> {
    // Collect IDs
    let local_ids = get_all_ids_for_identity(conn, identity_id)?;
    let server_ids: HashSet<String> = server_tags.iter().map(|t| t.tag_id.clone()).collect();

    // Delete non-pending tags that are not on server
    for local_id in &local_ids {
        if !server_ids.contains(local_id) {
            // Only delete if not pending (preserve offline-created tags)
            conn.execute(
                "DELETE FROM version_tags WHERE identity_id = ?1 AND tag_id = ?2 AND pending = 0",
                params![identity_id.to_byte_array().as_slice(), local_id],
            )?;
        }
    }

    // Upsert all server tags
    for tag in server_tags {
        save(conn, identity_id, tag)?;
    }

    Ok(())
}

/// Deletes a specific version tag by tag_id.
pub fn delete(conn: &Connection, identity_id: &Identity, tag_id: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM version_tags WHERE identity_id = ?1 AND tag_id = ?2",
        params![identity_id.to_byte_array().as_slice(), tag_id],
    )
}

/// Deletes all version tags for a specific identity.
pub fn delete_for_identity(conn: &Connection, identity_id: &Identity) -> Result<usize> {
    conn.execute(
        "DELETE FROM version_tags WHERE identity_id = ?1",
        params![identity_id.to_byte_array().as_slice()],
    )
}

/// Gets all version tags for a specific identity.
pub fn get_all_for_identity(
    conn: &Connection,
    identity_id: &Identity,
) -> Result<Vec<DecryptedVersionTag>> {
    let mut stmt = conn.prepare(
        "SELECT tag_id, doc_id, tag_name, timestamp, created_by, created_at
         FROM version_tags
         WHERE identity_id = ?1
         ORDER BY timestamp DESC",
    )?;

    let rows = stmt.query_map(params![identity_id.to_byte_array().as_slice()], |row| {
        let created_by_bytes: Vec<u8> = row.get(4)?;
        let created_by_arr: [u8; 32] = created_by_bytes.try_into().map_err(|_| {
            rusqlite::Error::InvalidColumnType(4, "created_by".into(), rusqlite::types::Type::Blob)
        })?;

        let timestamp: SqlU128 = row.get(3)?;
        let created_at: SqlU128 = row.get(5)?;

        Ok(DecryptedVersionTag {
            tag_id: row.get(0)?,
            doc_id: row.get(1)?,
            data: crate::encryption::version_tag::DecryptedVersionTagData {
                tag_name: row.get(2)?,
                timestamp: timestamp.0,
                created_by: Identity::from_byte_array(created_by_arr),
                created_at: created_at.0,
            },
        })
    })?;

    rows.collect()
}

/// Gets version tags for a specific document and identity.
pub fn get_by_doc_id(
    conn: &Connection,
    identity_id: &Identity,
    doc_id: &str,
) -> Result<Vec<DecryptedVersionTag>> {
    let mut stmt = conn.prepare(
        "SELECT tag_id, doc_id, tag_name, timestamp, created_by, created_at
         FROM version_tags
         WHERE identity_id = ?1 AND doc_id = ?2
         ORDER BY timestamp DESC",
    )?;

    let rows = stmt.query_map(
        params![identity_id.to_byte_array().as_slice(), doc_id],
        |row| {
            let created_by_bytes: Vec<u8> = row.get(4)?;
            let created_by_arr: [u8; 32] = created_by_bytes.try_into().map_err(|_| {
                rusqlite::Error::InvalidColumnType(
                    4,
                    "created_by".into(),
                    rusqlite::types::Type::Blob,
                )
            })?;

            let timestamp: SqlU128 = row.get(3)?;
            let created_at: SqlU128 = row.get(5)?;

            Ok(DecryptedVersionTag {
                tag_id: row.get(0)?,
                doc_id: row.get(1)?,
                data: crate::encryption::version_tag::DecryptedVersionTagData {
                    tag_name: row.get(2)?,
                    timestamp: timestamp.0,
                    created_by: Identity::from_byte_array(created_by_arr),
                    created_at: created_at.0,
                },
            })
        },
    )?;

    rows.collect()
}
