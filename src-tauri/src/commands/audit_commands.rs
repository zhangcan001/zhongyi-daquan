use crate::errors::AppResult;
use crate::models::runtime::{AuditLog, RecordAuditLogRequest};
use crate::services::audit_log_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn record_audit_log(
    state: State<'_, AppState>,
    request: RecordAuditLogRequest,
) -> AppResult<()> {
    audit_log_service::record(&state.database, request)
}

#[tauri::command]
pub fn list_audit_logs(state: State<'_, AppState>, limit: Option<u32>) -> AppResult<Vec<AuditLog>> {
    audit_log_service::list_recent(&state.database, limit)
}
