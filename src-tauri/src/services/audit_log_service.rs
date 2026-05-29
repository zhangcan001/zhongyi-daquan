use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::runtime::{AuditLog, RecordAuditLogRequest};
use crate::repositories::audit_repository;

pub fn record(database: &Database, request: RecordAuditLogRequest) -> AppResult<()> {
    if request.action.trim().is_empty() {
        return Err(AppError::InvalidInput("审计动作不能为空".to_string()));
    }
    audit_repository::record(
        database,
        request.action.trim(),
        request.target_type.as_deref(),
        request.target_id,
        request.before_json.as_deref(),
        request.after_json.as_deref(),
    )
}

pub fn list_recent(database: &Database, limit: Option<u32>) -> AppResult<Vec<AuditLog>> {
    audit_repository::list_recent(database, limit.unwrap_or(50))
}
