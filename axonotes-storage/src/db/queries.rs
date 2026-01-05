use super::models::{BlobOwnership, Document, User};
use crate::utils::Result;
use rusqlite::Connection;

/// Get or create user by user_id
pub fn get_or_create_user(conn: &Connection, user_id: &[u8]) -> Result<User> {
    // Try to get existing user
    let user = conn.query_row(
        "SELECT user_id, used_bytes, created_at, updated_at FROM users WHERE user_id = ?1",
        [user_id],
        |row| {
            Ok(User {
                user_id: row.get(0)?,
                used_bytes: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        },
    );

    match user {
        Ok(user) => Ok(user),
        Err(rusqlite::Error::QueryReturnedNoRows) => {
            // Create new user
            conn.execute(
                "INSERT INTO users (user_id, used_bytes) VALUES (?1, 0)",
                [user_id],
            )?;

            // Fetch the newly created user
            get_or_create_user(conn, user_id)
        }
        Err(e) => Err(e.into()),
    }
}

/// Get user by user_id
#[allow(dead_code)]
pub fn get_user(conn: &Connection, user_id: &[u8]) -> Result<Option<User>> {
    let user = conn.query_row(
        "SELECT user_id, used_bytes, created_at, updated_at FROM users WHERE user_id = ?1",
        [user_id],
        |row| {
            Ok(User {
                user_id: row.get(0)?,
                used_bytes: row.get(1)?,
                created_at: row.get(2)?,
                updated_at: row.get(3)?,
            })
        },
    );

    match user {
        Ok(user) => Ok(Some(user)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Update user's used_bytes
pub fn update_user_quota(conn: &Connection, user_id: &[u8], delta_bytes: i64) -> Result<()> {
    conn.execute(
        "UPDATE users SET used_bytes = used_bytes + ?1, updated_at = unixepoch() WHERE user_id = ?2",
        (delta_bytes, user_id),
    )?;
    Ok(())
}

/// Create document
pub fn create_document(
    conn: &Connection,
    document_id: &str,
    owner_id: &[u8],
    public_key: &[u8],
) -> Result<()> {
    conn.execute(
        "INSERT INTO documents (document_id, owner_id, public_key) VALUES (?1, ?2, ?3)",
        (document_id, owner_id, public_key),
    )?;
    Ok(())
}

/// Get document by ID
pub fn get_document(conn: &Connection, document_id: &str) -> Result<Option<Document>> {
    let doc = conn.query_row(
        "SELECT document_id, owner_id, public_key, created_at FROM documents WHERE document_id = ?1",
        [document_id],
        |row| {
            Ok(Document {
                document_id: row.get(0)?,
                owner_id: row.get(1)?,
                public_key: row.get(2)?,
                created_at: row.get(3)?,
            })
        },
    );

    match doc {
        Ok(doc) => Ok(Some(doc)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Update document public key
pub fn update_document_public_key(
    conn: &Connection,
    document_id: &str,
    new_public_key: &[u8],
) -> Result<()> {
    conn.execute(
        "UPDATE documents SET public_key = ?1 WHERE document_id = ?2",
        (new_public_key, document_id),
    )?;
    Ok(())
}

/// Create blob ownership record
pub fn create_blob_ownership(
    conn: &Connection,
    hash: &str,
    user_id: &[u8],
    document_id: &str,
    size_bytes: i64,
) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO blob_ownership (hash, user_id, document_id, size_bytes) VALUES (?1, ?2, ?3, ?4)",
        (hash, user_id, document_id, size_bytes),
    )?;
    Ok(())
}

/// Get all blobs for a document
pub fn get_blobs_for_document(conn: &Connection, document_id: &str) -> Result<Vec<BlobOwnership>> {
    let mut stmt = conn.prepare(
        "SELECT hash, user_id, document_id, size_bytes, created_at FROM blob_ownership WHERE document_id = ?1",
    )?;

    let blobs: Vec<BlobOwnership> = stmt
        .query_map([document_id], |row| {
            Ok(BlobOwnership {
                hash: row.get(0)?,
                user_id: row.get(1)?,
                document_id: row.get(2)?,
                size_bytes: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(blobs)
}

/// Delete document and all associated blobs
pub fn delete_document(conn: &Connection, document_id: &str) -> Result<u64> {
    // Get total size of blobs to subtract from user quota
    let total_size: i64 = conn.query_row(
        "SELECT COALESCE(SUM(size_bytes), 0) FROM blob_ownership WHERE document_id = ?1",
        [document_id],
        |row| row.get(0),
    )?;

    // Delete blob ownership records (CASCADE will handle this, but explicit is clearer)
    conn.execute(
        "DELETE FROM blob_ownership WHERE document_id = ?1",
        [document_id],
    )?;

    // Delete document
    conn.execute(
        "DELETE FROM documents WHERE document_id = ?1",
        [document_id],
    )?;

    Ok(total_size as u64)
}

/// Check if blob exists
pub fn blob_exists(conn: &Connection, hash: &str) -> Result<bool> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM blob_ownership WHERE hash = ?1",
        [hash],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::init_schema;

    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn test_get_or_create_user() {
        let conn = setup_test_db();
        let user_id = b"test_user_id_12345678901234567890";

        // Create user
        let user1 = get_or_create_user(&conn, user_id).unwrap();
        assert_eq!(user1.user_id, user_id);
        assert_eq!(user1.used_bytes, 0);

        // Get existing user
        let user2 = get_or_create_user(&conn, user_id).unwrap();
        assert_eq!(user1.user_id, user2.user_id);
        assert_eq!(user1.created_at, user2.created_at);
    }

    #[test]
    fn test_update_user_quota() {
        let conn = setup_test_db();
        let user_id = b"test_user_id_12345678901234567890";

        get_or_create_user(&conn, user_id).unwrap();
        update_user_quota(&conn, user_id, 1024).unwrap();

        let user = get_user(&conn, user_id).unwrap().unwrap();
        assert_eq!(user.used_bytes, 1024);

        update_user_quota(&conn, user_id, 512).unwrap();
        let user = get_user(&conn, user_id).unwrap().unwrap();
        assert_eq!(user.used_bytes, 1536);
    }

    #[test]
    fn test_document_crud() {
        let conn = setup_test_db();
        let user_id = b"test_user_id_12345678901234567890";
        let public_key = b"test_public_key_32_bytes_long___";

        get_or_create_user(&conn, user_id).unwrap();

        // Create document
        create_document(&conn, "doc_123", user_id, public_key).unwrap();

        // Get document
        let doc = get_document(&conn, "doc_123").unwrap().unwrap();
        assert_eq!(doc.document_id, "doc_123");
        assert_eq!(doc.owner_id, user_id);
        assert_eq!(doc.public_key, public_key);

        // Update public key
        let new_key = b"new_public_key_32_bytes_long____";
        update_document_public_key(&conn, "doc_123", new_key).unwrap();

        let doc = get_document(&conn, "doc_123").unwrap().unwrap();
        assert_eq!(doc.public_key, new_key);
    }

    #[test]
    fn test_blob_ownership() {
        let conn = setup_test_db();
        let user_id = b"test_user_id_12345678901234567890";
        let public_key = b"test_public_key_32_bytes_long___";

        get_or_create_user(&conn, user_id).unwrap();
        create_document(&conn, "doc_123", user_id, public_key).unwrap();

        // Create blob ownership
        create_blob_ownership(&conn, "abcd1234", user_id, "doc_123", 2048).unwrap();

        // Check blob exists
        assert!(blob_exists(&conn, "abcd1234").unwrap());
        assert!(!blob_exists(&conn, "nonexistent").unwrap());

        // Get blobs for document
        let blobs = get_blobs_for_document(&conn, "doc_123").unwrap();
        assert_eq!(blobs.len(), 1);
        assert_eq!(blobs[0].hash, "abcd1234");
        assert_eq!(blobs[0].size_bytes, 2048);
    }

    #[test]
    fn test_delete_document() {
        let conn = setup_test_db();
        let user_id = b"test_user_id_12345678901234567890";
        let public_key = b"test_public_key_32_bytes_long___";

        get_or_create_user(&conn, user_id).unwrap();
        create_document(&conn, "doc_123", user_id, public_key).unwrap();
        create_blob_ownership(&conn, "hash1", user_id, "doc_123", 1024).unwrap();
        create_blob_ownership(&conn, "hash2", user_id, "doc_123", 2048).unwrap();

        // Delete document
        let total_size = delete_document(&conn, "doc_123").unwrap();
        assert_eq!(total_size, 3072);

        // Verify document is gone
        assert!(get_document(&conn, "doc_123").unwrap().is_none());

        // Verify blobs are gone
        assert!(!blob_exists(&conn, "hash1").unwrap());
        assert!(!blob_exists(&conn, "hash2").unwrap());
    }
}
