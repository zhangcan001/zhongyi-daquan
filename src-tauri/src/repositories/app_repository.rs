use crate::db::connection::Database;
use crate::errors::AppResult;

pub fn database_ready(database: &Database) -> AppResult<bool> {
    database.with_connection(|connection| {
        let count: i64 = connection.query_row(
            "SELECT COUNT(1) FROM sqlite_master WHERE type = 'table' AND name = 'knowledge_items'",
            [],
            |row| row.get(0),
        )?;
        Ok(count == 1)
    })
}
