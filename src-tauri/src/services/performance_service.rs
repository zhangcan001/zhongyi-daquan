use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::search::PerformanceLogEntry;
use crate::repositories::performance_repository;

pub fn list_recent(database: &Database, limit: Option<u32>) -> AppResult<Vec<PerformanceLogEntry>> {
    performance_repository::list_recent(database, limit.unwrap_or(50))
}
