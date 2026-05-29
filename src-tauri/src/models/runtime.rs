use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundJob {
    pub id: Option<i64>,
    pub job_type: String,
    pub status: String,
    pub progress: f64,
    pub params_json: Option<String>,
    pub result_json: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceLog {
    pub id: Option<i64>,
    pub action: String,
    pub duration_ms: i64,
    pub row_count: Option<i64>,
    pub query_type: Option<String>,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLog {
    pub id: Option<i64>,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub created_at: String,
}
