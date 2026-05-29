use crate::errors::AppResult;
use crate::models::ai::{AiCommandResponse, AI_DISABLED_MESSAGE};

pub struct PromptTemplateService;

impl PromptTemplateService {
    pub fn placeholder_status() -> AppResult<AiCommandResponse> {
        Ok(AiCommandResponse {
            enabled: false,
            status: "disabled".to_string(),
            task_id: None,
            message: AI_DISABLED_MESSAGE.to_string(),
        })
    }
}
