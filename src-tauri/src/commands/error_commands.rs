use crate::errors::AppResult;
use crate::services::error_log_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn log_error(
    state: State<'_, AppState>,
    error_type: String,
    error_message: String,
    stack_trace: Option<String>,
    context: Option<String>,
) -> AppResult<i64> {
    error_log_service::log_error(
        &state.database,
        &error_type,
        &error_message,
        stack_trace.as_deref(),
        context.as_deref(),
    )
}

#[tauri::command]
pub fn get_recent_errors(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> AppResult<Vec<error_log_service::ErrorLog>> {
    error_log_service::get_recent_errors(&state.database, limit.unwrap_or(50))
}

#[tauri::command]
pub fn get_error_statistics(
    state: State<'_, AppState>,
) -> AppResult<error_log_service::ErrorStatistics> {
    error_log_service::get_error_statistics(&state.database)
}

#[tauri::command]
pub fn clear_old_error_logs(
    state: State<'_, AppState>,
    days: i64,
) -> AppResult<i64> {
    error_log_service::clear_old_error_logs(&state.database, days)
}
