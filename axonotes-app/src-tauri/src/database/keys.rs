use rusqlite::{params, Connection, Result};

pub struct Keys {
    pub user_id: String,
    pub public_encryption_key: Vec<u8>,
    pub private_encryption_key: Vec<u8>,
    pub public_signing_key: Vec<u8>,
    pub private_signing_key: Vec<u8>,
}

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
