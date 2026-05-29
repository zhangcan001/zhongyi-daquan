use crate::errors::AppResult;
use crate::models::ai::{
    AiCommandResponse, AiPlaceholderResponse, AiProviderSettingsResponse, AiTaskRequest,
    SaveAiProviderSettingsRequest,
};
use crate::services::ai_placeholder_service::{self, AiPlaceholderService};
use crate::services::ai_provider_service::AiProviderService;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn ai_placeholder() -> AppResult<AiPlaceholderResponse> {
    ai_placeholder_service::placeholder_response()
}

#[tauri::command]
pub fn get_ai_provider_settings(
    state: State<'_, AppState>,
) -> AppResult<AiProviderSettingsResponse> {
    AiProviderService::get_settings(&state.database)
}

#[tauri::command]
pub fn save_ai_provider_settings(
    state: State<'_, AppState>,
    settings: SaveAiProviderSettingsRequest,
) -> AppResult<AiProviderSettingsResponse> {
    AiProviderService::save_settings(&state.database, settings)
}

#[tauri::command]
pub fn test_ai_connection() -> AppResult<AiCommandResponse> {
    AiPlaceholderService::test_connection()
}

#[tauri::command]
pub fn run_ai_task(request: AiTaskRequest) -> AppResult<AiCommandResponse> {
    AiPlaceholderService::run_task(request)
}

#[tauri::command]
pub fn get_ai_task_status(task_id: i64) -> AppResult<AiCommandResponse> {
    AiPlaceholderService::get_task_status(task_id)
}

#[tauri::command]
pub fn cancel_ai_task(task_id: i64) -> AppResult<AiCommandResponse> {
    AiPlaceholderService::cancel_task(task_id)
}
