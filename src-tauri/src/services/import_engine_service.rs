use crate::models::data_pipeline::{FieldMappingSuggestion, ImportDetectionResult};
use serde_json::{Map, Value};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ImportEngineOutput {
    pub detection: ImportDetectionResult,
    pub mapped_rows: Vec<Map<String, Value>>,
    pub mapping_suggestions: Vec<FieldMappingSuggestion>,
    pub direct_import_ready: bool,
    pub warnings: Vec<String>,
}

const DIRECT_TYPES: &[&str] = &[
    "knowledge_items_v1",
    "classic_passages_v1",
    "annotation_items_v1",
];

pub fn detect_import_type(
    file_name: &str,
    source_format: &str,
    rows: &[Map<String, Value>],
) -> ImportDetectionResult {
    let fields = sample_fields(rows);
    let field_set = fields
        .iter()
        .map(|field| normalize_header(field))
        .collect::<HashSet<_>>();
    let record_count = rows.len() as i64;
    let lower_name = file_name.to_ascii_lowercase();

    let candidates = [
        detect_knowledge_items(&lower_name, &field_set),
        detect_classic_passages(&lower_name, &field_set),
        detect_annotation_items(&lower_name, &field_set),
        detect_search_terms(&lower_name, &field_set),
        detect_standard_terms(&field_set),
        detect_relation_suggestions(&field_set),
    ];

    let mut best = candidates.into_iter().flatten().max_by(|left, right| {
        left.1
            .partial_cmp(&right.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    if best.is_none() {
        best = Some(match source_format {
            "csv" => (
                "generic_csv",
                0.5,
                "未命中特定数据包结构，按普通 CSV 进入评分映射",
            ),
            "json" => (
                "generic_json",
                0.45,
                "未命中特定数据包结构，按普通 JSON 进入评分映射",
            ),
            "zip" => (
                "generic_json",
                0.4,
                "ZIP 中未发现可直接导入的 manifest 或标准数据文件",
            ),
            _ => ("unknown", 0.0, "无法识别导入格式"),
        });
    }

    let (detected_type, confidence, reason) = best.unwrap();
    ImportDetectionResult {
        detected_type: detected_type.to_string(),
        confidence,
        reason: reason.to_string(),
        sample_fields: fields,
        record_count,
    }
}

pub fn prepare_import_rows(
    file_name: &str,
    source_format: &str,
    target_type: &str,
    rows: &[Map<String, Value>],
    explicit_mapping: Option<&HashMap<String, String>>,
) -> ImportEngineOutput {
    let detection = detect_import_type(file_name, source_format, rows);
    let suggestions = score_mapping(rows, target_type);
    let direct_import_ready = DIRECT_TYPES.contains(&detection.detected_type.as_str());
    let mut warnings = Vec::new();

    let mapped_rows = if direct_import_ready && explicit_mapping.is_none() {
        match detection.detected_type.as_str() {
            "knowledge_items_v1" => rows.iter().map(adapt_knowledge_item).collect(),
            "classic_passages_v1" => rows.iter().map(adapt_classic_passage).collect(),
            "annotation_items_v1" => rows.iter().map(adapt_annotation_item).collect(),
            _ => Vec::new(),
        }
    } else {
        if detection.detected_type == "search_terms_v1" {
            warnings.push("search_terms_v1 已识别，但 v0.1 当前仍以知识条目导入为主，搜索词表批量入库将在后续版本接入。".to_string());
        }
        if detection.detected_type == "standard_terms_v1" {
            warnings.push("standard_terms_v1 已识别，但 v0.1 当前不直接导入标准词表，请先转为知识条目或等待维护工具接入。".to_string());
        }
        if detection.detected_type == "relation_suggestions_v1" {
            warnings.push("relation_suggestions_v1 已识别，但 v0.1 当前不直接导入关系建议表，请先导入知识条目后生成关系建议。".to_string());
        }
        rows.iter()
            .map(|row| apply_scored_mapping(row, target_type, explicit_mapping, &suggestions))
            .collect()
    };

    ImportEngineOutput {
        detection,
        mapped_rows,
        mapping_suggestions: suggestions,
        direct_import_ready,
        warnings,
    }
}

pub fn score_mapping(
    rows: &[Map<String, Value>],
    target_type: &str,
) -> Vec<FieldMappingSuggestion> {
    let headers = sample_fields(rows);
    headers
        .into_iter()
        .map(|source| {
            let (target, confidence, reason) = score_field(&source, rows, target_type);
            let decision = if confidence >= 0.85 {
                "auto"
            } else if confidence >= 0.55 {
                "confirm"
            } else {
                "ignore"
            };
            FieldMappingSuggestion {
                source_field: source,
                target_field: target,
                confidence,
                decision: decision.to_string(),
                reason,
            }
        })
        .collect()
}

pub fn is_direct_import_type(detected_type: &str) -> bool {
    DIRECT_TYPES.contains(&detected_type)
}

fn detect_knowledge_items(
    lower_name: &str,
    fields: &HashSet<String>,
) -> Option<(&'static str, f64, &'static str)> {
    let has_type_name = has_all(fields, &["type", "name"]);
    let has_core = has_any(fields, &["content", "sourcenote", "detail"]);
    let alias_hits = count_any(
        fields,
        &[
            "type",
            "code",
            "name",
            "category",
            "summary",
            "content",
            "sourcenote",
            "tags",
            "datastatus",
            "detail",
        ],
    );
    if lower_name.contains("knowledge_items_import") || lower_name.contains("knowledgeitems") {
        return Some((
            "knowledge_items_v1",
            0.98,
            "文件名匹配 knowledge_items 导入包，且可按标准知识条目适配",
        ));
    }
    if has_type_name && has_core {
        return Some((
            "knowledge_items_v1",
            0.94,
            "样本同时包含 type/name 与 content/source_note/detail 等知识条目字段",
        ));
    }
    if alias_hits >= 5 {
        return Some((
            "knowledge_items_v1",
            0.86,
            "字段集合接近 knowledge_items 标准结构",
        ));
    }
    None
}

fn detect_classic_passages(
    lower_name: &str,
    fields: &HashSet<String>,
) -> Option<(&'static str, f64, &'static str)> {
    if lower_name.contains("classic_passages") {
        return Some((
            "classic_passages_v1",
            0.98,
            "文件名匹配 classic_passages，按原典条文适配",
        ));
    }
    if has_all(fields, &["worktitle", "originaltext", "sectiontitle"])
        || has_all(fields, &["classicid", "pagetitle", "originaltext"])
    {
        return Some((
            "classic_passages_v1",
            0.95,
            "样本包含 work_title/original_text/section_title 或 classic_id/page_title/original_text",
        ));
    }
    if has_all(fields, &["classicname", "originaltext"]) {
        return Some((
            "classic_passages_v1",
            0.85,
            "样本包含 classic_name 与 original_text",
        ));
    }
    None
}

fn detect_search_terms(
    lower_name: &str,
    fields: &HashSet<String>,
) -> Option<(&'static str, f64, &'static str)> {
    if lower_name.contains("search_terms") {
        return Some(("search_terms_v1", 0.98, "文件名匹配 search_terms"));
    }
    if has_all(fields, &["term", "termtype", "weight"]) || has_all(fields, &["itemname", "term"]) {
        return Some((
            "search_terms_v1",
            0.94,
            "样本包含搜索词 term/term_type/weight 结构",
        ));
    }
    None
}

fn detect_annotation_items(
    lower_name: &str,
    fields: &HashSet<String>,
) -> Option<(&'static str, f64, &'static str)> {
    if lower_name.contains("annotation_items_import") {
        return Some((
            "annotation_items_v1",
            0.99,
            "文件名匹配 annotation_items 注解导入包",
        ));
    }
    if has_all(fields, &["canonicalkey", "content"])
        && has_any(fields, &["annotationtype", "targettype", "sourcenote"])
    {
        return Some((
            "annotation_items_v1",
            0.95,
            "样本包含 canonical_key/content 与注解字段",
        ));
    }
    None
}

fn detect_standard_terms(fields: &HashSet<String>) -> Option<(&'static str, f64, &'static str)> {
    has_all(fields, &["termtype", "standardname", "aliases"]).then_some((
        "standard_terms_v1",
        0.94,
        "样本包含 standard_terms 标准词表字段",
    ))
}

fn detect_relation_suggestions(
    fields: &HashSet<String>,
) -> Option<(&'static str, f64, &'static str)> {
    if has_all(fields, &["sourcename", "targetname", "relationtype"])
        || has_all(fields, &["sourcetype", "targettype"])
    {
        Some((
            "relation_suggestions_v1",
            0.92,
            "样本包含 source/target/relation_type 关系建议字段",
        ))
    } else {
        None
    }
}

fn adapt_knowledge_item(raw: &Map<String, Value>) -> Map<String, Value> {
    let mut output = Map::new();
    copy_alias(raw, &mut output, "type", &["type", "类型", "知识类型"]);
    copy_alias(raw, &mut output, "code", &["code", "编号", "编码"]);
    copy_alias(
        raw,
        &mut output,
        "name",
        &["name", "名称", "标题", "条目名", "药名", "方名", "穴名"],
    );
    copy_alias(raw, &mut output, "category", &["category", "分类", "类别"]);
    copy_alias(raw, &mut output, "summary", &["summary", "摘要", "简介"]);
    copy_alias(
        raw,
        &mut output,
        "content",
        &["content", "正文", "原文", "条文", "original_text"],
    );
    copy_alias(
        raw,
        &mut output,
        "source_note",
        &["source_note", "source", "source_url", "出处", "来源"],
    );
    copy_alias(
        raw,
        &mut output,
        "tags",
        &["tags", "keywords", "标签", "关键词"],
    );
    copy_alias(raw, &mut output, "data_status", &["data_status"]);

    if let Some(detail_value) = get_alias(raw, &["detail", "详情", "扩展字段"]) {
        output.insert("detail".to_string(), normalize_detail_value(detail_value));
    }
    if let Some(Value::Object(detail)) = get_alias(raw, &["detail", "详情", "扩展字段"]) {
        merge_detail_into_output(&mut output, detail);
    }
    preserve_private_fields(raw, &mut output);
    preserve_context_fields(raw, &mut output);
    normalize_knowledge_item_defaults(&mut output);
    output
}

fn adapt_classic_passage(raw: &Map<String, Value>) -> Map<String, Value> {
    let work_title = text_alias(raw, &["work_title", "classic_name", "classic_title"])
        .unwrap_or_else(|| "原典".to_string());
    let section_title = text_alias(raw, &["section_title", "section", "title"])
        .or_else(|| text_alias(raw, &["page_title"]))
        .unwrap_or_else(|| "未命名条文".to_string());
    let page_title = text_alias(raw, &["page_title", "volume"]).unwrap_or_default();
    let original_text =
        text_alias(raw, &["original_text", "content", "正文", "原文"]).unwrap_or_default();
    let source_note =
        text_alias(raw, &["source_note", "source", "出处", "来源"]).unwrap_or_else(|| {
            [
                work_title.as_str(),
                page_title.as_str(),
                section_title.as_str(),
            ]
            .into_iter()
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" / ")
        });

    let mut output = Map::new();
    output.insert("type".to_string(), Value::String("syndrome".to_string()));
    output.insert(
        "name".to_string(),
        Value::String(format!("{} - {}", work_title, section_title)),
    );
    output.insert(
        "category".to_string(),
        Value::String(format!("原典 / {}", work_title)),
    );
    output.insert(
        "summary".to_string(),
        Value::String(truncate_summary(&original_text)),
    );
    output.insert("content".to_string(), Value::String(original_text.clone()));
    output.insert("source_note".to_string(), Value::String(source_note));
    output.insert(
        "data_status".to_string(),
        Value::String("validated".to_string()),
    );
    output.insert(
        "tags".to_string(),
        Value::String(
            ["原典", work_title.as_str(), section_title.as_str()]
                .into_iter()
                .filter(|part| !part.trim().is_empty())
                .collect::<Vec<_>>()
                .join(","),
        ),
    );
    output.insert("symptoms".to_string(), Value::String(original_text));
    output.insert("notes".to_string(), Value::String(detail_notes(raw)));
    output.insert("detail".to_string(), Value::Object(raw.clone()));
    preserve_private_fields(raw, &mut output);
    preserve_context_fields(raw, &mut output);
    normalize_knowledge_item_defaults(&mut output);
    output
}

fn adapt_annotation_item(raw: &Map<String, Value>) -> Map<String, Value> {
    let canonical_key = text_alias(raw, &["canonical_key", "canonicalKey"]).unwrap_or_default();
    let (target_type, target_name) = parse_canonical_key(&canonical_key);
    let mut output = Map::new();
    output.insert("type".to_string(), Value::String(target_type));
    output.insert("name".to_string(), Value::String(target_name));
    copy_alias(
        raw,
        &mut output,
        "content",
        &["content", "annotation", "annotation_text"],
    );
    copy_alias(raw, &mut output, "source_title", &["source_title", "title"]);
    copy_alias(
        raw,
        &mut output,
        "source_note",
        &["source_note", "source", "page_ref"],
    );
    copy_alias(raw, &mut output, "tags", &["tags", "keywords"]);
    output.insert(
        "category".to_string(),
        Value::String(text_alias(raw, &["category"]).unwrap_or_else(|| "人纪注解".to_string())),
    );
    output.insert(
        "summary".to_string(),
        Value::String(
            text(&output, "content")
                .map(|content| truncate_summary(&content))
                .unwrap_or_else(|| "人纪注解资料".to_string()),
        ),
    );
    output.insert(
        "data_status".to_string(),
        Value::String("imported".to_string()),
    );

    let mut detail = Map::new();
    for (key, value) in raw {
        detail.insert(key.clone(), value.clone());
    }
    detail.insert("canonical_key".to_string(), Value::String(canonical_key));
    output.insert("detail".to_string(), Value::Object(detail));
    preserve_private_fields(raw, &mut output);
    normalize_knowledge_item_defaults(&mut output);
    output
}

fn parse_canonical_key(canonical_key: &str) -> (String, String) {
    let mut parts = canonical_key.split(':').collect::<Vec<_>>();
    if parts.len() < 2 {
        return ("note".to_string(), canonical_key.trim().to_string());
    }
    let key_type = parts.remove(0);
    let item_type = match key_type {
        "herb" => "herb",
        "formula" => "formula",
        "acupoint" | "meridian" => "acupuncture",
        "classic_chapter" => "theory",
        "classic_passage" => "syndrome",
        _ => "note",
    };
    let name = match key_type {
        "classic_passage" if parts.len() >= 2 => parts[1].trim().to_string(),
        _ => parts.last().copied().unwrap_or_default().trim().to_string(),
    };
    (item_type.to_string(), name)
}

fn apply_scored_mapping(
    raw: &Map<String, Value>,
    target_type: &str,
    explicit_mapping: Option<&HashMap<String, String>>,
    suggestions: &[FieldMappingSuggestion],
) -> Map<String, Value> {
    let mut output = Map::new();
    let auto_mapping = suggestions
        .iter()
        .filter(|suggestion| suggestion.decision == "auto")
        .filter_map(|suggestion| {
            suggestion
                .target_field
                .as_ref()
                .map(|target| (suggestion.source_field.clone(), target.clone()))
        })
        .collect::<HashMap<_, _>>();
    let active = explicit_mapping.unwrap_or(&auto_mapping);

    for (source, value) in raw {
        if let Some(target) = active.get(source).or_else(|| alias_target(source)) {
            insert_target(&mut output, target, value.clone());
        }
    }
    output
        .entry("type".to_string())
        .or_insert_with(|| Value::String(target_type.to_string()));
    normalize_knowledge_item_defaults(&mut output);
    output
}

fn normalize_knowledge_item_defaults(output: &mut Map<String, Value>) {
    let original_type = text(output, "type").unwrap_or_else(|| "note".to_string());
    let normalized_type = normalize_item_type(&original_type);
    if normalized_type != original_type {
        let mut detail = output
            .get("detail")
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        if !detail.is_object() {
            detail = normalize_detail_value(&detail);
        }
        if let Some(map) = detail.as_object_mut() {
            map.insert(
                "import_warning".to_string(),
                Value::String(format!(
                    "原 type '{original_type}' 不在允许集合中，已按 {normalized_type} 导入。"
                )),
            );
            map.insert("original_type".to_string(), Value::String(original_type));
        }
        output.insert("detail".to_string(), detail);
    }
    output.insert("type".to_string(), Value::String(normalized_type));
    output
        .entry("alias".to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    output
        .entry("pinyin".to_string())
        .or_insert_with(|| Value::String(String::new()));
    output
        .entry("detail".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if text(output, "summary").is_none() {
        if let Some(content) = text(output, "content") {
            output.insert(
                "summary".to_string(),
                Value::String(truncate_summary(&content)),
            );
        }
    }
    if text(output, "tags").is_none() {
        let tags = [
            text(output, "name"),
            text(output, "category"),
            text(output, "type"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        output.insert("tags".to_string(), Value::String(tags.join(",")));
    }
    let status = text(output, "data_status").unwrap_or_else(|| "pending_review".to_string());
    let status = match status.as_str() {
        "pending_review" | "reviewed" | "needs_check" | "imported" | "validated" | "ready" => {
            status
        }
        _ => "pending_review".to_string(),
    };
    output.insert("data_status".to_string(), Value::String(status));
}

fn normalize_item_type(value: &str) -> String {
    match value {
        "herb" | "formula" | "acupuncture" | "syndrome" | "theory" | "note" => value.to_string(),
        "classic" => "theory".to_string(),
        "acupoint" | "meridian" => "acupuncture".to_string(),
        "unknown" | "mixed" | "auto" | "disease" => "note".to_string(),
        "中药" => "herb".to_string(),
        "方剂" => "formula".to_string(),
        "经络" | "穴位" => "acupuncture".to_string(),
        "证型" => "syndrome".to_string(),
        "理论" => "theory".to_string(),
        "笔记" | "病症" => "note".to_string(),
        _ => "note".to_string(),
    }
}

fn normalize_detail_value(value: &Value) -> Value {
    match value {
        Value::Object(_) => value.clone(),
        Value::String(text) => serde_json::from_str::<Value>(text)
            .unwrap_or_else(|_| serde_json::json!({ "raw_detail": text, "parse_error": true })),
        Value::Null => Value::Object(Map::new()),
        other => serde_json::json!({ "raw_detail": other }),
    }
}

fn score_field(
    source: &str,
    rows: &[Map<String, Value>],
    target_type: &str,
) -> (Option<String>, f64, String) {
    let normalized = normalize_header(source);
    let alias = alias_target(source).cloned();
    let candidates = candidate_fields(target_type);
    let mut best_target = alias.or_else(|| {
        candidates
            .iter()
            .max_by(|left, right| {
                string_similarity(&normalized, &normalize_header(left))
                    .partial_cmp(&string_similarity(&normalized, &normalize_header(right)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|value| (*value).to_string())
    });

    let mut name_score = if alias_target(source).is_some() {
        40.0
    } else {
        best_target
            .as_ref()
            .map(|target| string_similarity(&normalized, &normalize_header(target)) * 40.0)
            .unwrap_or_default()
    };
    if name_score < 18.0 {
        best_target = None;
    }

    let value_score = best_target
        .as_ref()
        .map(|target| value_pattern_score(source, target, rows) * 35.0)
        .unwrap_or_default();
    let context_score = best_target
        .as_ref()
        .map(|target| context_score(target, rows) * 15.0)
        .unwrap_or_default();
    let prior_score = best_target
        .as_ref()
        .map(|target| type_prior_score(target_type, target) * 10.0)
        .unwrap_or_default();

    if best_target.is_none() {
        name_score = 0.0;
    }
    let confidence = ((name_score + value_score + context_score + prior_score) / 100.0).min(1.0);
    let reason = format!(
        "字段名 {:.0}/40，值模式 {:.0}/35，上下文 {:.0}/15，类型先验 {:.0}/10",
        name_score, value_score, context_score, prior_score
    );
    (best_target, confidence, reason)
}

fn alias_target(source: &str) -> Option<&'static String> {
    lazy_static::lazy_static! {
        static ref ALIASES: HashMap<String, String> = {
            let mut map = HashMap::new();
            for (aliases, target) in [
                (&["名称", "name", "标题", "条目名", "药名", "方名", "穴名"][..], "name"),
                (&["编号", "code", "穴号", "经络编号", "编码"][..], "code"),
                (&["分类", "类别", "category", "所属经络", "经络"][..], "category"),
                (&["摘要", "summary", "简介"][..], "summary"),
                (&["原文", "正文", "content", "条文", "original_text"][..], "content"),
                (&["出处", "来源", "source", "source_note", "source_url"][..], "source_note"),
                (&["标签", "tags", "关键词", "keywords"][..], "tags"),
                (&["类型", "type", "知识类型"][..], "type"),
                (&["性味"][..], "detail.nature_flavor"),
                (&["归经"][..], "detail.meridians"),
                (&["功效"][..], "detail.effects"),
                (&["主治"][..], "detail.indications"),
                (&["禁忌"][..], "detail.contraindications"),
                (&["组成", "方药", "药物组成"][..], "detail.composition"),
                (&["用法"][..], "detail.usage"),
                (&["方解"][..], "detail.explanation"),
                (&["部位"][..], "detail.body_region"),
                (&["定位"][..], "detail.standard_location"),
                (&["取穴"][..], "detail.locating_method"),
                (&["注意事项"][..], "detail.precautions"),
            ] {
                for alias in aliases {
                    map.insert(normalize_header(alias), target.to_string());
                }
            }
            map
        };
    }
    ALIASES.get(&normalize_header(source))
}

fn insert_target(output: &mut Map<String, Value>, target: &str, value: Value) {
    let target = target.strip_prefix("detail.").unwrap_or(target);
    output.entry(target.to_string()).or_insert(value);
}

fn merge_detail_into_output(output: &mut Map<String, Value>, detail: &Map<String, Value>) {
    for (key, value) in detail {
        if let Some(target) = alias_target(key) {
            insert_target(output, target, value.clone());
        } else {
            insert_target(output, key, value.clone());
        }
    }
    let notes = detail_notes(detail);
    if !notes.is_empty() {
        output
            .entry("notes".to_string())
            .or_insert(Value::String(notes));
    }
}

fn preserve_context_fields(raw: &Map<String, Value>, output: &mut Map<String, Value>) {
    for field in [
        "classic_id",
        "page_title",
        "section_title",
        "source_url",
        "work_title",
    ] {
        if let Some(value) = raw.get(field) {
            output.entry(field.to_string()).or_insert(value.clone());
        }
    }
    let mut source_parts = text(output, "source_note")
        .map(|text| vec![text])
        .unwrap_or_default();
    for field in ["source_url", "classic_id", "page_title", "section_title"] {
        if let Some(value) = output.get(field).and_then(value_to_text) {
            if !source_parts.contains(&value) {
                source_parts.push(value);
            }
        }
    }
    if !source_parts.is_empty() {
        output.insert(
            "source_note".to_string(),
            Value::String(source_parts.join(" / ")),
        );
    }
}

fn preserve_private_fields(raw: &Map<String, Value>, output: &mut Map<String, Value>) {
    for (key, value) in raw {
        if key.starts_with('_') {
            output.insert(key.clone(), value.clone());
        }
    }
}

fn copy_alias(
    raw: &Map<String, Value>,
    output: &mut Map<String, Value>,
    target: &str,
    aliases: &[&str],
) {
    if let Some(value) = get_alias(raw, aliases) {
        output.insert(target.to_string(), value.clone());
    }
}

fn get_alias<'a>(raw: &'a Map<String, Value>, aliases: &[&str]) -> Option<&'a Value> {
    aliases.iter().find_map(|alias| {
        raw.iter()
            .find(|(key, _)| normalize_header(key) == normalize_header(alias))
            .map(|(_, value)| value)
    })
}

fn text_alias(raw: &Map<String, Value>, aliases: &[&str]) -> Option<String> {
    get_alias(raw, aliases).and_then(value_to_text)
}

fn text(object: &Map<String, Value>, field: &str) -> Option<String> {
    object.get(field).and_then(value_to_text)
}

fn value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Array(values) => {
            let text = values
                .iter()
                .filter_map(value_to_text)
                .collect::<Vec<_>>()
                .join(",");
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}

fn detail_notes(raw: &Map<String, Value>) -> String {
    let mut kept = Map::new();
    for field in [
        "classic_id",
        "page_title",
        "section_title",
        "source_url",
        "work_title",
        "volume",
        "annotation",
    ] {
        if let Some(value) = get_alias(raw, &[field]) {
            kept.insert(field.to_string(), value.clone());
        }
    }
    if kept.is_empty() {
        String::new()
    } else {
        serde_json::to_string(&Value::Object(kept)).unwrap_or_default()
    }
}

fn sample_fields(rows: &[Map<String, Value>]) -> Vec<String> {
    rows.iter()
        .take(20)
        .flat_map(|row| row.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn has_all(fields: &HashSet<String>, required: &[&str]) -> bool {
    required.iter().all(|field| fields.contains(*field))
}

fn has_any(fields: &HashSet<String>, required: &[&str]) -> bool {
    required.iter().any(|field| fields.contains(*field))
}

fn count_any(fields: &HashSet<String>, required: &[&str]) -> usize {
    required
        .iter()
        .filter(|field| fields.contains(**field))
        .count()
}

pub fn normalize_header(header: &str) -> String {
    header
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-', '_', '.', '/'], "")
}

fn truncate_summary(text: &str) -> String {
    text.chars().take(80).collect()
}

fn candidate_fields(target_type: &str) -> Vec<&'static str> {
    let mut fields = vec![
        "type",
        "code",
        "name",
        "category",
        "summary",
        "content",
        "source_note",
        "tags",
    ];
    match target_type {
        "中药" | "herb" => fields.extend([
            "detail.nature_flavor",
            "detail.meridians",
            "detail.effects",
            "detail.indications",
            "detail.contraindications",
        ]),
        "方剂" | "formula" => fields.extend([
            "detail.composition",
            "detail.usage",
            "detail.effects",
            "detail.indications",
            "detail.explanation",
        ]),
        "穴位" | "acupoint" => fields.extend([
            "detail.meridian_name",
            "detail.body_region",
            "detail.standard_location",
            "detail.locating_method",
            "detail.precautions",
        ]),
        _ => {}
    }
    fields
}

fn value_pattern_score(source: &str, target: &str, rows: &[Map<String, Value>]) -> f64 {
    let values = rows
        .iter()
        .take(20)
        .filter_map(|row| row.get(source).and_then(value_to_text))
        .collect::<Vec<_>>();
    if values.is_empty() {
        return 0.0;
    }
    let target = target.strip_prefix("detail.").unwrap_or(target);
    let hits = values
        .iter()
        .filter(|value| match target {
            "type" => matches!(
                value.as_str(),
                "中药"
                    | "方剂"
                    | "经络"
                    | "穴位"
                    | "证型"
                    | "病症"
                    | "herb"
                    | "formula"
                    | "meridian"
                    | "acupoint"
                    | "syndrome"
                    | "disease"
            ),
            "code" => value.chars().any(|ch| ch.is_ascii_digit()) && value.len() <= 40,
            "tags" => value.contains([',', '，', ';', '；', '、', '|']),
            "content" => value.chars().count() >= 20,
            "summary" => value.chars().count() <= 120,
            "source_note" => {
                value.contains('《')
                    || value.contains("http")
                    || value.contains("经")
                    || value.contains("论")
            }
            _ => !value.trim().is_empty(),
        })
        .count();
    hits as f64 / values.len() as f64
}

fn context_score(target: &str, rows: &[Map<String, Value>]) -> f64 {
    let fields = sample_fields(rows)
        .into_iter()
        .map(|field| normalize_header(&field))
        .collect::<HashSet<_>>();
    match target.strip_prefix("detail.").unwrap_or(target) {
        "name" => has_any(&fields, &["type", "content", "summary", "原文"]) as u8 as f64,
        "content" => has_any(&fields, &["name", "sourcenote", "originaltext"]) as u8 as f64,
        "source_note" => has_any(&fields, &["content", "originaltext", "worktitle"]) as u8 as f64,
        "tags" => has_any(&fields, &["name", "type", "content"]) as u8 as f64,
        _ => has_any(&fields, &["type", "name", "category"]) as u8 as f64,
    }
}

fn type_prior_score(target_type: &str, target: &str) -> f64 {
    let target = target.strip_prefix("detail.").unwrap_or(target);
    match (target_type, target) {
        ("中药" | "herb", "nature_flavor" | "meridians" | "effects" | "indications") => 1.0,
        ("方剂" | "formula", "composition" | "usage" | "explanation" | "indications") => 1.0,
        ("穴位" | "acupoint", "standard_location" | "locating_method" | "body_region") => 1.0,
        (_, "name" | "type" | "content" | "source_note" | "tags") => 0.8,
        _ => 0.4,
    }
}

fn string_similarity(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }
    let common = s1.chars().filter(|ch| s2.contains(*ch)).count();
    (common as f64 * 2.0) / (s1.chars().count() + s2.chars().count()) as f64
}

#[cfg(test)]
mod tests {
    use super::{detect_import_type, prepare_import_rows, score_mapping};
    use serde_json::{json, Map, Value};

    fn object(value: Value) -> Map<String, Value> {
        value.as_object().cloned().unwrap()
    }

    #[test]
    fn detects_knowledge_items_v1() {
        let rows = vec![object(json!({
            "type": "herb",
            "name": "桂枝",
            "content": "桂枝原文",
            "source_note": "神农本草经",
            "detail": {"effects": "温通"}
        }))];
        let detection = detect_import_type("knowledge_items_import_curated.json", "json", &rows);
        assert_eq!(detection.detected_type, "knowledge_items_v1");
        assert!(detection.confidence >= 0.9);
    }

    #[test]
    fn detects_classic_passages_v1() {
        let rows = vec![object(json!({
            "work_title": "黄帝内经·素问",
            "section_title": "上古天真论",
            "original_text": "昔在黄帝，生而神灵。"
        }))];
        let detection = detect_import_type("classic_passages_curated.json", "json", &rows);
        assert_eq!(detection.detected_type, "classic_passages_v1");
    }

    #[test]
    fn scores_csv_confidence_bands() {
        let rows = vec![object(json!({
            "名称": "足三里",
            "原文": "太阳之为病，脉浮，头项强痛而恶寒。此为经典条文原文内容，用于测试正文识别。",
            "随机列": "x"
        }))];
        let suggestions = score_mapping(&rows, "mixed");
        let name = suggestions
            .iter()
            .find(|item| item.source_field == "名称")
            .unwrap();
        let content = suggestions
            .iter()
            .find(|item| item.source_field == "原文")
            .unwrap();
        let random = suggestions
            .iter()
            .find(|item| item.source_field == "随机列")
            .unwrap();
        assert_eq!(name.decision, "auto");
        assert!(matches!(content.decision.as_str(), "auto" | "confirm"));
        assert_eq!(random.decision, "ignore");
    }

    #[test]
    fn adapts_tags_array_and_detail_object() {
        let rows = vec![object(json!({
            "type": "herb",
            "name": "桂枝",
            "content": "桂枝，味辛温。",
            "tags": ["原典", "神农本草经"],
            "detail": {"nature_flavor": "味辛温", "effects": "发表"}
        }))];
        let output = prepare_import_rows(
            "knowledge_items_import_curated.json",
            "json",
            "mixed",
            &rows,
            None,
        );
        assert!(output.direct_import_ready);
        assert_eq!(
            output.mapped_rows[0].get("tags").unwrap(),
            &json!(["原典", "神农本草经"])
        );
        assert_eq!(
            output.mapped_rows[0].get("nature_flavor").unwrap(),
            "味辛温"
        );
    }
}
