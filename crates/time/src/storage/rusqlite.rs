use rusqlite::{types::FromSql, ToSql};

use crate::{date_time::PaceDateTime, duration::PaceDuration};

impl ToSql for PaceDateTime {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Text(self.to_string()),
        ))
    }
}

impl FromSql for PaceDateTime {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        value
            .as_str()?
            .parse::<Self>()
            .map_err(|err| rusqlite::types::FromSqlError::Other(Box::new(err)))
    }
}

impl ToSql for PaceDuration {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        Ok(rusqlite::types::ToSqlOutput::Owned(
            rusqlite::types::Value::Integer(
                i64::try_from(self.inner())
                    .map_err(|err| rusqlite::Error::ToSqlConversionFailure(Box::new(err)))?,
            ),
        ))
    }
}

impl FromSql for PaceDuration {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        Ok(Self::new(u64::try_from(value.as_i64()?).map_err(
            |err| rusqlite::types::FromSqlError::Other(Box::new(err)),
        )?))
    }
}
