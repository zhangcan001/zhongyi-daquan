use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundJob {
    pub id: i64,
    pub job_type: String,
    pub status: String,
    pub progress: f64,
    pub params_json: Option<String>,
    pub result_json: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateJobRequest {
    pub job_type: String,
    pub params_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateJobProgressRequest {
    pub job_id: i64,
    pub progress: f64,
    pub result_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkJobSuccessRequest {
    pub job_id: i64,
    pub result_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkJobFailedRequest {
    pub job_id: i64,
    pub error_message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListJobsRequest {
    pub status: Option<String>,
    pub job_type: Option<String>,
    pub limit: Option<u32>,
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
    pub id: i64,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordPerformanceLogRequest {
    pub action: String,
    pub duration_ms: i64,
    pub row_count: Option<i64>,
    pub query_type: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordAuditLogRequest {
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub before_json: Option<String>,
    pub after_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupManifest {
    pub backup_id: String,
    pub created_at: String,
    pub app_name: String,
    pub database_file: String,
    pub includes_images: bool,
    pub includes_config: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupReport {
    pub job: BackgroundJob,
    pub backup_id: String,
    pub backup_dir: String,
    pub manifest_path: String,
    pub database_path: String,
    pub images_path: Option<String>,
    pub config_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreBackupRequest {
    pub backup_dir: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub job: BackgroundJob,
    pub restored_from: String,
    pub safety_backup_dir: String,
    pub database_restored: bool,
    pub images_restored: bool,
    pub config_restored: bool,
    pub rebuild_search_index_note: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MaintenanceReport {
    pub job: BackgroundJob,
    pub action: String,
    pub message: String,
    pub affected_rows: Option<i64>,
    pub output_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanOldPerformanceLogsRequest {
    pub keep_days: Option<u32>,
}
