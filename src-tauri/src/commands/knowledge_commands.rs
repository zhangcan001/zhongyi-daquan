use crate::errors::AppResult;
use crate::models::knowledge::{
    DashboardStats, FavoriteItem, KnowledgeDetailResponse, KnowledgeInput, KnowledgeListRequest,
    KnowledgeListResponse, RecentView, UserNote,
};
use crate::services::knowledge_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn list_knowledge_items(
    state: State<'_, AppState>,
    request: KnowledgeListRequest,
) -> AppResult<KnowledgeListResponse> {
    knowledge_service::list(&state.database, request)
}

#[tauri::command]
pub fn get_knowledge_detail(
    state: State<'_, AppState>,
    item_id: i64,
) -> AppResult<KnowledgeDetailResponse> {
    knowledge_service::get(&state.database, item_id)
}

#[tauri::command]
pub fn create_knowledge_item(
    state: State<'_, AppState>,
    input: KnowledgeInput,
) -> AppResult<KnowledgeDetailResponse> {
    knowledge_service::create(&state.database, input)
}

#[tauri::command]
pub fn update_knowledge_item(
    state: State<'_, AppState>,
    item_id: i64,
    input: KnowledgeInput,
) -> AppResult<KnowledgeDetailResponse> {
    knowledge_service::update(&state.database, item_id, input)
}

#[tauri::command]
pub fn delete_knowledge_item(state: State<'_, AppState>, item_id: i64) -> AppResult<()> {
    knowledge_service::delete(&state.database, item_id)
}

#[tauri::command]
pub fn set_knowledge_favorite(
    state: State<'_, AppState>,
    request: crate::models::knowledge::FavoriteRequest,
) -> AppResult<KnowledgeDetailResponse> {
    knowledge_service::set_favorite(&state.database, request.item_id, request.is_favorite)
}

#[tauri::command]
pub fn toggle_favorite(
    state: State<'_, AppState>,
    item_id: i64,
) -> AppResult<KnowledgeDetailResponse> {
    knowledge_service::toggle_favorite(&state.database, item_id)
}

#[tauri::command]
pub fn list_favorites(state: State<'_, AppState>) -> AppResult<Vec<FavoriteItem>> {
    knowledge_service::list_favorites(&state.database)
}

#[tauri::command]
pub fn record_recent_view(state: State<'_, AppState>, item_id: i64) -> AppResult<RecentView> {
    knowledge_service::record_recent_view(&state.database, item_id)
}

#[tauri::command]
pub fn list_recent_views(
    state: State<'_, AppState>,
    limit: Option<i64>,
) -> AppResult<Vec<RecentView>> {
    knowledge_service::list_recent_views(&state.database, limit)
}

#[tauri::command]
pub fn save_user_note(
    state: State<'_, AppState>,
    item_id: i64,
    note_text: String,
) -> AppResult<UserNote> {
    knowledge_service::save_user_note(&state.database, item_id, note_text)
}

#[tauri::command]
pub fn delete_user_note(state: State<'_, AppState>, note_id: i64) -> AppResult<()> {
    knowledge_service::delete_user_note(&state.database, note_id)
}

#[tauri::command]
pub fn get_dashboard_stats(state: State<'_, AppState>) -> AppResult<DashboardStats> {
    knowledge_service::dashboard_stats(&state.database)
}

#[tauri::command]
pub fn batch_delete_knowledge_items(
    state: State<'_, AppState>,
    item_ids: Vec<i64>,
) -> AppResult<crate::services::knowledge_service::BatchOperationResult> {
    crate::services::knowledge_service::batch_delete(&state.database, item_ids)
}

#[tauri::command]
pub fn batch_update_knowledge_status(
    state: State<'_, AppState>,
    item_ids: Vec<i64>,
    data_status: String,
) -> AppResult<crate::services::knowledge_service::BatchOperationResult> {
    crate::services::knowledge_service::batch_update_status(&state.database, item_ids, data_status)
}

#[tauri::command]
pub fn batch_add_knowledge_tags(
    state: State<'_, AppState>,
    item_ids: Vec<i64>,
    tags: Vec<String>,
) -> AppResult<crate::services::knowledge_service::BatchOperationResult> {
    crate::services::knowledge_service::batch_add_tags(&state.database, item_ids, tags)
}
