use crate::errors::AppResult;
use crate::models::search::PerformanceLogEntry;
use crate::services::performance_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn list_performance_logs(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> AppResult<Vec<PerformanceLogEntry>> {
    performance_service::list_recent(&state.database, limit)
}
