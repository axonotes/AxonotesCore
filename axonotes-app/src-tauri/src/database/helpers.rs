//! # Database Helpers
//!
//! Type adapters for storing Rust types in SQLite.
//!
//! ## SqlU128
//!
//! SQLite natively supports up to 64-bit integers. Timestamps in Axonotes
//! use `u128` (128-bit) for nanosecond precision. This type stores u128
//! as a 16-byte big-endian blob for correct lexicographic ordering.
//!
//! ## Usage
//!
//! ```ignore
//! use crate::database::helpers::SqlU128;
//!
//! conn.execute(
//!     "INSERT INTO table (timestamp) VALUES (?1)",
//!     params![SqlU128(timestamp)],
//! )?;
//! ```

use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

/// Wrapper for storing u128 values in SQLite as 16-byte big-endian blobs.
///
/// Big-endian encoding ensures correct lexicographic ordering when
/// timestamps are compared as blobs in SQL queries.
pub struct SqlU128(pub u128);

impl FromSql for SqlU128 {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let bytes = value.as_blob()?;
        let arr: [u8; 16] =
            bytes
                .try_into()
                .map_err(|_| rusqlite::types::FromSqlError::InvalidBlobSize {
                    expected_size: 16,
                    blob_size: bytes.len(),
                })?;
        Ok(SqlU128(u128::from_be_bytes(arr))) // <-- be instead of le
    }
}

impl ToSql for SqlU128 {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.to_be_bytes().to_vec())) // <-- be instead of le
    }
}
