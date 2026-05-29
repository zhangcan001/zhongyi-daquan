use serde::{Deserialize, Serialize};

pub const AI_DISABLED_MESSAGE: &str = "当前版本未启用 AI 调用";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderSettings {
    pub id: Option<i64>,
    pub provider_type: String,
    pub provider_name: Option<String>,
    pub base_url: Option<String>,
    pub model_name: Option<String>,
    pub timeout_seconds: Option<i64>,
    pub max_tokens: Option<i64>,
    pub temperature: Option<f64>,
    pub enabled: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl Default for AiProviderSettings {
    fn default() -> Self {
        Self {
            id: None,
            provider_type: "disabled".to_string(),
            provider_name: None,
            base_url: None,
            model_name: None,
            timeout_seconds: Some(30),
            max_tokens: Some(1024),
            temperature: Some(0.2),
            enabled: false,
            created_at: None,
            updated_at: None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAiProviderSettingsRequest {
    pub provider_type: String,
    pub provider_name: Option<String>,
    pub base_url: Option<String>,
    pub model_name: Option<String>,
    pub timeout_seconds: Option<i64>,
    pub max_tokens: Option<i64>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTaskRequest {
    pub task_type: String,
    pub input_json: Option<String>,
    pub related_batch_id: Option<i64>,
    pub related_row_id: Option<i64>,
    pub related_item_id: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderSettingsResponse {
    pub settings: AiProviderSettings,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCommandResponse {
    pub enabled: bool,
    pub status: String,
    pub task_id: Option<i64>,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPlaceholderResponse {
    pub enabled: bool,
    pub message: String,
}
