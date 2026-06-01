use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataImportBatch {
    pub id: Option<i64>,
    pub file_name: String,
    pub import_type: String,
    pub target_type: String,
    pub status: String,
    pub total_count: i64,
    pub parsed_count: i64,
    pub valid_count: i64,
    pub warning_count: i64,
    pub error_count: i64,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataImportRow {
    pub id: Option<i64>,
    pub batch_id: i64,
    pub row_index: i64,
    pub raw_json: Option<String>,
    pub mapped_json: Option<String>,
    pub normalized_json: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub warning_message: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataValidationIssue {
    pub id: Option<i64>,
    pub batch_id: i64,
    pub row_id: Option<i64>,
    pub severity: String,
    pub issue_code: String,
    pub field_name: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldMappingTemplate {
    pub id: Option<i64>,
    pub name: String,
    pub target_type: String,
    pub source_headers_json: String,
    pub mapping_json: String,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StandardTerm {
    pub id: Option<i64>,
    pub term_type: String,
    pub standard_name: String,
    pub aliases: Option<String>,
    pub code: Option<String>,
    pub notes: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationRule {
    pub id: Option<i64>,
    pub target_type: String,
    pub field_name: String,
    pub rule_type: String,
    pub rule_params_json: Option<String>,
    pub severity: String,
    pub message: String,
    pub enabled: bool,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportParsedPreview {
    pub headers: Vec<String>,
    pub rows: Vec<serde_json::Value>,
    pub detection: ImportDetectionResult,
    pub mapping_suggestions: Vec<FieldMappingSuggestion>,
    pub direct_import_ready: bool,
    pub warnings: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPackageDescriptor {
    pub package_root: String,
    pub package_name: Option<String>,
    pub import_profile: Option<String>,
    pub import_intent: Option<String>,
    pub duplicate_policy: Option<String>,
    pub ai_assist: Option<bool>,
    pub manifest_found: bool,
    pub manifest_path: Option<String>,
    pub files: Vec<ImportPackageFile>,
    pub primary_files: Vec<String>,
    pub auxiliary_files: Vec<ImportPackageFile>,
    pub auto_stage_files: Vec<String>,
    pub skipped_manifest_files: Vec<ImportPackageFile>,
    pub detected_type: String,
    pub record_count: i64,
    pub direct_import_ready: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPackageFile {
    pub path: String,
    pub import_type: String,
    pub target: Option<String>,
    pub primary: bool,
    pub role: Option<String>,
    pub auto_stage: bool,
    pub description: Option<String>,
    pub skip_reason: Option<String>,
    pub required: bool,
    pub exists: bool,
    pub record_count: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDetectionResult {
    pub detected_type: String,
    pub confidence: f64,
    pub reason: String,
    pub sample_fields: Vec<String>,
    pub record_count: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldMappingSuggestion {
    pub source_field: String,
    pub target_field: Option<String>,
    pub confidence: f64,
    pub decision: String,
    pub reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateImportRequest {
    pub file_name: String,
    pub target_type: String,
    pub content: String,
    pub mapping: Option<std::collections::HashMap<String, String>>,
    pub template_id: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportBatchSummary {
    pub batch: DataImportBatch,
    pub total_rows: i64,
    pub importable_rows: i64,
    pub warning_rows: i64,
    pub error_rows: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingIssue {
    pub severity: String,
    pub issue_code: String,
    pub field_name: Option<String>,
    pub message: String,
    pub suggestion: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingRowView {
    pub id: i64,
    pub row_index: i64,
    pub raw: serde_json::Value,
    pub mapped: serde_json::Value,
    pub normalized: serde_json::Value,
    pub status: String,
    pub error_message: Option<String>,
    pub warning_message: Option<String>,
    pub issues: Vec<StagingIssue>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingPage {
    pub summary: ImportBatchSummary,
    pub rows: Vec<StagingRowView>,
    pub page: i64,
    pub page_size: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveMappingTemplateRequest {
    pub name: String,
    pub target_type: String,
    pub source_headers: Vec<String>,
    pub mapping: std::collections::HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanStepRequest {
    pub batch_id: i64,
    pub step_type: String,
    pub params: Option<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanStepResult {
    pub step_id: Option<i64>,
    pub affected_rows: i64,
    pub summary: ImportBatchSummary,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmImportResult {
    pub batch_id: i64,
    pub imported_count: i64,
    pub skipped_count: i64,
    pub summary: ImportBatchSummary,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportQualityReport {
    pub batch_id: i64,
    pub detected_type: String,
    pub total_rows: i64,
    pub importable_rows: i64,
    pub warning_rows: i64,
    pub error_rows: i64,
    pub field_coverage: std::collections::BTreeMap<String, f64>,
    pub empty_field_counts: std::collections::BTreeMap<String, i64>,
    pub duplicate_fingerprint_count: i64,
    pub search_terms_imported_count: i64,
    pub import_diff: ImportDiffReport,
    pub searchable_keywords_checked: std::collections::BTreeMap<String, bool>,
    pub suggestions: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportDiffReport {
    pub inserted_items: i64,
    pub skipped_rows: i64,
    pub duplicate_warning_rows: i64,
    pub imported_search_terms: i64,
    pub affected_types: std::collections::BTreeMap<String, i64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RollbackImportResult {
    pub batch_id: i64,
    pub deleted_items: i64,
    pub deleted_search_terms: i64,
    pub summary: ImportBatchSummary,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlan {
    pub plan_id: String,
    pub package_path: String,
    pub package_name: Option<String>,
    pub import_intent: String,
    pub duplicate_policy: String,
    pub total_records: i64,
    pub create_count: i64,
    pub update_count: i64,
    pub attach_annotation_count: i64,
    pub skip_duplicate_count: i64,
    pub needs_review_count: i64,
    pub reject_invalid_count: i64,
    pub warnings: Vec<String>,
    pub actions: Vec<ImportPlanAction>,
    pub ai_message: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPlanAction {
    pub row_index: i64,
    pub action_type: String,
    pub item_type: Option<String>,
    pub name: Option<String>,
    pub existing_item_id: Option<i64>,
    pub confidence: f64,
    pub reason: String,
    pub draft_json: serde_json::Value,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteImportPlanResult {
    pub plan_id: String,
    pub created_count: i64,
    pub merged_count: i64,
    pub attached_annotation_count: i64,
    pub skipped_count: i64,
    pub needs_review_count: i64,
    pub rejected_count: i64,
    pub search_index_rebuilt: bool,
    pub warnings: Vec<String>,
}
