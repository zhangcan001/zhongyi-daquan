use crate::db::connection::Database;
use crate::errors::AppResult;

pub fn ai_enabled(database: &Database) -> AppResult<bool> {
    database.with_connection(|connection| {
        let enabled_count: i64 = connection.query_row(
            "SELECT COUNT(1) FROM ai_provider_settings WHERE enabled = 1",
            [],
            |row| row.get(0),
        )?;
        Ok(enabled_count > 0)
    })
}
