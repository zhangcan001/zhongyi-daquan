use crate::db::connection::Database;
use crate::errors::AppResult;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

const FORMULA_SYSTEM_PROMPT: &str = "你可以展示本地知识库中记载的经方原方组成、原文剂量、药材比例和原文煎服法。这些内容应明确标注为“本地资料原方信息”或“古籍/讲义原文记录”。你不能把这些内容改写成针对用户个人的直接服用指令。你不能自行换算现代剂量。如果用户要求“给我具体吃多少克、吃几天”，你应说明可以提供资料中的原方组成和比例，但个人剂量、加减和疗程需要专业中医师确认。如果本地资料没有方剂组成，不得编造。回答必须引用本地资料来源。";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormulaAiRequest {
    pub question: String,
    pub related_item_id: Option<i64>,
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormulaAiAnswer {
    pub enabled: bool,
    pub status: String,
    pub message: String,
    pub system_prompt: String,
    pub formula_cards: Vec<FormulaCard>,
    pub retrieval_scope: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormulaCard {
    pub formula_name: String,
    pub item_id: Option<i64>,
    pub related_pattern: Option<String>,
    pub composition: Option<String>,
    pub original_dosage: Option<String>,
    pub ratio: Option<String>,
    pub usage: Option<String>,
    pub decoction_method: Option<String>,
    pub original_text: Option<String>,
    pub indications: Option<String>,
    pub explanation: Option<String>,
    pub contraindications: Option<String>,
    pub annotation_snippets: Vec<String>,
    pub sources: Vec<FormulaSource>,
    pub missing_composition: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormulaSource {
    pub title: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
struct FormulaCandidate {
    item_id: i64,
    score: i64,
}

pub fn answer_formula_question(
    database: &Database,
    request: FormulaAiRequest,
) -> AppResult<FormulaAiAnswer> {
    let question = request.question.trim().to_string();
    let candidates = find_formula_candidates(database, &question, request.related_item_id)?;
    let mut cards = Vec::new();
    for candidate in candidates.into_iter().take(6) {
        if let Some(card) = build_formula_card(database, candidate.item_id)? {
            cards.push(card);
        }
    }

    let message = build_answer_markdown(&question, &cards, request.mode.as_deref());
    Ok(FormulaAiAnswer {
        enabled: true,
        status: "local_rag_ready".to_string(),
        message,
        system_prompt: FORMULA_SYSTEM_PROMPT.to_string(),
        formula_cards: cards,
        retrieval_scope: vec![
            "knowledge_items.type = formula".to_string(),
            "formula_details".to_string(),
            "knowledge_items.detail JSON".to_string(),
            "knowledge_annotations.content/detail_json/tags_json".to_string(),
            "knowledge_items 中原典条文、证候、注解、正文、source_note、tags".to_string(),
        ],
    })
}

fn find_formula_candidates(
    database: &Database,
    question: &str,
    related_item_id: Option<i64>,
) -> AppResult<Vec<FormulaCandidate>> {
    database.with_connection(|connection| {
        let mut scores: BTreeMap<i64, i64> = BTreeMap::new();
        if let Some(item_id) = related_item_id {
            let related = connection
                .query_row(
                    "SELECT type, name, content, summary, detail FROM knowledge_items WHERE id = ?1",
                    params![item_id],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, Option<String>>(2)?,
                            row.get::<_, Option<String>>(3)?,
                            row.get::<_, Option<String>>(4)?,
                        ))
                    },
                )
                .optional()?;
            if let Some((item_type, name, content, summary, detail)) = related {
                if item_type == "formula" {
                    *scores.entry(item_id).or_default() += 200;
                }
                for name in formula_names_from_text(&[question, &name, opt(&content), opt(&summary), opt(&detail)].join("\n")) {
                    add_name_hits(connection, &name, 120, &mut scores)?;
                }
            }
        }

        for name in formula_names_from_text(question) {
            add_name_hits(connection, &name, 160, &mut scores)?;
        }

        for term in query_terms(question) {
            let like = format!("%{}%", term);
            let mut statement = connection.prepare(
                "SELECT id, type
                 FROM knowledge_items
                 WHERE type = 'formula'
                   AND (name LIKE ?1 OR alias LIKE ?1 OR category LIKE ?1 OR summary LIKE ?1
                        OR content LIKE ?1 OR source_note LIKE ?1 OR tags LIKE ?1 OR detail LIKE ?1)
                 LIMIT 80",
            )?;
            let rows = statement.query_map(params![like], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?;
            for row in rows {
                let (item_id, item_type) = row?;
                *scores.entry(item_id).or_default() += if item_type == "formula" { 90 } else { 30 };
            }

            let mut broad = connection.prepare(
                "SELECT id, type, name, content, summary, detail
                 FROM knowledge_items
                 WHERE type != 'formula'
                   AND (name LIKE ?1 OR summary LIKE ?1 OR content LIKE ?1 OR source_note LIKE ?1 OR tags LIKE ?1 OR detail LIKE ?1)
                 LIMIT 80",
            )?;
            let rows = broad.query_map(params![like], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?;
            for row in rows {
                let (_, _, name, content, summary, detail) = row?;
                let text = [name.as_str(), opt(&content), opt(&summary), opt(&detail)].join("\n");
                for formula_name in formula_names_from_text(&text) {
                    add_name_hits(connection, &formula_name, 70, &mut scores)?;
                }
            }

            let mut annotations = connection.prepare(
                "SELECT knowledge_item_id, content, source_title, source_note, tags_json, detail_json
                 FROM knowledge_annotations
                 WHERE content LIKE ?1 OR source_title LIKE ?1 OR source_note LIKE ?1 OR tags_json LIKE ?1 OR detail_json LIKE ?1
                 LIMIT 120",
            )?;
            let rows = annotations.query_map(params![like], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })?;
            for row in rows {
                let (item_id, content, title, note, tags, detail) = row?;
                let item_type: Option<String> = connection
                    .query_row(
                        "SELECT type FROM knowledge_items WHERE id = ?1",
                        params![item_id],
                        |row| row.get(0),
                    )
                    .optional()?;
                if item_type.as_deref() == Some("formula") {
                    *scores.entry(item_id).or_default() += 90;
                }
                let text = [
                    content.as_str(),
                    opt(&title),
                    opt(&note),
                    opt(&tags),
                    opt(&detail),
                ]
                .join("\n");
                for formula_name in formula_names_from_text(&text) {
                    add_name_hits(connection, &formula_name, 65, &mut scores)?;
                }
            }
        }

        let mut candidates = scores
            .into_iter()
            .map(|(item_id, score)| FormulaCandidate { item_id, score })
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| b.score.cmp(&a.score).then(a.item_id.cmp(&b.item_id)));
        Ok(candidates)
    })
}

fn add_name_hits(
    connection: &rusqlite::Connection,
    formula_name: &str,
    score: i64,
    scores: &mut BTreeMap<i64, i64>,
) -> AppResult<()> {
    let like = format!("%{}%", formula_name.trim());
    let mut statement = connection.prepare(
        "SELECT id FROM knowledge_items
         WHERE type = 'formula'
           AND (name = ?1 OR name LIKE ?2 OR alias LIKE ?2 OR content LIKE ?2 OR detail LIKE ?2)
         LIMIT 20",
    )?;
    let rows = statement.query_map(params![formula_name.trim(), like], |row| row.get(0))?;
    for row in rows {
        *scores.entry(row?).or_default() += score;
    }
    Ok(())
}

fn build_formula_card(database: &Database, item_id: i64) -> AppResult<Option<FormulaCard>> {
    database.with_connection(|connection| {
        let item = connection
            .query_row(
                "SELECT id, name, category, summary, content, source_note, tags, detail, source_package
                 FROM knowledge_items
                 WHERE id = ?1 AND type = 'formula'",
                params![item_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                    ))
                },
            )
            .optional()?;
        let Some((item_id, name, category, summary, content, source_note, _tags, detail_json, source_package)) = item else {
            return Ok(None);
        };
        let mut detail = parse_json_object(detail_json.as_deref());
        merge_formula_table_detail(connection, item_id, &mut detail)?;
        let annotations = load_annotations(connection, item_id)?;

        let composition = pick_text(
            &detail,
            &[
                "composition",
                "ingredients",
                "originalFormula",
                "original_formula",
                "formula_composition",
            ],
        )
        .or_else(|| extract_labeled_text(content.as_deref(), &["组成", "原方组成", "本地资料原方组成"]))
        .or_else(|| extract_from_annotations(&annotations, &["组成", "原方组成"]));
        let original_dosage = pick_text(&detail, &["originalDosage", "original_dosage", "dosage"])
            .or_else(|| composition.clone());
        let usage = pick_text(&detail, &["usage", "administration", "preparation"])
            .or_else(|| extract_labeled_text(content.as_deref(), &["煎服法", "服法", "用法"]));
        let decoction_method = pick_text(&detail, &["decoctionMethod", "decoction_method", "preparation"])
            .or_else(|| extract_labeled_text(content.as_deref(), &["煎服法", "煎法"]));
        let original_text = pick_text(
            &detail,
            &["originalText", "original_text", "sourceText", "source_text", "classic_original"],
        )
        .or_else(|| content.clone());
        let indications = pick_text(&detail, &["indications", "pattern", "symptoms"])
            .or_else(|| summary.clone());
        let explanation = pick_text(&detail, &["explanation", "niNote", "ni_note", "notes"])
            .or_else(|| extract_from_annotations(&annotations, &["方义", "注解", "倪注"]));
        let contraindications = pick_text(&detail, &["contraindications", "precautions"]);
        let ratio = composition.as_deref().and_then(extract_formula_ratio);
        let annotation_snippets = annotations
            .iter()
            .take(4)
            .map(|annotation| truncate(&annotation.content, 150))
            .collect::<Vec<_>>();
        let mut sources = Vec::new();
        sources.push(FormulaSource {
            title: source_package.clone().or_else(|| category.clone()),
            note: source_note.clone(),
        });
        for annotation in &annotations {
            sources.push(FormulaSource {
                title: annotation.source_title.clone(),
                note: annotation.source_note.clone(),
            });
        }
        sources.retain(|source| {
            source
                .title
                .as_deref()
                .unwrap_or_default()
                .trim()
                .len()
                + source.note.as_deref().unwrap_or_default().trim().len()
                > 0
        });
        dedupe_sources(&mut sources);

        Ok(Some(FormulaCard {
            formula_name: name,
            item_id: Some(item_id),
            related_pattern: pick_text(&detail, &["pattern", "clauseNo", "clause_no"]).or(category),
            composition: composition.clone(),
            original_dosage,
            ratio,
            usage,
            decoction_method,
            original_text,
            indications,
            explanation,
            contraindications,
            annotation_snippets,
            sources,
            missing_composition: composition
                .as_deref()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true),
        }))
    })
}

fn merge_formula_table_detail(
    connection: &rusqlite::Connection,
    item_id: i64,
    detail: &mut serde_json::Map<String, Value>,
) -> AppResult<()> {
    let table_detail = connection
        .query_row(
            "SELECT source_text, composition, usage, effects, indications, explanation, modifications, contraindications, notes
             FROM formula_details WHERE item_id = ?1",
            params![item_id],
            |row| {
                let keys = [
                    "sourceText",
                    "composition",
                    "usage",
                    "effects",
                    "indications",
                    "explanation",
                    "modifications",
                    "contraindications",
                    "notes",
                ];
                let mut map = serde_json::Map::new();
                for (index, key) in keys.iter().enumerate() {
                    let value: Option<String> = row.get(index)?;
                    if let Some(value) = value.filter(|text| !text.trim().is_empty()) {
                        map.insert((*key).to_string(), Value::String(value));
                    }
                }
                Ok(map)
            },
        )
        .optional()?;
    if let Some(table_detail) = table_detail {
        for (key, value) in table_detail {
            detail.entry(key).or_insert(value);
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct AnnotationText {
    source_title: Option<String>,
    source_note: Option<String>,
    content: String,
}

fn load_annotations(
    connection: &rusqlite::Connection,
    item_id: i64,
) -> AppResult<Vec<AnnotationText>> {
    let mut statement = connection.prepare(
        "SELECT source_title, source_note, content
         FROM knowledge_annotations
         WHERE knowledge_item_id = ?1
         ORDER BY created_at DESC, id DESC
         LIMIT 8",
    )?;
    let rows = statement.query_map(params![item_id], |row| {
        Ok(AnnotationText {
            source_title: row.get(0)?,
            source_note: row.get(1)?,
            content: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn extract_formula_ratio(composition_text: &str) -> Option<String> {
    let entries = parse_composition_entries(composition_text);
    if entries.len() < 2 {
        return None;
    }
    let mut by_unit: BTreeMap<String, Vec<(String, f64, String)>> = BTreeMap::new();
    for entry in entries {
        by_unit.entry(entry.unit.clone()).or_default().push((
            entry.name,
            entry.amount,
            entry.original,
        ));
    }
    let mut comparable = by_unit
        .iter()
        .filter(|(_, values)| values.len() >= 2)
        .collect::<Vec<_>>();
    comparable.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    let (_, values) = comparable.first()?;
    let names = values
        .iter()
        .map(|(name, _, _)| name.clone())
        .collect::<Vec<_>>()
        .join(" : ");
    let amounts = values
        .iter()
        .map(|(_, amount, _)| format_amount(*amount))
        .collect::<Vec<_>>()
        .join(" : ");
    let ratio = format!("{names} = {amounts}");
    let comparable_names = values
        .iter()
        .map(|(name, _, _)| name.as_str())
        .collect::<HashSet<_>>();
    let extras = by_unit
        .values()
        .flat_map(|values| values.iter())
        .filter(|(name, _, _)| !comparable_names.contains(name.as_str()))
        .map(|(_, _, original)| format!("{original}另计"))
        .collect::<Vec<_>>();
    if extras.is_empty() {
        Some(ratio)
    } else {
        Some(format!("{ratio}；{}。", extras.join("，")))
    }
}

#[derive(Debug)]
struct CompositionEntry {
    name: String,
    amount: f64,
    unit: String,
    original: String,
}

fn parse_composition_entries(text: &str) -> Vec<CompositionEntry> {
    let normalized = text.replace(
        ['：', ':', '，', ',', '、', ';', '；', '\n', '\r', '\t'],
        " ",
    );
    let chars = normalized.chars().collect::<Vec<_>>();
    let units = ['两', '枚', '升', '合', '钱', '分'];
    let mut entries = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < chars.len() {
        if units.contains(&chars[index]) {
            let unit_index = index;
            let mut num_start = unit_index;
            while num_start > start && is_chinese_number(chars[num_start - 1]) {
                num_start -= 1;
            }
            if num_start < unit_index {
                let mut name = chars[start..num_start]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .trim_matches(|ch: char| ch.is_ascii_punctuation() || ch.is_whitespace())
                    .to_string();
                if let Some(last_space) = name.rfind(' ') {
                    name = name[last_space + 1..].trim().to_string();
                }
                let number = chars[num_start..unit_index].iter().collect::<String>();
                if !name.is_empty() {
                    if let Some(amount) = chinese_number_to_f64(&number) {
                        let unit = chars[unit_index].to_string();
                        entries.push(CompositionEntry {
                            name: clean_herb_name(&name),
                            amount,
                            unit,
                            original: chars[start..=unit_index]
                                .iter()
                                .collect::<String>()
                                .trim()
                                .to_string(),
                        });
                    }
                }
                start = unit_index + 1;
            }
        }
        index += 1;
    }
    entries
}

fn is_chinese_number(ch: char) -> bool {
    matches!(
        ch,
        '一' | '二' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '十' | '半'
    )
}

fn chinese_number_to_f64(text: &str) -> Option<f64> {
    if text == "半" {
        return Some(0.5);
    }
    if let Some(prefix) = text.strip_suffix("半") {
        return chinese_number_to_f64(prefix).map(|value| value + 0.5);
    }
    if text == "十" {
        return Some(10.0);
    }
    if let Some((left, right)) = text.split_once('十') {
        let tens = if left.is_empty() {
            1.0
        } else {
            single_chinese_digit(left)? as f64
        };
        let ones = if right.is_empty() {
            0.0
        } else {
            single_chinese_digit(right)? as f64
        };
        return Some(tens * 10.0 + ones);
    }
    single_chinese_digit(text).map(|value| value as f64)
}

fn single_chinese_digit(text: &str) -> Option<i64> {
    match text {
        "一" => Some(1),
        "二" => Some(2),
        "三" => Some(3),
        "四" => Some(4),
        "五" => Some(5),
        "六" => Some(6),
        "七" => Some(7),
        "八" => Some(8),
        "九" => Some(9),
        _ => None,
    }
}

fn format_amount(value: f64) -> String {
    if (value.fract() - 0.0).abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        format!("{value:.1}")
    }
}

fn clean_herb_name(name: &str) -> String {
    name.trim()
        .trim_matches(|ch: char| ch == '*' || ch == '-' || ch == '：' || ch == ':')
        .to_string()
}

fn build_answer_markdown(question: &str, cards: &[FormulaCard], _mode: Option<&str>) -> String {
    let mut lines = vec![
        "本回答基于本地知识库检索，展示的是本地资料原方信息和古籍/讲义原文记录。".to_string(),
        format!("问题：{question}"),
        String::new(),
    ];
    let personal_dose_request =
        question.contains('克') || question.contains("吃几天") || question.contains("直接告诉我");
    if personal_dose_request {
        lines.push("剂量边界：可以展示本地资料中的原文剂量和比例，但不直接给个人服用剂量、加减和疗程；实际用药需由专业中医师结合面诊确认。".to_string());
        lines.push(String::new());
    }
    if cards.is_empty() {
        lines.push("本地资料中未检索到完整组成。".to_string());
        lines.push("提示：没有来源支撑的方剂组成不会编造。".to_string());
        return lines.join("\n");
    }
    for card in cards {
        lines.push(format!("方剂：{}", card.formula_name));
        lines.push(String::new());
        lines.push("本地资料原方组成：".to_string());
        if let Some(composition) = &card.composition {
            lines.push(format_bullet_block(composition));
        } else {
            lines.push("本地资料中未检索到完整组成。".to_string());
        }
        if let Some(ratio) = &card.ratio {
            lines.push(String::new());
            lines.push("药材比例：".to_string());
            lines.push(ratio.clone());
        }
        if let Some(usage) = card.decoction_method.as_ref().or(card.usage.as_ref()) {
            lines.push(String::new());
            lines.push("原文煎服法：".to_string());
            lines.push(usage.clone());
        }
        if let Some(indications) = &card.indications {
            lines.push(String::new());
            lines.push("适用描述 / 关联证候：".to_string());
            lines.push(indications.clone());
        }
        if let Some(explanation) = &card.explanation {
            lines.push(String::new());
            lines.push("本地注解摘要：".to_string());
            lines.push(truncate(explanation, 220));
        }
        if let Some(contraindications) = &card.contraindications {
            lines.push(String::new());
            lines.push("谨慎或不适用情况：".to_string());
            lines.push(contraindications.clone());
        }
        lines.push(String::new());
        lines.push("资料依据：".to_string());
        if card.sources.is_empty() {
            lines.push("* 未记录来源；该卡片仅可作为待补来源资料。".to_string());
        } else {
            for source in &card.sources {
                lines.push(format!(
                    "* {}",
                    [source.title.as_deref(), source.note.as_deref()]
                        .into_iter()
                        .flatten()
                        .filter(|value| !value.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join("｜")
                ));
            }
        }
        lines.push("---".to_string());
    }
    lines.join("\n")
}

fn format_bullet_block(text: &str) -> String {
    let parts = text
        .split(['\n', '，', ',', '、', ';', '；'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if parts.len() >= 2 {
        parts
            .into_iter()
            .map(|part| format!("* {part}"))
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        text.to_string()
    }
}

fn formula_names_from_text(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let chars = text.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().enumerate() {
        if matches!(ch, '汤' | '丸' | '散' | '方' | '饮' | '剂') {
            let start = index.saturating_sub(8);
            let window = &chars[start..=index];
            let split = window.iter().rposition(|c| {
                c.is_whitespace()
                    || matches!(
                        *c,
                        '，' | '。'
                            | '、'
                            | '；'
                            | ';'
                            | ':'
                            | '：'
                            | '“'
                            | '”'
                            | '"'
                            | '\''
                            | '？'
                            | '?'
                            | '！'
                            | '!'
                    )
            });
            let name_start = split.map(|idx| idx + 1).unwrap_or(0);
            let name = window[name_start..].iter().collect::<String>();
            let name = name.trim().to_string();
            if name.chars().count() >= 2 && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

fn query_terms(question: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let full = question.trim();
    if !full.is_empty() {
        terms.push(full.to_string());
    }
    for token in full.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '，' | '。' | '、' | '；' | ';' | ':' | '：' | '？' | '?' | '！' | '!'
            )
    }) {
        let token = token
            .trim_matches(|ch: char| {
                matches!(
                    ch,
                    '有' | '哪'
                        | '些'
                        | '可'
                        | '以'
                        | '参'
                        | '考'
                        | '什'
                        | '么'
                        | '方'
                        | '向'
                        | '的'
                )
            })
            .trim();
        if token.chars().count() >= 2 && !terms.iter().any(|term| term == token) {
            terms.push(token.to_string());
        }
    }
    for marker in [
        "太阳病",
        "少阳病",
        "阳明病",
        "太阴病",
        "少阴病",
        "厥阴病",
        "上热下寒",
    ] {
        if full.contains(marker) && !terms.iter().any(|term| term == marker) {
            terms.push(marker.to_string());
        }
    }
    for formula_name in formula_names_from_text(full) {
        if !terms.iter().any(|term| term == &formula_name) {
            terms.push(formula_name);
        }
    }
    terms
}

fn parse_json_object(text: Option<&str>) -> serde_json::Map<String, Value> {
    text.and_then(|value| serde_json::from_str::<Value>(value).ok())
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn pick_text(detail: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = detail
            .get(*key)
            .or_else(|| detail.get(&snake_to_camel(key)))
        {
            if let Some(text) = json_text(value) {
                if !text.trim().is_empty() {
                    return Some(text.trim().to_string());
                }
            }
        }
    }
    None
}

fn json_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => Some(
            items
                .iter()
                .filter_map(json_text)
                .collect::<Vec<_>>()
                .join("，"),
        ),
        Value::Object(_) => Some(value.to_string()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

fn extract_labeled_text(text: Option<&str>, labels: &[&str]) -> Option<String> {
    let text = text?;
    for label in labels {
        if let Some(index) = text.find(label) {
            let tail = &text[index + label.len()..];
            let tail = tail.trim_start_matches(['：', ':', ' ', '\n', '\r']);
            let end = tail
                .find(|ch| matches!(ch, '\n' | '。'))
                .unwrap_or(tail.len());
            let value = tail[..end].trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn extract_from_annotations(annotations: &[AnnotationText], labels: &[&str]) -> Option<String> {
    annotations
        .iter()
        .find_map(|annotation| extract_labeled_text(Some(&annotation.content), labels))
}

fn dedupe_sources(sources: &mut Vec<FormulaSource>) {
    let mut seen = HashSet::new();
    sources.retain(|source| {
        let key = format!(
            "{}|{}",
            source.title.as_deref().unwrap_or_default(),
            source.note.as_deref().unwrap_or_default()
        );
        seen.insert(key)
    });
}

fn snake_to_camel(key: &str) -> String {
    let mut result = String::new();
    let mut upper = false;
    for ch in key.chars() {
        if ch == '_' {
            upper = true;
        } else if upper {
            result.extend(ch.to_uppercase());
            upper = false;
        } else {
            result.push(ch);
        }
    }
    result
}

fn opt(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or_default()
}

fn truncate(value: &str, limit: usize) -> String {
    let mut output = value.chars().take(limit).collect::<String>();
    if value.chars().count() > limit {
        output.push_str("...");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn ratio_extracts_same_unit_and_keeps_other_units_separate() {
        let ratio =
            extract_formula_ratio("桂枝三两 芍药三两 甘草二两 生姜三两 大枣十二枚").expect("ratio");
        assert!(ratio.contains("桂枝 : 芍药 : 甘草 : 生姜 = 3 : 3 : 2 : 3"));
        assert!(ratio.contains("大枣十二枚另计"));
    }

    #[test]
    fn guizhi_formula_answer_includes_composition_ratio_sources_and_safety_boundary() {
        let data_dir = temp_data_dir("ai-formula-guizhi");
        let database = Database::initialize(&data_dir).expect("database initializes");
        seed_guizhi_tang(&database);

        let response = answer_formula_question(
            &database,
            FormulaAiRequest {
                question: "桂枝汤组成是什么？".to_string(),
                related_item_id: None,
                mode: None,
            },
        )
        .expect("answer");
        assert_eq!(response.formula_cards.len(), 1);
        let card = &response.formula_cards[0];
        assert!(card
            .composition
            .as_deref()
            .unwrap_or_default()
            .contains("桂枝三两"));
        assert!(card
            .ratio
            .as_deref()
            .unwrap_or_default()
            .contains("桂枝 : 芍药 : 甘草 : 生姜 = 3 : 3 : 2 : 3"));
        assert!(response.message.contains("资料依据"));
        assert!(response.message.contains("4人纪-伤寒论.pdf"));

        let dose_response = answer_formula_question(
            &database,
            FormulaAiRequest {
                question: "直接告诉我每味多少克吃几天".to_string(),
                related_item_id: card.item_id,
                mode: None,
            },
        )
        .expect("dose answer");
        assert!(dose_response.message.contains("不直接给个人服用剂量"));

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn syndrome_question_returns_formula_candidates_with_composition() {
        let data_dir = temp_data_dir("ai-formula-syndrome");
        let database = Database::initialize(&data_dir).expect("database initializes");
        seed_guizhi_tang(&database);
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, summary, content, source_note, tags, data_status, completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES ('syndrome', '太阳病条文', '太阳病可参考桂枝汤', '太阳病，发热汗出，桂枝汤主之。', '4人纪-伤寒论.pdf｜PDF页码10', '太阳病,桂枝汤', 'imported', 'complete', 1, 0, '{}', datetime('now'), datetime('now'))",
                    [],
                )?;
                Ok(())
            })
            .unwrap();

        let response = answer_formula_question(
            &database,
            FormulaAiRequest {
                question: "太阳病可以参考哪些方？".to_string(),
                related_item_id: None,
                mode: None,
            },
        )
        .expect("answer");
        assert!(response
            .formula_cards
            .iter()
            .any(|card| card.formula_name == "桂枝汤"));

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn upper_heat_lower_cold_returns_candidates_without_personal_dosing() {
        let data_dir = temp_data_dir("ai-formula-upper-lower");
        let database = Database::initialize(&data_dir).expect("database initializes");
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, category, summary, content, source_note, tags, data_status, completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES ('formula', '乌梅丸', '伤寒论', '厥阴病，上热下寒可见相关讨论。', '乌梅丸原方资料。', '4人纪-伤寒论.pdf｜PDF页码88', '经方,上热下寒', 'imported', 'complete', 1, 0, ?1, datetime('now'), datetime('now'))",
                    params![serde_json::json!({
                        "composition": "乌梅三百枚 细辛六两 干姜十两 黄连十六两 当归四两 附子六两 蜀椒四两 桂枝六两 人参六两 黄柏六两",
                        "usage": "本地资料记录为丸剂原文服法，按原文展示。",
                        "indications": "厥阴病相关条文方向。"
                    }).to_string()],
                )?;
                let formula_id = connection.last_insert_rowid();
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, summary, content, source_note, tags, data_status, completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES ('note', '上热下寒经方方向', '上热下寒可参考乌梅丸方向。', '讲义注解：上热下寒讨论中常提及乌梅丸。', '讲义摘录｜PDF页码90', '上热下寒,乌梅丸', 'imported', 'complete', 1, 0, '{}', datetime('now'), datetime('now'))",
                    [],
                )?;
                connection.execute(
                    "INSERT INTO knowledge_annotations
                     (knowledge_item_id, annotation_type, source_title, source_note, content, detail_json, tags_json, created_at, updated_at)
                     VALUES (?1, 'source_annotation', '4人纪-伤寒论.pdf', 'PDF页码88-90', '乌梅丸注解摘要：用于厥阴寒热错杂相关学习。', '{}', '上热下寒,乌梅丸', datetime('now'), datetime('now'))",
                    params![formula_id],
                )?;
                Ok(())
            })
            .unwrap();

        let response = answer_formula_question(
            &database,
            FormulaAiRequest {
                question: "上热下寒有哪些经方方向？".to_string(),
                related_item_id: None,
                mode: None,
            },
        )
        .expect("answer");
        assert!(response
            .formula_cards
            .iter()
            .any(|card| card.formula_name == "乌梅丸"
                && card
                    .composition
                    .as_deref()
                    .unwrap_or_default()
                    .contains("乌梅三百枚")));
        assert!(response.message.contains("乌梅丸"));

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn missing_composition_is_not_fabricated() {
        let data_dir = temp_data_dir("ai-formula-missing");
        let database = Database::initialize(&data_dir).expect("database initializes");
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, summary, content, source_note, tags, data_status, completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES ('formula', '未知方', '仅有方名', '未知方用于测试。', '测试来源', '方剂', 'imported', 'partial', 1, 0, '{}', datetime('now'), datetime('now'))",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let response = answer_formula_question(
            &database,
            FormulaAiRequest {
                question: "未知方组成是什么？".to_string(),
                related_item_id: None,
                mode: None,
            },
        )
        .expect("answer");
        assert!(response.formula_cards[0].missing_composition);
        assert!(response.message.contains("本地资料中未检索到完整组成"));

        let _ = fs::remove_dir_all(data_dir);
    }

    fn seed_guizhi_tang(database: &Database) -> i64 {
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, category, summary, content, source_note, tags, data_status, completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES ('formula', '桂枝汤', '伤寒论', '太阳中风，营卫不和。', '桂枝汤原文。', '辨太阳病脉证并治法上｜PDF页码12', '经方,太阳病', 'imported', 'complete', 1, 0, ?1, datetime('now'), datetime('now'))",
                    params![serde_json::json!({
                        "composition": "桂枝三两 芍药三两 甘草二两 生姜三两 大枣十二枚",
                        "usage": "上五味，以水七升，微火煮取三升，去滓，适寒温，服一升。",
                        "indications": "太阳病，头痛，发热，汗出，恶风。",
                        "explanation": "本地注解：调和营卫。"
                    }).to_string()],
                )?;
                let item_id = connection.last_insert_rowid();
                connection.execute(
                    "INSERT INTO knowledge_annotations
                     (knowledge_item_id, annotation_type, source_title, source_note, content, detail_json, tags_json, created_at, updated_at)
                     VALUES (?1, 'source_annotation', '4人纪-伤寒论.pdf', '辨太阳病脉证并治法上｜PDF页码12', '桂枝汤注解：桂枝芍药等量，甘草二两。', '{}', '桂枝汤,倪注', datetime('now'), datetime('now'))",
                    params![item_id],
                )?;
                Ok(item_id)
            })
            .unwrap()
    }

    fn temp_data_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("zhongyi-daquan-{test_name}-{unique}"))
    }
}
