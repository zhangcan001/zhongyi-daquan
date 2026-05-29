use crate::db::connection::Database;
use crate::errors::AppResult;

#[allow(dead_code)]
pub fn table_exists(database: &Database, table_name: &str) -> AppResult<bool> {
    database.with_connection(|connection| {
        let count: i64 = connection.query_row(
            "SELECT COUNT(1) FROM sqlite_master WHERE type IN ('table', 'virtual table') AND name = ?1",
            [table_name],
            |row| row.get(0),
        )?;
        Ok(count == 1)
    })
}

#[allow(dead_code)]
pub fn index_exists(database: &Database, index_name: &str) -> AppResult<bool> {
    database.with_connection(|connection| {
        let count: i64 = connection.query_row(
            "SELECT COUNT(1) FROM sqlite_master WHERE type = 'index' AND name = ?1",
            [index_name],
            |row| row.get(0),
        )?;
        Ok(count == 1)
    })
}

#[allow(dead_code)]
pub fn foreign_keys_enabled(database: &Database) -> AppResult<bool> {
    database.with_connection(|connection| {
        let enabled: i64 = connection.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
        Ok(enabled == 1)
    })
}
