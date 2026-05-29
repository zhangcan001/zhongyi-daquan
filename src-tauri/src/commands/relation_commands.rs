use crate::errors::AppResult;
use crate::models::relation::{
    AcceptRelationSuggestionResponse, GenerateRelationSuggestionsRequest,
    GenerateRelationSuggestionsResponse, ListDuplicateCandidatesRequest,
    ListDuplicateCandidatesResponse, ListRelationSuggestionsRequest,
    ListRelationSuggestionsResponse, MergeDuplicateCandidateRequest,
    MergeDuplicateCandidateResponse, RunDuplicateDetectionRequest, RunDuplicateDetectionResponse,
};
use crate::services::{dedup_service, relation_suggest_service};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn run_duplicate_detection(
    state: State<'_, AppState>,
    request: RunDuplicateDetectionRequest,
) -> AppResult<RunDuplicateDetectionResponse> {
    dedup_service::run_duplicate_detection(&state.database, request)
}

#[tauri::command]
pub fn list_duplicate_candidates(
    state: State<'_, AppState>,
    request: ListDuplicateCandidatesRequest,
) -> AppResult<ListDuplicateCandidatesResponse> {
    dedup_service::list_duplicate_candidates(&state.database, request)
}

#[tauri::command]
pub fn merge_duplicate_candidate(
    state: State<'_, AppState>,
    request: MergeDuplicateCandidateRequest,
) -> AppResult<MergeDuplicateCandidateResponse> {
    dedup_service::merge_duplicate_candidate(&state.database, request)
}

#[tauri::command]
pub fn generate_relation_suggestions(
    state: State<'_, AppState>,
    request: GenerateRelationSuggestionsRequest,
) -> AppResult<GenerateRelationSuggestionsResponse> {
    relation_suggest_service::generate_relation_suggestions(&state.database, request)
}

#[tauri::command]
pub fn list_relation_suggestions(
    state: State<'_, AppState>,
    request: ListRelationSuggestionsRequest,
) -> AppResult<ListRelationSuggestionsResponse> {
    relation_suggest_service::list_relation_suggestions(&state.database, request)
}

#[tauri::command]
pub fn accept_relation_suggestion(
    state: State<'_, AppState>,
    suggestion_id: i64,
) -> AppResult<AcceptRelationSuggestionResponse> {
    relation_suggest_service::accept_relation_suggestion(&state.database, suggestion_id)
}

#[tauri::command]
pub fn reject_relation_suggestion(state: State<'_, AppState>, suggestion_id: i64) -> AppResult<()> {
    relation_suggest_service::reject_relation_suggestion(&state.database, suggestion_id)
}
