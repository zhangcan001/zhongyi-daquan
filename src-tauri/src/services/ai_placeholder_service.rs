use crate::errors::AppResult;
use crate::models::ai::AiPlaceholderResponse;

pub fn placeholder_response() -> AppResult<AiPlaceholderResponse> {
    Ok(AiPlaceholderResponse {
        enabled: false,
        message: "当前版本未启用 AI 调用".to_string(),
    })
}
