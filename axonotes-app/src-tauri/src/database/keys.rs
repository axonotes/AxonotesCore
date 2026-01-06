//! # User Key Storage
//!
//! Handles storage and retrieval of user cryptographic keys in the local database.
//!
//! ## Security Note
//!
//! Private keys stored here are already encrypted with the user's password-derived key.
//! The database itself is also encrypted with SQLCipher.

#![allow(dead_code)]
#![allow(clippy::needless_pass_by_value)]

use rusqlite::{params, Connection, Result};

/// User's cryptographic key bundle.
///
/// Contains both X25519 (encryption) and Ed25519 (signing) keypairs.
/// Private keys are encrypted with the user's password-derived key before storage.
#[derive(Clone)]
pub struct Keys {
    /// User identifier this key bundle belongs to
    pub user_id: String,
    /// 32-byte X25519 public key for receiving encrypted data
    pub public_encryption_key: Vec<u8>,
    /// Encrypted X25519 private key
    pub private_encryption_key: Vec<u8>,
    /// 32-byte Ed25519 public key for signature verification
    pub public_signing_key: Vec<u8>,
    /// Encrypted Ed25519 private key
    pub private_signing_key: Vec<u8>,
}

/// Saves a key bundle for a user, replacing any existing keys.
pub fn save(conn: &Connection, keys: Keys) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO keys
        (user_id, public_encryption_key, private_encryption_key, public_signing_key, private_signing_key)\
        VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            keys.user_id,
            keys.public_encryption_key,
            keys.private_encryption_key,
            keys.public_signing_key,
            keys.private_signing_key,
        ],
    )?;

    Ok(())
}

/// Saves keys for the currently active user profile.
pub fn save_active_user_keys(conn: &Connection, keys: Keys) -> Result<()> {
    let user_id: String = conn.query_row(
        "SELECT id FROM profiles WHERE is_active = 1 LIMIT 1",
        [],
        |row| row.get(0),
    )?;

    conn.execute(
        "INSERT OR REPLACE INTO keys
        (user_id, public_encryption_key, private_encryption_key, public_signing_key, private_signing_key)\
        VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            user_id,
            keys.public_encryption_key,
            keys.private_encryption_key,
            keys.public_signing_key,
            keys.private_signing_key,
        ],
    )?;

    Ok(())
}

/// Retrieves keys for a specific user by ID.
pub fn get_keys(conn: &Connection, user_id: &str) -> Result<Option<Keys>> {
    let mut stmt = conn.prepare(
        "SELECT user_id, public_encryption_key, private_encryption_key,
                public_signing_key, private_signing_key
         FROM keys
         WHERE user_id = ?1",
    )?;

    let mut rows = stmt.query(params![user_id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Keys {
            user_id: row.get(0)?,
            public_encryption_key: row.get(1)?,
            private_encryption_key: row.get(2)?,
            public_signing_key: row.get(3)?,
            private_signing_key: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}

/// Retrieves keys for the currently active user profile.
pub fn get_active_user_keys(conn: &Connection) -> Result<Option<Keys>> {
    let mut stmt = conn.prepare(
        "SELECT k.user_id, k.public_encryption_key, k.private_encryption_key,
                k.public_signing_key, k.private_signing_key
         FROM keys k
         JOIN profiles p ON k.user_id = p.id
         WHERE p.is_active = 1
         LIMIT 1",
    )?;

    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Keys {
            user_id: row.get(0)?,
            public_encryption_key: row.get(1)?,
            private_encryption_key: row.get(2)?,
            public_signing_key: row.get(3)?,
            private_signing_key: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}
