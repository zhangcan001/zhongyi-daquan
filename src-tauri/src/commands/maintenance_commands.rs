use crate::errors::AppResult;
use crate::models::runtime::{BackgroundJob, CleanOldPerformanceLogsRequest, MaintenanceReport};
use crate::services::maintenance_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn run_rebuild_search_index_job(state: State<'_, AppState>) -> AppResult<MaintenanceReport> {
    maintenance_service::rebuild_search_index_job(&state.database)
}

#[tauri::command]
pub fn start_rebuild_search_index_job(state: State<'_, AppState>) -> AppResult<BackgroundJob> {
    maintenance_service::start_rebuild_search_index_job(&state.database)
}

#[tauri::command]
pub fn optimize_database(state: State<'_, AppState>) -> AppResult<MaintenanceReport> {
    maintenance_service::optimize_database(&state.database)
}

#[tauri::command]
pub fn clean_temp_imports(state: State<'_, AppState>) -> AppResult<MaintenanceReport> {
    maintenance_service::clean_temp_imports(&state.database, &state.data_dir)
}

#[tauri::command]
pub fn clean_old_performance_logs(
    state: State<'_, AppState>,
    request: CleanOldPerformanceLogsRequest,
) -> AppResult<MaintenanceReport> {
    maintenance_service::clean_old_performance_logs(&state.database, request)
}

#[tauri::command]
pub fn export_performance_report(state: State<'_, AppState>) -> AppResult<MaintenanceReport> {
    maintenance_service::export_performance_report(&state.database, &state.data_dir)
}

#[tauri::command]
pub fn check_data_integrity(state: State<'_, AppState>) -> AppResult<MaintenanceReport> {
    maintenance_service::check_data_integrity(&state.database)
}

#[tauri::command]
pub fn clear_database_content(state: State<'_, AppState>) -> AppResult<MaintenanceReport> {
    maintenance_service::clear_database_content(&state.database)
}
