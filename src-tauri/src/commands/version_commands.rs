use crate::errors::AppResult;
use crate::models::knowledge::KnowledgeVersion;
use crate::services::version_service::{self, VersionComparison};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn list_knowledge_versions(
    state: State<'_, AppState>,
    item_id: i64,
) -> AppResult<Vec<KnowledgeVersion>> {
    version_service::list_versions(&state.database, item_id)
}

#[tauri::command]
pub fn get_knowledge_version(
    state: State<'_, AppState>,
    version_id: i64,
) -> AppResult<KnowledgeVersion> {
    version_service::get_version(&state.database, version_id)
}

#[tauri::command]
pub fn compare_knowledge_versions(
    state: State<'_, AppState>,
    version_id_a: i64,
    version_id_b: i64,
) -> AppResult<VersionComparison> {
    version_service::compare_versions(&state.database, version_id_a, version_id_b)
}

#[tauri::command]
pub fn rollback_knowledge_version(
    state: State<'_, AppState>,
    version_id: i64,
) -> AppResult<i64> {
    version_service::rollback_to_version(&state.database, version_id)
}
