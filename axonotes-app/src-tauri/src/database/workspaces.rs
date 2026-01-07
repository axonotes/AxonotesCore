//! # Workspace Storage
//!
//! Manages storage of UI workspace configurations (Dockview layouts).
//!
//! ## Workspace Management
//!
//! Workspaces store JSON-stringified Dockview layout configurations.
//! Each workspace has a unique ID and stores the complete layout state
//! so users can organize their app however they want.

#![allow(dead_code)]
#![allow(clippy::cast_sign_loss)] // Unix timestamps are always positive
#![allow(clippy::cast_possible_wrap)] // Timestamps fit well within i64 range

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

/// Workspace configuration entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub config: String,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Create a new workspace.
pub fn create(conn: &Connection, id: &str, config: &str) -> Result<Workspace> {
    let now = chrono::Utc::now().timestamp() as u64;

    conn.execute(
        "INSERT INTO workspaces (id, config, created_at, updated_at) VALUES (?, ?, ?, ?)",
        params![id, config, now as i64, now as i64],
    )?;

    Ok(Workspace {
        id: id.to_string(),
        config: config.to_string(),
        created_at: now,
        updated_at: now,
    })
}

/// Get a workspace by ID.
pub fn get_by_id(conn: &Connection, id: &str) -> Result<Option<Workspace>> {
    let mut stmt =
        conn.prepare("SELECT id, config, created_at, updated_at FROM workspaces WHERE id = ?")?;

    let mut rows = stmt.query(params![id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Workspace {
            id: row.get(0)?,
            config: row.get(1)?,
            created_at: row.get::<_, i64>(2)? as u64,
            updated_at: row.get::<_, i64>(3)? as u64,
        }))
    } else {
        Ok(None)
    }
}

/// List all workspaces.
pub fn get_all(conn: &Connection) -> Result<Vec<Workspace>> {
    let mut stmt = conn.prepare(
        "SELECT id, config, created_at, updated_at FROM workspaces ORDER BY created_at DESC",
    )?;

    let workspaces = stmt
        .query_map([], |row| {
            Ok(Workspace {
                id: row.get(0)?,
                config: row.get(1)?,
                created_at: row.get::<_, i64>(2)? as u64,
                updated_at: row.get::<_, i64>(3)? as u64,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    Ok(workspaces)
}

/// Update a workspace's config.
pub fn update(conn: &Connection, id: &str, config: &str) -> Result<Workspace> {
    let now = chrono::Utc::now().timestamp() as u64;

    let rows = conn.execute(
        "UPDATE workspaces SET config = ?, updated_at = ? WHERE id = ?",
        params![config, now as i64, id],
    )?;

    if rows == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }

    Ok(Workspace {
        id: id.to_string(),
        config: config.to_string(),
        created_at: 0, // We don't fetch original created_at, caller can re-fetch if needed
        updated_at: now,
    })
}

/// Delete a workspace.
pub fn delete(conn: &Connection, id: &str) -> Result<bool> {
    let rows = conn.execute("DELETE FROM workspaces WHERE id = ?", params![id])?;
    Ok(rows > 0)
}
