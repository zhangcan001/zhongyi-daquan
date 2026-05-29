use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchTerm {
    pub id: Option<i64>,
    pub item_id: i64,
    pub term: String,
    pub term_type: String,
    pub weight: i64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeListViewCache {
    pub item_id: i64,
    pub item_type: String,
    pub code: Option<String>,
    pub name: String,
    pub pinyin: Option<String>,
    pub category: Option<String>,
    pub summary: Option<String>,
    pub tags: Option<String>,
    pub data_status: String,
    pub is_favorite: bool,
    pub relation_count: i64,
    pub updated_at: String,
}
