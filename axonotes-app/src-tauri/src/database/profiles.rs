#![allow(dead_code)]

use crate::database::helpers::SqlU128;
use crate::workos_auth::Profile;
use rusqlite::{params, Connection, Result};

/// Get the currently active profile
pub fn get_active(conn: &Connection) -> Result<Option<Profile>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, name, access_token, refresh_token 
         FROM profiles 
         WHERE is_active = 1 
         LIMIT 1",
    )?;

    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Profile {
            id: row.get(0)?,
            email: row.get(1)?,
            name: row.get(2)?,
            access_token: row.get(3)?,
            refresh_token: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}

/// Get last sync time of active user
pub fn get_last_active_sync_time(conn: &Connection) -> Result<Option<u128>> {
    let mut stmt = conn.prepare(
        "SELECT last_batch_sync
        FROM profiles
        WHERE is_active = 1
        LIMIT 1",
    )?;

    let mut rows = stmt.query([])?;

    if let Some(row) = rows.next()? {
        let maybe_sync: Option<SqlU128> = row.get(0)?;
        Ok(Some(maybe_sync.map_or(0, |s| s.0)))
    } else {
        Ok(None)
    }
}

/// Get all profiles
pub fn get_all(conn: &Connection) -> Result<Vec<Profile>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, name, access_token, refresh_token 
         FROM profiles 
         ORDER BY created_at DESC",
    )?;

    let profiles = stmt
        .query_map([], |row| {
            Ok(Profile {
                id: row.get(0)?,
                email: row.get(1)?,
                name: row.get(2)?,
                access_token: row.get(3)?,
                refresh_token: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>>>()?;

    Ok(profiles)
}

/// Save a profile (insert or update) and optionally set as active
pub fn save(conn: &Connection, profile: &Profile, set_active: bool) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    // If setting as active, deactivate all others first
    if set_active {
        conn.execute("UPDATE profiles SET is_active = 0", [])?;
    }

    // Insert or replace profile
    conn.execute(
        "INSERT OR REPLACE INTO profiles 
         (id, email, name, access_token, refresh_token, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 
                 COALESCE((SELECT created_at FROM profiles WHERE id = ?1), ?7),
                 ?8)",
        params![
            &profile.id,
            &profile.email,
            &profile.name,
            &profile.access_token,
            &profile.refresh_token,
            if set_active { 1 } else { 0 },
            now,
            now,
        ],
    )?;

    Ok(())
}

/// Set a profile as the active one
pub fn set_active(conn: &Connection, profile_id: &str) -> Result<()> {
    // Deactivate all
    conn.execute("UPDATE profiles SET is_active = 0", [])?;

    // Activate the selected one
    let rows_affected = conn.execute(
        "UPDATE profiles SET is_active = 1 WHERE id = ?1",
        params![profile_id],
    )?;

    if rows_affected == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }

    Ok(())
}

/// Set the last sync time of the active user
pub fn set_last_active_sync_time(conn: &Connection, last_sync_time: u128) -> Result<()> {
    conn.execute(
        "UPDATE profiles SET last_batch_sync = ?1 WHERE is_active = 1",
        params![SqlU128(last_sync_time)],
    )?;

    Ok(())
}

/// Delete a profile
pub fn delete(conn: &Connection, profile_id: &str) -> Result<()> {
    conn.execute("DELETE FROM profiles WHERE id = ?1", params![profile_id])?;
    Ok(())
}

/// Update access token for a profile
pub fn update_access_token(conn: &Connection, profile_id: &str, access_token: &str) -> Result<()> {
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "UPDATE profiles SET access_token = ?1, updated_at = ?2 WHERE id = ?3",
        params![access_token, now, profile_id],
    )?;

    Ok(())
}

/// Get a profile by ID
pub fn get_by_id(conn: &Connection, profile_id: &str) -> Result<Option<Profile>> {
    let mut stmt = conn.prepare(
        "SELECT id, email, name, access_token, refresh_token 
         FROM profiles 
         WHERE id = ?1",
    )?;

    let mut rows = stmt.query(params![profile_id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(Profile {
            id: row.get(0)?,
            email: row.get(1)?,
            name: row.get(2)?,
            access_token: row.get(3)?,
            refresh_token: row.get(4)?,
        }))
    } else {
        Ok(None)
    }
}
