use crate::errors::AppResult;
use crate::models::data_pipeline::{CleanStepRequest, CleanStepResult};
use crate::services::import_project_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn apply_import_clean_step(
    state: State<'_, AppState>,
    request: CleanStepRequest,
) -> AppResult<CleanStepResult> {
    import_project_service::apply_clean_step(&state.database, request)
}

#[tauri::command]
pub fn undo_last_import_clean_step(
    state: State<'_, AppState>,
    batch_id: i64,
) -> AppResult<CleanStepResult> {
    import_project_service::undo_last_clean_step(&state.database, batch_id)
}
