use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeRelation {
    pub id: Option<i64>,
    pub source_item_id: i64,
    pub target_item_id: i64,
    pub relation_type: String,
    pub note: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeRelationView {
    pub id: i64,
    pub source_item_id: i64,
    pub target_item_id: i64,
    pub related_item_id: i64,
    pub related_item_type: String,
    pub related_name: String,
    pub related_code: Option<String>,
    pub related_category: Option<String>,
    pub relation_type: String,
    pub direction: String,
    pub note: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationSuggestion {
    pub id: Option<i64>,
    pub source_item_id: Option<i64>,
    pub target_item_id: Option<i64>,
    pub relation_type: String,
    pub confidence: Option<f64>,
    pub reason: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCandidate {
    pub id: Option<i64>,
    pub batch_id: Option<i64>,
    pub existing_item_id: Option<i64>,
    pub imported_row_id: Option<i64>,
    pub match_type: String,
    pub match_score: Option<f64>,
    pub reason: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeFingerprint {
    pub item_id: i64,
    pub item_type: String,
    pub code_norm: Option<String>,
    pub name_norm: Option<String>,
    pub pinyin_norm: Option<String>,
    pub alias_norm: Option<String>,
    pub fingerprint: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateCandidateDetail {
    pub id: i64,
    pub batch_id: Option<i64>,
    pub existing_item_id: Option<i64>,
    pub duplicate_item_id: Option<i64>,
    pub imported_row_id: Option<i64>,
    pub existing_name: Option<String>,
    pub duplicate_name: Option<String>,
    pub imported_name: Option<String>,
    pub match_type: String,
    pub match_score: Option<f64>,
    pub reason: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDuplicateDetectionRequest {
    pub batch_id: Option<i64>,
    pub item_type: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunDuplicateDetectionResponse {
    pub fingerprints_upserted: i64,
    pub candidates_created: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDuplicateCandidatesRequest {
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDuplicateCandidatesResponse {
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub candidates: Vec<DuplicateCandidateDetail>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeDuplicateCandidateRequest {
    pub candidate_id: i64,
    pub strategy: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeDuplicateCandidateResponse {
    pub candidate_id: i64,
    pub existing_item_id: i64,
    pub created_item_id: Option<i64>,
    pub merge_record_id: Option<i64>,
    pub status: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRelationSuggestionsRequest {
    pub item_type: Option<String>,
    pub source_item_id: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRelationSuggestionsResponse {
    pub suggestions_created: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRelationSuggestionsRequest {
    pub status: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationSuggestionDetail {
    pub id: i64,
    pub source_item_id: Option<i64>,
    pub target_item_id: Option<i64>,
    pub source_name: Option<String>,
    pub target_name: Option<String>,
    pub relation_type: String,
    pub confidence: Option<f64>,
    pub reason: Option<String>,
    pub status: String,
    pub created_at: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListRelationSuggestionsResponse {
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub suggestions: Vec<RelationSuggestionDetail>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptRelationSuggestionResponse {
    pub suggestion_id: i64,
    pub relation_id: i64,
}
