use crate::errors::AppResult;
use crate::models::ai::AiPlaceholderResponse;
use crate::services::ai_placeholder_service;

#[tauri::command]
pub fn ai_placeholder() -> AppResult<AiPlaceholderResponse> {
    ai_placeholder_service::placeholder_response()
}
