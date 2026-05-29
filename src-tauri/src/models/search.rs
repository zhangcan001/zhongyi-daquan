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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchRequest {
    pub query: String,
    pub item_type: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSearchResult {
    pub item_id: i64,
    pub item_type: String,
    pub code: Option<String>,
    pub name: String,
    pub pinyin: Option<String>,
    pub category: Option<String>,
    pub summary: Option<String>,
    pub tags: Option<String>,
    pub data_status: String,
    pub relation_count: i64,
    pub score: i64,
    pub matched_by: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub query: String,
    pub total: usize,
    pub page: u32,
    pub page_size: u32,
    pub duration_ms: i64,
    pub results: Vec<KnowledgeSearchResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCacheRequest {
    pub item_type: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCacheResponse {
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub duration_ms: i64,
    pub results: Vec<KnowledgeSearchResult>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RebuildSearchIndexResponse {
    pub indexed_items: i64,
    pub search_terms: i64,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSeedOptions {
    pub item_count: Option<u32>,
    pub relation_count: Option<u32>,
    pub reset_existing: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchSeedResponse {
    pub inserted_items: u32,
    pub inserted_relations: u32,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceLogEntry {
    pub id: i64,
    pub action: String,
    pub duration_ms: i64,
    pub row_count: Option<i64>,
    pub query_type: Option<String>,
    pub created_at: String,
}
