use serde::{Deserialize, Serialize};
use serde_json::Value;

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeItem {
    pub id: Option<i64>,
    pub item_type: String,
    pub code: Option<String>,
    pub name: String,
    pub alias: Option<String>,
    pub pinyin: Option<String>,
    pub category: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub source_note: Option<String>,
    pub tags: Option<String>,
    pub data_status: String,
    pub completeness_status: String,
    pub content_version: i64,
    pub is_favorite: bool,
    pub detail: Option<Value>,
    pub import_batch_id: Option<String>,
    pub source_package: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HerbDetail {
    pub item_id: i64,
    pub nature_flavor: Option<String>,
    pub four_qi: Option<String>,
    pub five_flavors: Option<String>,
    pub meridians: Option<String>,
    pub channel_tropism: Option<String>,
    pub toxicity: Option<String>,
    pub effects: Option<String>,
    pub indications: Option<String>,
    pub dosage: Option<String>,
    pub contraindications: Option<String>,
    pub compatibility: Option<String>,
    pub notes: Option<String>,
    pub property_notes: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormulaDetail {
    pub item_id: i64,
    pub source_text: Option<String>,
    pub composition: Option<String>,
    pub usage: Option<String>,
    pub effects: Option<String>,
    pub indications: Option<String>,
    pub explanation: Option<String>,
    pub modifications: Option<String>,
    pub contraindications: Option<String>,
    pub notes: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeridianDetail {
    pub item_id: i64,
    pub meridian_code: Option<String>,
    pub category: Option<String>,
    pub yin_yang: Option<String>,
    pub hand_foot: Option<String>,
    pub organ_relation: Option<String>,
    pub paired_meridian: Option<String>,
    pub pathway_text: Option<String>,
    pub main_indications: Option<String>,
    pub notes: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcupointDetail {
    pub item_id: i64,
    pub acupoint_code: Option<String>,
    pub meridian_item_id: Option<i64>,
    pub body_region: Option<String>,
    pub body_subregion: Option<String>,
    pub side_type: Option<String>,
    pub standard_location: Option<String>,
    pub locating_method: Option<String>,
    pub bone_cun: Option<String>,
    pub anatomy: Option<String>,
    pub functions: Option<String>,
    pub indications: Option<String>,
    pub needling_summary: Option<String>,
    pub moxibustion_summary: Option<String>,
    pub massage_summary: Option<String>,
    pub contraindications: Option<String>,
    pub precautions: Option<String>,
    pub risk_level: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyndromeDetail {
    pub item_id: i64,
    pub symptoms: Option<String>,
    pub tongue: Option<String>,
    pub pulse: Option<String>,
    pub pathogenesis: Option<String>,
    pub treatment_principle: Option<String>,
    pub notes: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiseaseDetail {
    pub item_id: i64,
    pub symptoms: Option<String>,
    pub common_syndromes: Option<String>,
    pub care_advice: Option<String>,
    pub medical_warning: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeListRequest {
    pub item_type: Option<String>,
    pub query: Option<String>,
    pub data_status: Option<String>,
    pub favorite_only: Option<bool>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeListResponse {
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub items: Vec<KnowledgeItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeInput {
    pub item_type: String,
    pub code: Option<String>,
    pub name: String,
    pub alias: Option<String>,
    pub pinyin: Option<String>,
    pub category: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub source_note: Option<String>,
    pub tags: Option<String>,
    pub data_status: String,
    pub completeness_status: String,
    pub is_favorite: bool,
    pub detail: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeDetailResponse {
    pub item: KnowledgeItem,
    pub detail: Value,
    pub annotations: Vec<KnowledgeAnnotation>,
    pub notes: Vec<UserNote>,
    pub versions: Vec<KnowledgeVersion>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeAnnotation {
    pub id: i64,
    pub knowledge_item_id: i64,
    pub annotation_type: String,
    pub source_title: Option<String>,
    pub source_note: Option<String>,
    pub content: String,
    pub detail: Option<Value>,
    pub tags: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentView {
    pub id: i64,
    pub item_id: i64,
    pub item_name: String,
    pub item_type: String,
    pub category: Option<String>,
    pub viewed_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteItem {
    pub id: i64,
    pub item_id: i64,
    pub item_name: String,
    pub item_type: String,
    pub category: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserNote {
    pub id: i64,
    pub item_id: i64,
    pub note_text: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub knowledge_count: i64,
    pub annotation_count: i64,
    pub import_run_count: i64,
    pub favorite_count: i64,
    pub recent_view_count: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeVersion {
    pub id: i64,
    pub item_id: i64,
    pub version_no: i64,
    pub snapshot_json: String,
    pub change_summary: Option<String>,
    pub changed_at: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteRequest {
    pub item_id: i64,
    pub is_favorite: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridSaveRequest {
    pub item_type: String,
    pub rows: Vec<KnowledgeInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridSaveResponse {
    pub saved_count: usize,
    pub item_ids: Vec<i64>,
    pub errors: Vec<GridSaveError>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridSaveError {
    pub row_index: usize,
    pub field_name: String,
    pub message: String,
}
