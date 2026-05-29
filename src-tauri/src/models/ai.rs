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

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderSetting {
    pub id: Option<i64>,
    pub provider_type: String,
    pub provider_name: Option<String>,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub model_name: Option<String>,
    pub timeout_seconds: Option<i64>,
    pub max_tokens: Option<i64>,
    pub temperature: Option<f64>,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiPromptTemplate {
    pub id: Option<i64>,
    pub task_type: String,
    pub name: String,
    pub system_prompt: Option<String>,
    pub user_prompt_template: Option<String>,
    pub output_schema_json: Option<String>,
    pub safety_rules: Option<String>,
    pub version_no: i64,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTask {
    pub id: Option<i64>,
    pub task_type: String,
    pub status: String,
    pub input_json: Option<String>,
    pub output_json: Option<String>,
    pub error_message: Option<String>,
    pub related_batch_id: Option<i64>,
    pub related_row_id: Option<i64>,
    pub related_item_id: Option<i64>,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiDraft {
    pub id: Option<i64>,
    pub task_id: Option<i64>,
    pub draft_type: String,
    pub draft_json: String,
    pub target_type: Option<String>,
    pub status: String,
    pub review_note: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCallLog {
    pub id: Option<i64>,
    pub provider_type: Option<String>,
    pub model_name: Option<String>,
    pub task_type: Option<String>,
    pub input_hash: Option<String>,
    pub prompt_version: Option<i64>,
    pub request_summary: Option<String>,
    pub response_summary: Option<String>,
    pub duration_ms: Option<i64>,
    pub token_usage_json: Option<String>,
    pub status: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
}
