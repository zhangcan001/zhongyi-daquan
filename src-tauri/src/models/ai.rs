use serde::{Deserialize, Serialize};

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
