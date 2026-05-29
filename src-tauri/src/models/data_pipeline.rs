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
