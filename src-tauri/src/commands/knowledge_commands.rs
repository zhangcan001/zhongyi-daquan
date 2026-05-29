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
