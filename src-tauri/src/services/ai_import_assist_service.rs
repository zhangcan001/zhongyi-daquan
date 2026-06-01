use crate::errors::AppResult;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiImportAssistResponse {
    pub task_type: String,
    pub enabled: bool,
    pub message: String,
}

pub fn request_assist(task_type: &str) -> AppResult<AiImportAssistResponse> {
    Ok(AiImportAssistResponse {
        task_type: task_type.to_string(),
        enabled: false,
        message: "AI 导入辅助当前未启用，系统使用本地规则处理。".to_string(),
    })
}
