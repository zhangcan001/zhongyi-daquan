use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::data_pipeline::{FieldMappingTemplate, SaveMappingTemplateRequest};
use crate::repositories::mapping_repository;
use serde_json::{Map, Value};
use std::collections::HashMap;

const TARGET_FIELDS: &[&str] = &[
    "type",
    "code",
    "name",
    "alias",
    "pinyin",
    "category",
    "summary",
    "content",
    "source_note",
    "tags",
    "nature_flavor",
    "meridians",
    "effects",
    "indications",
    "dosage",
    "contraindications",
    "compatibility",
    "notes",
    "source_text",
    "composition",
    "usage",
    "explanation",
    "modifications",
    "meridian_code",
    "yin_yang",
    "hand_foot",
    "organ_relation",
    "paired_meridian",
    "pathway_text",
    "main_indications",
    "acupoint_code",
    "body_region",
    "body_subregion",
    "side_type",
    "standard_location",
    "locating_method",
    "bone_cun",
    "anatomy",
    "functions",
    "needling_summary",
    "moxibustion_summary",
    "massage_summary",
    "precautions",
    "risk_level",
    "symptoms",
    "tongue",
    "pulse",
    "pathogenesis",
    "treatment_principle",
    "common_syndromes",
    "care_advice",
    "medical_warning",
];

pub fn save_template(
    database: &Database,
    request: SaveMappingTemplateRequest,
) -> AppResult<FieldMappingTemplate> {
    mapping_repository::insert_template(database, request)
}

pub fn list_templates(
    database: &Database,
    target_type: Option<String>,
) -> AppResult<Vec<FieldMappingTemplate>> {
    mapping_repository::list_templates(database, target_type)
}

pub fn mapping_from_template(
    database: &Database,
    template_id: Option<i64>,
) -> AppResult<Option<HashMap<String, String>>> {
    if let Some(id) = template_id {
        let template = mapping_repository::get_template(database, id)?;
        let mapping = serde_json::from_str(&template.mapping_json)?;
        Ok(Some(mapping))
    } else {
        Ok(None)
    }
}

pub fn auto_mapping(headers: &[String]) -> HashMap<String, String> {
    headers
        .iter()
        .filter_map(|header| {
            let normalized = normalize_header(header);
            field_alias(&normalized).map(|field| (header.clone(), field.to_string()))
        })
        .collect()
}

pub fn apply_mapping(
    raw: &Map<String, Value>,
    mapping: Option<&HashMap<String, String>>,
    target_type: &str,
) -> Map<String, Value> {
    let mut output = Map::new();
    let auto = mapping
        .is_none()
        .then(|| auto_mapping(&raw.keys().map(ToString::to_string).collect::<Vec<String>>()));
    let active = mapping.or(auto.as_ref());

    for (source, value) in raw {
        let target = active
            .and_then(|mapping| mapping.get(source))
            .cloned()
            .or_else(|| field_alias(&normalize_header(source)).map(ToString::to_string));

        if let Some(target) = target {
            if TARGET_FIELDS.contains(&target.as_str()) {
                output.insert(target, value.clone());
            }
        }
    }
    output
        .entry("type".to_string())
        .or_insert_with(|| Value::String(target_type.to_string()));
    output
}

fn normalize_header(header: &str) -> String {
    header
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "")
}

fn field_alias(header: &str) -> Option<&'static str> {
    match header {
        "type" | "类型" | "知识类型" => Some("type"),
        "code" | "编号" | "编码" | "穴位编号" | "经络编号" => Some("code"),
        "name" | "名称" | "药名" | "方名" | "穴名" | "病名" | "证名" => Some("name"),
        "alias" | "别名" | "异名" => Some("alias"),
        "pinyin" | "拼音" => Some("pinyin"),
        "category" | "分类" | "类别" => Some("category"),
        "summary" | "摘要" | "简介" => Some("summary"),
        "content" | "正文" | "内容" => Some("content"),
        "sourcenote" | "来源" | "出处" => Some("source_note"),
        "tags" | "标签" => Some("tags"),
        "natureflavor" | "性味" => Some("nature_flavor"),
        "meridians" | "归经" | "经络" => Some("meridians"),
        "effects" | "功效" => Some("effects"),
        "indications" | "主治" => Some("indications"),
        "dosage" | "用量" => Some("dosage"),
        "contraindications" | "禁忌" => Some("contraindications"),
        "composition" | "组成" => Some("composition"),
        "usage" | "用法" => Some("usage"),
        "standardlocation" | "定位" | "标准定位" => Some("standard_location"),
        "locatingmethod" | "取穴" | "取穴方法" => Some("locating_method"),
        "functions" | "作用" => Some("functions"),
        "symptoms" | "症状" => Some("symptoms"),
        "notes" | "备注" => Some("notes"),
        _ => None,
    }
}
