use crate::db::connection::Database;
use crate::errors::AppResult;

#[allow(dead_code)]
pub fn count_items(database: &Database) -> AppResult<i64> {
    database.with_connection(|connection| {
        let count =
            connection.query_row("SELECT COUNT(1) FROM knowledge_items", [], |row| row.get(0))?;
        Ok(count)
    })
}
