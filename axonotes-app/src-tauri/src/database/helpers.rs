use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

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
        Ok(SqlU128(u128::from_le_bytes(arr)))
    }
}

impl ToSql for SqlU128 {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.0.to_le_bytes().to_vec()))
    }
}
