use crate::errors::AppResult;
use crate::models::ai::{
    AiCommandResponse, AiPlaceholderResponse, AiProviderSettingsResponse, AiTaskRequest,
    SaveAiProviderSettingsRequest,
};
use crate::services::ai_formula_service::{self, FormulaAiAnswer, FormulaAiRequest};
use crate::services::ai_openai_service;
use crate::services::ai_placeholder_service;
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
pub fn clear_ai_api_key(state: State<'_, AppState>) -> AppResult<AiProviderSettingsResponse> {
    AiProviderService::clear_api_key(&state.database)
}

#[tauri::command]
pub fn get_ai_settings(state: State<'_, AppState>) -> AppResult<AiProviderSettingsResponse> {
    AiProviderService::get_settings(&state.database)
}

#[tauri::command]
pub fn save_ai_settings(
    state: State<'_, AppState>,
    settings: SaveAiProviderSettingsRequest,
) -> AppResult<AiProviderSettingsResponse> {
    AiProviderService::save_settings(&state.database, settings)
}

#[tauri::command]
pub fn test_ai_connection(state: State<'_, AppState>) -> AppResult<AiCommandResponse> {
    ai_openai_service::test_connection(&state.database)
}

#[tauri::command]
pub fn run_ai_task(
    state: State<'_, AppState>,
    request: AiTaskRequest,
) -> AppResult<AiCommandResponse> {
    ai_openai_service::run_task(&state.database, request)
}

#[tauri::command]
pub fn get_ai_task_status(
    state: State<'_, AppState>,
    task_id: i64,
) -> AppResult<AiCommandResponse> {
    ai_openai_service::get_task_status(&state.database, task_id)
}

#[tauri::command]
pub fn cancel_ai_task(state: State<'_, AppState>, task_id: i64) -> AppResult<AiCommandResponse> {
    ai_openai_service::cancel_task(&state.database, task_id)
}

#[tauri::command]
pub fn answer_formula_ai_question(
    state: State<'_, AppState>,
    request: FormulaAiRequest,
) -> AppResult<FormulaAiAnswer> {
    ai_formula_service::answer_formula_question(&state.database, request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::ai::AI_DISABLED_MESSAGE;

    #[test]
    fn ai_placeholder_command_returns_legacy_disabled_status() {
        let placeholder = ai_placeholder().expect("placeholder response");
        assert!(!placeholder.enabled);
        assert_eq!(placeholder.message, AI_DISABLED_MESSAGE);
    }
}
