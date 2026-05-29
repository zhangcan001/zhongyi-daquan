use crate::errors::AppResult;
use crate::models::runtime::{BackupReport, RestoreBackupRequest, RestoreReport};
use crate::services::backup_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn create_backup(state: State<'_, AppState>) -> AppResult<BackupReport> {
    backup_service::create_backup(&state.database, &state.data_dir)
}

#[tauri::command]
pub fn restore_backup(
    state: State<'_, AppState>,
    request: RestoreBackupRequest,
) -> AppResult<RestoreReport> {
    backup_service::restore_backup(&state.database, &state.data_dir, request)
}
