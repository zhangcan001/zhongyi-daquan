use crate::errors::AppResult;
use crate::models::ai::{AiCommandResponse, AI_DISABLED_MESSAGE};

pub struct AiSafetyService;

impl AiSafetyService {
    pub fn blocked_placeholder() -> AppResult<AiCommandResponse> {
        Ok(AiCommandResponse {
            enabled: false,
            status: "blocked".to_string(),
            task_id: None,
            message: AI_DISABLED_MESSAGE.to_string(),
        })
    }
}
