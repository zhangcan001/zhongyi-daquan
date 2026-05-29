use crate::errors::AppResult;
use crate::models::knowledge::{GridSaveRequest, GridSaveResponse};
use crate::services::entry_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn save_grid_dirty_rows(
    state: State<'_, AppState>,
    request: GridSaveRequest,
) -> AppResult<GridSaveResponse> {
    entry_service::save_grid_dirty_rows(&state.database, request)
}
