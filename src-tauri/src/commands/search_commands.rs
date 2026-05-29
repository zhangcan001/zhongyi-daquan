use crate::errors::AppResult;
use crate::models::search::{
    KnowledgeSearchResult, ListCacheRequest, ListCacheResponse, RebuildSearchIndexResponse,
    SearchRequest, SearchResponse, SearchSeedOptions, SearchSeedResponse,
};
use crate::services::search_index_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn search_knowledge(
    state: State<'_, AppState>,
    request: SearchRequest,
) -> AppResult<SearchResponse> {
    search_index_service::search(&state.database, request)
}

#[tauri::command]
pub fn list_knowledge_cache(
    state: State<'_, AppState>,
    request: ListCacheRequest,
) -> AppResult<ListCacheResponse> {
    search_index_service::list_cache(&state.database, request)
}

#[tauri::command]
pub fn rebuild_search_index(state: State<'_, AppState>) -> AppResult<RebuildSearchIndexResponse> {
    search_index_service::rebuild_search_index(&state.database)
}

#[tauri::command]
pub fn generate_search_performance_test_data(
    state: State<'_, AppState>,
    options: SearchSeedOptions,
) -> AppResult<SearchSeedResponse> {
    search_index_service::generate_performance_test_data(&state.database, options)
}

#[tauri::command]
pub fn smoke_test_searches(state: State<'_, AppState>) -> AppResult<Vec<KnowledgeSearchResult>> {
    search_index_service::smoke_test_searches(&state.database)
}
