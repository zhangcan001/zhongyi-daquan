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

            // 首先尝试精确匹配
            if let Some(field) = field_alias(&normalized) {
                return Some((header.clone(), field.to_string()));
            }

            // 如果精确匹配失败，尝试相似度匹配
            let best_match = find_best_match(&normalized, TARGET_FIELDS);
            if let Some((field, score)) = best_match {
                if score > 0.6 {
                    return Some((header.clone(), String::from(field)));
                }
            }

            None
        })
        .collect()
}

fn find_best_match<'a>(input: &str, candidates: &'a [&'a str]) -> Option<(&'a str, f64)> {
    let mut best_match = None;
    let mut best_score = 0.0;

    for candidate in candidates {
        let score = string_similarity(input, &normalize_header(candidate));
        if score > best_score {
            best_score = score;
            best_match = Some(*candidate);
        }
    }

    best_match.map(|m| (m, best_score))
}

fn string_similarity(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }

    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    // Jaro-Winkler 相似度简化版
    let len1 = s1.chars().count();
    let len2 = s2.chars().count();

    if len1 == 0 && len2 == 0 {
        return 1.0;
    }

    let max_len = len1.max(len2);
    let match_window = (max_len / 2).saturating_sub(1).max(1);

    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();

    let mut s1_matches = vec![false; len1];
    let mut s2_matches = vec![false; len2];

    let mut matches = 0;

    for i in 0..len1 {
        let start = i.saturating_sub(match_window);
        let end = (i + match_window + 1).min(len2);

        for j in start..end {
            if s2_matches[j] || s1_chars[i] != s2_chars[j] {
                continue;
            }
            s1_matches[i] = true;
            s2_matches[j] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    let mut transpositions = 0;
    let mut k = 0;

    for i in 0..len1 {
        if !s1_matches[i] {
            continue;
        }
        while !s2_matches[k] {
            k += 1;
        }
        if s1_chars[i] != s2_chars[k] {
            transpositions += 1;
        }
        k += 1;
    }

    let jaro = (matches as f64 / len1 as f64
        + matches as f64 / len2 as f64
        + (matches as f64 - transpositions as f64 / 2.0) / matches as f64)
        / 3.0;

    jaro
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
