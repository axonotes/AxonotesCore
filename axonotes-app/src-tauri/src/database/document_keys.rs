//! # Document Keys Storage
//!
//! Handles storage and retrieval of document encryption keys in the local database.
//! Keys are stored DECRYPTED - decryption happens during sync from STDB.
//! The database itself is encrypted with SQLCipher, so this is safe.
//!
//! ## Multi-Account Support
//!
//! All operations are scoped by `identity_id` to support multiple user accounts
//! on the same device.

use super::helpers::SqlU128;
use crate::encryption::document::{DecryptedDocumentKey, DecryptedKeyData};
use rusqlite::{params, Connection, Result};
use spacetimedb_sdk::Identity;
use std::collections::HashSet;

/// Saves a single decrypted document key, replacing if it already exists.
pub fn save(conn: &Connection, identity_id: &Identity, key: &DecryptedDocumentKey) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO document_keys
        (key_id, identity_id, doc_id, user_id, key_timestamp, key_index, encryption_key, signing_private_key)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            key.key_id,
            identity_id.to_byte_array().as_slice(),
            key.doc_id,
            key.user_id.to_byte_array().as_slice(),
            SqlU128(key.key_timestamp),
            key.key_index as i64,
            key.key_data.encryption_key,
            key.key_data.signing_private_key,
        ],
    )?;
    Ok(())
}

/// Gets all key IDs for a specific identity (for sync comparison).
fn get_all_ids_for_identity(conn: &Connection, identity_id: &Identity) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT key_id FROM document_keys WHERE identity_id = ?1")?;
    let rows = stmt.query_map(params![identity_id.to_byte_array().as_slice()], |row| {
        row.get::<_, String>(0)
    })?;
    rows.collect()
}

/// Syncs document keys for a specific identity.
///
/// Strategy to avoid race conditions with real-time callbacks:
/// 1. Collect local key IDs and server key IDs
/// 2. Delete keys that are NOT in server set
/// 3. Upsert all server keys
///
/// This preserves keys inserted by real-time callbacks during sync.
pub fn sync(
    conn: &Connection,
    identity_id: &Identity,
    server_keys: &[DecryptedDocumentKey],
) -> Result<()> {
    // Collect IDs
    let local_ids = get_all_ids_for_identity(conn, identity_id)?;
    let server_ids: HashSet<String> = server_keys.iter().map(|k| k.key_id.clone()).collect();

    // Delete keys that are not on server
    for local_id in &local_ids {
        if !server_ids.contains(local_id) {
            conn.execute(
                "DELETE FROM document_keys WHERE identity_id = ?1 AND key_id = ?2",
                params![identity_id.to_byte_array().as_slice(), local_id],
            )?;
        }
    }

    // Upsert all server keys
    for key in server_keys {
        save(conn, identity_id, key)?;
    }

    Ok(())
}

/// Deletes a specific document key by key_id.
pub fn delete(conn: &Connection, identity_id: &Identity, key_id: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM document_keys WHERE identity_id = ?1 AND key_id = ?2",
        params![identity_id.to_byte_array().as_slice(), key_id],
    )
}

/// Deletes all document keys for a specific identity.
pub fn delete_for_identity(conn: &Connection, identity_id: &Identity) -> Result<usize> {
    conn.execute(
        "DELETE FROM document_keys WHERE identity_id = ?1",
        params![identity_id.to_byte_array().as_slice()],
    )
}

/// Gets all document keys for a specific identity.
pub fn get_all_for_identity(
    conn: &Connection,
    identity_id: &Identity,
) -> Result<Vec<DecryptedDocumentKey>> {
    let mut stmt = conn.prepare(
        "SELECT key_id, doc_id, user_id, key_timestamp, key_index, encryption_key, signing_private_key
         FROM document_keys
         WHERE identity_id = ?1
         ORDER BY key_timestamp DESC",
    )?;

    let rows = stmt.query_map(params![identity_id.to_byte_array().as_slice()], |row| {
        let user_id_bytes: Vec<u8> = row.get(2)?;
        let user_id_arr: [u8; 32] = user_id_bytes.try_into().map_err(|_| {
            rusqlite::Error::InvalidColumnType(2, "user_id".into(), rusqlite::types::Type::Blob)
        })?;

        let key_timestamp: SqlU128 = row.get(3)?;
        let key_index: i64 = row.get(4)?;

        Ok(DecryptedDocumentKey {
            key_id: row.get(0)?,
            doc_id: row.get(1)?,
            user_id: Identity::from_byte_array(user_id_arr),
            key_timestamp: key_timestamp.0,
            key_index: key_index as u32,
            key_data: DecryptedKeyData {
                encryption_key: row.get(5)?,
                signing_private_key: row.get(6)?,
            },
        })
    })?;

    rows.collect()
}

/// Gets document keys for a specific document and identity.
pub fn get_by_doc_id(
    conn: &Connection,
    identity_id: &Identity,
    doc_id: &str,
) -> Result<Vec<DecryptedDocumentKey>> {
    let mut stmt = conn.prepare(
        "SELECT key_id, doc_id, user_id, key_timestamp, key_index, encryption_key, signing_private_key
         FROM document_keys
         WHERE identity_id = ?1 AND doc_id = ?2
         ORDER BY key_timestamp DESC",
    )?;

    let rows = stmt.query_map(
        params![identity_id.to_byte_array().as_slice(), doc_id],
        |row| {
            let user_id_bytes: Vec<u8> = row.get(2)?;
            let user_id_arr: [u8; 32] = user_id_bytes.try_into().map_err(|_| {
                rusqlite::Error::InvalidColumnType(2, "user_id".into(), rusqlite::types::Type::Blob)
            })?;

            let key_timestamp: SqlU128 = row.get(3)?;
            let key_index: i64 = row.get(4)?;

            Ok(DecryptedDocumentKey {
                key_id: row.get(0)?,
                doc_id: row.get(1)?,
                user_id: Identity::from_byte_array(user_id_arr),
                key_timestamp: key_timestamp.0,
                key_index: key_index as u32,
                key_data: DecryptedKeyData {
                    encryption_key: row.get(5)?,
                    signing_private_key: row.get(6)?,
                },
            })
        },
    )?;

    rows.collect()
}
