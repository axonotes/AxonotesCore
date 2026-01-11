//! # Document Permissions Storage
//!
//! Handles storage and retrieval of document permissions in the local database.
//! Synced from STDB for offline access.
//!
//! ## Multi-Account Support
//!
//! All operations are scoped by `identity_id` to support multiple user accounts
//! on the same device.

use crate::stdb_bindings::{DocumentPermission, Role};
use rusqlite::{params, Connection, Result};
use spacetimedb_sdk::Identity;
use std::collections::HashSet;

/// Converts a Role enum to its string representation for storage.
const fn role_to_string(role: Role) -> &'static str {
    match role {
        Role::Owner => "Owner",
        Role::Editor => "Editor",
        Role::Reader => "Reader",
    }
}

/// Converts a string back to a Role enum.
fn string_to_role(s: &str) -> Result<Role, rusqlite::Error> {
    match s {
        "Owner" => Ok(Role::Owner),
        "Editor" => Ok(Role::Editor),
        "Reader" => Ok(Role::Reader),
        _ => Err(rusqlite::Error::InvalidColumnType(
            0,
            "role".into(),
            rusqlite::types::Type::Text,
        )),
    }
}

/// Saves a single document permission, replacing if it already exists.
pub fn save(
    conn: &Connection,
    identity_id: &Identity,
    permission: &DocumentPermission,
) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO permissions
        (permission_id, identity_id, doc_id, user_id, role)
        VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            permission.permission_id,
            identity_id.to_byte_array().as_slice(),
            permission.doc_id,
            permission.user_id.to_byte_array().as_slice(),
            role_to_string(permission.role),
        ],
    )?;
    Ok(())
}

/// Gets all permission IDs for a specific identity (for sync comparison).
fn get_all_ids_for_identity(conn: &Connection, identity_id: &Identity) -> Result<HashSet<String>> {
    let mut stmt = conn.prepare("SELECT permission_id FROM permissions WHERE identity_id = ?1")?;
    let rows = stmt.query_map(params![identity_id.to_byte_array().as_slice()], |row| {
        row.get::<_, String>(0)
    })?;
    rows.collect()
}

/// Syncs permissions for a specific identity.
///
/// Strategy to avoid race conditions with real-time callbacks:
/// 1. Collect local permission IDs and server permission IDs
/// 2. Delete permissions that are NOT in server set
/// 3. Upsert all server permissions
///
/// This preserves permissions inserted by real-time callbacks during sync.
pub fn sync(
    conn: &Connection,
    identity_id: &Identity,
    server_permissions: &[DocumentPermission],
) -> Result<()> {
    // Collect IDs
    let local_ids = get_all_ids_for_identity(conn, identity_id)?;
    let server_ids: HashSet<String> = server_permissions
        .iter()
        .map(|p| p.permission_id.clone())
        .collect();

    // Delete permissions that are not on server
    for local_id in &local_ids {
        if !server_ids.contains(local_id) {
            conn.execute(
                "DELETE FROM permissions WHERE identity_id = ?1 AND permission_id = ?2",
                params![identity_id.to_byte_array().as_slice(), local_id],
            )?;
        }
    }

    // Upsert all server permissions
    for permission in server_permissions {
        save(conn, identity_id, permission)?;
    }

    Ok(())
}

/// Deletes a specific permission by permission_id.
pub fn delete(conn: &Connection, identity_id: &Identity, permission_id: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM permissions WHERE identity_id = ?1 AND permission_id = ?2",
        params![identity_id.to_byte_array().as_slice(), permission_id],
    )
}

/// Deletes all permissions for a specific identity.
pub fn delete_for_identity(conn: &Connection, identity_id: &Identity) -> Result<usize> {
    conn.execute(
        "DELETE FROM permissions WHERE identity_id = ?1",
        params![identity_id.to_byte_array().as_slice()],
    )
}

/// Gets all permissions for a specific identity.
pub fn get_all_for_identity(
    conn: &Connection,
    identity_id: &Identity,
) -> Result<Vec<DocumentPermission>> {
    let mut stmt = conn.prepare(
        "SELECT permission_id, doc_id, user_id, role
         FROM permissions
         WHERE identity_id = ?1",
    )?;

    let rows = stmt.query_map(params![identity_id.to_byte_array().as_slice()], |row| {
        let user_id_bytes: Vec<u8> = row.get(2)?;
        let user_id_arr: [u8; 32] = user_id_bytes.try_into().map_err(|_| {
            rusqlite::Error::InvalidColumnType(2, "user_id".into(), rusqlite::types::Type::Blob)
        })?;

        let role_str: String = row.get(3)?;
        let role = string_to_role(&role_str)?;

        Ok(DocumentPermission {
            permission_id: row.get(0)?,
            doc_id: row.get(1)?,
            user_id: Identity::from_byte_array(user_id_arr),
            role,
        })
    })?;

    rows.collect()
}

/// Gets permissions for a specific document and identity.
pub fn get_by_doc_id(
    conn: &Connection,
    identity_id: &Identity,
    doc_id: &str,
) -> Result<Vec<DocumentPermission>> {
    let mut stmt = conn.prepare(
        "SELECT permission_id, doc_id, user_id, role
         FROM permissions
         WHERE identity_id = ?1 AND doc_id = ?2",
    )?;

    let rows = stmt.query_map(
        params![identity_id.to_byte_array().as_slice(), doc_id],
        |row| {
            let user_id_bytes: Vec<u8> = row.get(2)?;
            let user_id_arr: [u8; 32] = user_id_bytes.try_into().map_err(|_| {
                rusqlite::Error::InvalidColumnType(2, "user_id".into(), rusqlite::types::Type::Blob)
            })?;

            let role_str: String = row.get(3)?;
            let role = string_to_role(&role_str)?;

            Ok(DocumentPermission {
                permission_id: row.get(0)?,
                doc_id: row.get(1)?,
                user_id: Identity::from_byte_array(user_id_arr),
                role,
            })
        },
    )?;

    rows.collect()
}
