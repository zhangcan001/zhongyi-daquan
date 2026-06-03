use crate::errors::AppResult;
use crate::models::ai::{
    AiCommandResponse, AiPlaceholderResponse, AiTaskRequest, AI_DISABLED_MESSAGE,
};
use crate::services::ai_draft_service::AiDraftService;
use crate::services::ai_safety_service::AiSafetyService;
use crate::services::prompt_template_service::PromptTemplateService;

pub fn placeholder_response() -> AppResult<AiPlaceholderResponse> {
    Ok(AiPlaceholderResponse {
        enabled: false,
        message: AI_DISABLED_MESSAGE.to_string(),
    })
}

pub struct AiPlaceholderService;

#[allow(dead_code)]
impl AiPlaceholderService {
    pub fn test_connection() -> AppResult<AiCommandResponse> {
        PromptTemplateService::placeholder_status()
    }

    pub fn run_task(request: AiTaskRequest) -> AppResult<AiCommandResponse> {
        let _ = (
            request.task_type,
            request.input_json,
            request.related_batch_id,
            request.related_row_id,
            request.related_item_id,
        );
        AiSafetyService::blocked_placeholder()
    }

    pub fn get_task_status(task_id: i64) -> AppResult<AiCommandResponse> {
        AiDraftService::placeholder_status(Some(task_id))
    }

    pub fn cancel_task(task_id: i64) -> AppResult<AiCommandResponse> {
        AiDraftService::placeholder_status(Some(task_id))
    }
}
