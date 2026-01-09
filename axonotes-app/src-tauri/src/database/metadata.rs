//! # Document Metadata Storage
//!
//! Handles storage and retrieval of document metadata in the local database.
//! Synced from STDB for offline access with merge semantics.
//!
//! ## Sync Strategy
//!
//! - **Tags**: Merge (union) of local and server tags
//! - **Path**: Server wins, but returns old path if different for notification
//!
//! ## Multi-Account Support
//!
//! All operations are scoped by `identity_id`.

use crate::encryption::document::{DecryptedDocumentMetadata, DecryptedMetadata};
use rusqlite::{params, Connection, Result};
use spacetimedb_sdk::Identity;

/// Result of merging server metadata with local metadata.
pub struct MergeResult {
    /// The merged metadata to save
    pub merged: DecryptedDocumentMetadata,
    /// If path changed, contains (old_path, new_path) for notification
    pub path_changed: Option<(String, String)>,
}

/// Saves a single decrypted metadata entry, replacing if it already exists.
pub fn save(
    conn: &Connection,
    identity_id: &Identity,
    meta: &DecryptedDocumentMetadata,
) -> Result<()> {
    let tags_json = serde_json::to_string(&meta.metadata.tags).unwrap_or_else(|_| "[]".to_string());

    conn.execute(
        "INSERT OR REPLACE INTO document_metadata
        (meta_id, identity_id, doc_id, path, tags, version)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            meta.meta_id,
            identity_id.to_byte_array().as_slice(),
            meta.doc_id,
            meta.metadata.path,
            tags_json,
            meta.metadata.version,
        ],
    )?;
    Ok(())
}

/// Gets metadata for a specific document and identity.
pub fn get_by_doc_id(
    conn: &Connection,
    identity_id: &Identity,
    doc_id: &str,
) -> Result<Option<DecryptedDocumentMetadata>> {
    let mut stmt = conn.prepare(
        "SELECT meta_id, doc_id, path, tags, version
         FROM document_metadata
         WHERE identity_id = ?1 AND doc_id = ?2",
    )?;

    let mut rows = stmt.query(params![identity_id.to_byte_array().as_slice(), doc_id])?;

    if let Some(row) = rows.next()? {
        let tags_json: String = row.get(3)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

        Ok(Some(DecryptedDocumentMetadata {
            meta_id: row.get(0)?,
            user_id: *identity_id,
            doc_id: row.get(1)?,
            metadata: DecryptedMetadata {
                version: row.get(4)?,
                path: row.get(2)?,
                tags,
            },
        }))
    } else {
        Ok(None)
    }
}

/// Gets all metadata for a specific identity.
pub fn get_all_for_identity(
    conn: &Connection,
    identity_id: &Identity,
) -> Result<Vec<DecryptedDocumentMetadata>> {
    let mut stmt = conn.prepare(
        "SELECT meta_id, doc_id, path, tags, version
         FROM document_metadata
         WHERE identity_id = ?1",
    )?;

    let rows = stmt.query_map(params![identity_id.to_byte_array().as_slice()], |row| {
        let tags_json: String = row.get(3)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

        Ok(DecryptedDocumentMetadata {
            meta_id: row.get(0)?,
            user_id: *identity_id,
            doc_id: row.get(1)?,
            metadata: DecryptedMetadata {
                version: row.get(4)?,
                path: row.get(2)?,
                tags,
            },
        })
    })?;

    rows.collect()
}

/// Deletes metadata for a specific document.
pub fn delete_by_doc_id(conn: &Connection, identity_id: &Identity, doc_id: &str) -> Result<usize> {
    conn.execute(
        "DELETE FROM document_metadata WHERE identity_id = ?1 AND doc_id = ?2",
        params![identity_id.to_byte_array().as_slice(), doc_id],
    )
}

/// Deletes all metadata for a specific identity.
pub fn delete_for_identity(conn: &Connection, identity_id: &Identity) -> Result<usize> {
    conn.execute(
        "DELETE FROM document_metadata WHERE identity_id = ?1",
        params![identity_id.to_byte_array().as_slice()],
    )
}

/// Merges server metadata with local metadata for a single document.
///
/// Merge strategy:
/// - Tags: Union of local and server tags (deduplicated, sorted)
/// - Path: Server wins, but returns old path if different
/// - Version: Take server version
fn merge_single(
    conn: &Connection,
    identity_id: &Identity,
    server_meta: &DecryptedDocumentMetadata,
) -> Result<MergeResult> {
    // Get existing local metadata
    let local = get_by_doc_id(conn, identity_id, &server_meta.doc_id)?;

    match local {
        Some(local_meta) => {
            // Merge tags (union)
            let mut merged_tags: Vec<String> = local_meta.metadata.tags.clone();
            for tag in &server_meta.metadata.tags {
                if !merged_tags.contains(tag) {
                    merged_tags.push(tag.clone());
                }
            }
            merged_tags.sort();

            // Check if path changed
            let path_changed = if local_meta.metadata.path != server_meta.metadata.path {
                Some((
                    local_meta.metadata.path.clone(),
                    server_meta.metadata.path.clone(),
                ))
            } else {
                None
            };

            // Build merged metadata (server path wins)
            let merged = DecryptedDocumentMetadata {
                meta_id: server_meta.meta_id.clone(),
                user_id: server_meta.user_id,
                doc_id: server_meta.doc_id.clone(),
                metadata: DecryptedMetadata {
                    version: server_meta.metadata.version,
                    path: server_meta.metadata.path.clone(),
                    tags: merged_tags,
                },
            };

            Ok(MergeResult {
                merged,
                path_changed,
            })
        }
        None => {
            // No local data, use server data as-is
            Ok(MergeResult {
                merged: server_meta.clone(),
                path_changed: None,
            })
        }
    }
}

/// Sync metadata with merge logic.
/// Returns list of (doc_id, old_path, new_path) for documents where path changed.
pub fn sync_with_merge(
    conn: &Connection,
    identity_id: &Identity,
    server_metadata: &[DecryptedDocumentMetadata],
) -> Result<Vec<(String, String, String)>> {
    let mut path_changes = Vec::new();

    // Get current local doc_ids to detect deletions
    let local_doc_ids: Vec<String> = get_all_for_identity(conn, identity_id)?
        .into_iter()
        .map(|m| m.doc_id)
        .collect();

    // Track which doc_ids exist on server
    let server_doc_ids: Vec<String> = server_metadata.iter().map(|m| m.doc_id.clone()).collect();

    // Delete metadata for documents no longer on server
    for doc_id in &local_doc_ids {
        if !server_doc_ids.contains(doc_id) {
            delete_by_doc_id(conn, identity_id, doc_id)?;
        }
    }

    // Merge each server metadata entry
    for meta in server_metadata {
        let result = merge_single(conn, identity_id, meta)?;

        if let Some((old_path, new_path)) = result.path_changed {
            path_changes.push((meta.doc_id.clone(), old_path, new_path));
        }

        save(conn, identity_id, &result.merged)?;
    }

    Ok(path_changes)
}
