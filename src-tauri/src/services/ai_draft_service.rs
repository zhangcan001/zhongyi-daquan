use crate::errors::AppResult;
use crate::models::ai::{AiCommandResponse, AI_DISABLED_MESSAGE};

pub struct AiDraftService;

impl AiDraftService {
    pub fn placeholder_status(task_id: Option<i64>) -> AppResult<AiCommandResponse> {
        Ok(AiCommandResponse {
            enabled: false,
            status: "disabled".to_string(),
            task_id,
            message: AI_DISABLED_MESSAGE.to_string(),
            answer: None,
            citations: Vec::new(),
            used_context_items: Vec::new(),
            warnings: Vec::new(),
            safety_notice: None,
        })
    }
}
