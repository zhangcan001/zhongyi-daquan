use crate::errors::AppResult;
use crate::models::ai::{AiCommandResponse, AI_DISABLED_MESSAGE};

#[allow(dead_code)]
pub struct PromptTemplateService;

#[allow(dead_code)]
impl PromptTemplateService {
    pub fn placeholder_status() -> AppResult<AiCommandResponse> {
        Ok(AiCommandResponse {
            enabled: false,
            status: "disabled".to_string(),
            task_id: None,
            message: AI_DISABLED_MESSAGE.to_string(),
            answer: None,
            citations: Vec::new(),
            used_context_items: Vec::new(),
            warnings: Vec::new(),
            safety_notice: None,
        })
    }
}
