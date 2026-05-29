use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::runtime::RecordPerformanceLogRequest;
use crate::models::search::PerformanceLogEntry;
use crate::repositories::performance_repository;

pub fn list_recent(database: &Database, limit: Option<u32>) -> AppResult<Vec<PerformanceLogEntry>> {
    performance_repository::list_recent(database, limit.unwrap_or(50))
}

pub fn record(database: &Database, request: RecordPerformanceLogRequest) -> AppResult<()> {
    if request.action.trim().is_empty() {
        return Err(AppError::InvalidInput("性能日志动作不能为空".to_string()));
    }
    performance_repository::record(
        database,
        request.action.trim(),
        request.duration_ms.max(0),
        request.row_count,
        request.query_type.as_deref(),
    )
}
