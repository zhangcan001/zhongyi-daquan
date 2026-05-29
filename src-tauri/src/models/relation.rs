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
