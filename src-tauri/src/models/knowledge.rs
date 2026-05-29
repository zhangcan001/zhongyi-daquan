use serde::{Deserialize, Serialize};

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
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HerbDetail {
    pub item_id: i64,
    pub nature_flavor: Option<String>,
    pub meridians: Option<String>,
    pub effects: Option<String>,
    pub indications: Option<String>,
    pub dosage: Option<String>,
    pub contraindications: Option<String>,
    pub compatibility: Option<String>,
    pub notes: Option<String>,
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
