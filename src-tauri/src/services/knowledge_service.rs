use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::repositories::knowledge_repository;

#[allow(dead_code)]
pub fn count_items(database: &Database) -> AppResult<i64> {
    knowledge_repository::count_items(database)
}
