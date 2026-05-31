use crate::errors::AppResult;
use crate::models::knowledge::{
    FavoriteRequest, KnowledgeDetailResponse, KnowledgeInput, KnowledgeListRequest,
    KnowledgeListResponse,
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
    request: FavoriteRequest,
) -> AppResult<KnowledgeDetailResponse> {
    knowledge_service::set_favorite(&state.database, request.item_id, request.is_favorite)
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
