use crate::errors::AppResult;
use crate::services::export_service;
use crate::AppState;
use std::path::PathBuf;
use tauri::State;

#[tauri::command]
pub fn export_knowledge_to_json(
    state: State<'_, AppState>,
    item_ids: Vec<i64>,
    output_path: String,
) -> AppResult<export_service::ExportResult> {
    let path = PathBuf::from(output_path);
    export_service::export_to_json(&state.database, item_ids, &path)
}

#[tauri::command]
pub fn export_knowledge_to_csv(
    state: State<'_, AppState>,
    item_ids: Vec<i64>,
    output_path: String,
) -> AppResult<export_service::ExportResult> {
    let path = PathBuf::from(output_path);
    export_service::export_to_csv(&state.database, item_ids, &path)
}

#[tauri::command]
pub fn export_knowledge_to_excel(
    state: State<'_, AppState>,
    item_ids: Vec<i64>,
    output_path: String,
) -> AppResult<export_service::ExportResult> {
    let path = PathBuf::from(output_path);
    export_service::export_to_excel(&state.database, item_ids, &path)
}
