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

/// Saves a single decrypted document key, replacing if it already exists.
pub fn save(conn: &Connection, identity_id: &Identity, key: &DecryptedDocumentKey) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO document_keys
        (key_id, identity_id, doc_id, user_id, key_timestamp, encryption_key, signing_private_key)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            key.key_id,
            identity_id.to_byte_array().as_slice(),
            key.doc_id,
            key.user_id.to_byte_array().as_slice(),
            SqlU128(key.key_timestamp),
            key.key_data.encryption_key,
            key.key_data.signing_private_key,
        ],
    )?;
    Ok(())
}

/// Syncs document keys for a specific identity.
/// Deletes all existing keys for this identity and inserts the new ones.
pub fn sync(
    conn: &Connection,
    identity_id: &Identity,
    keys: &[DecryptedDocumentKey],
) -> Result<()> {
    // Delete all keys for this identity
    conn.execute(
        "DELETE FROM document_keys WHERE identity_id = ?1",
        params![identity_id.to_byte_array().as_slice()],
    )?;

    // Insert all new keys
    for key in keys {
        save(conn, identity_id, key)?;
    }

    Ok(())
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
        "SELECT key_id, doc_id, user_id, key_timestamp, encryption_key, signing_private_key
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

        Ok(DecryptedDocumentKey {
            key_id: row.get(0)?,
            doc_id: row.get(1)?,
            user_id: Identity::from_byte_array(user_id_arr),
            key_timestamp: key_timestamp.0,
            key_data: DecryptedKeyData {
                encryption_key: row.get(4)?,
                signing_private_key: row.get(5)?,
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
        "SELECT key_id, doc_id, user_id, key_timestamp, encryption_key, signing_private_key
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

            Ok(DecryptedDocumentKey {
                key_id: row.get(0)?,
                doc_id: row.get(1)?,
                user_id: Identity::from_byte_array(user_id_arr),
                key_timestamp: key_timestamp.0,
                key_data: DecryptedKeyData {
                    encryption_key: row.get(4)?,
                    signing_private_key: row.get(5)?,
                },
            })
        },
    )?;

    rows.collect()
}
