use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::relation::{DuplicateCandidateDetail, KnowledgeFingerprint};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct DuplicateInput {
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
    pub detail: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct MergeResult {
    pub existing_item_id: i64,
    pub created_item_id: Option<i64>,
    pub merge_record_id: Option<i64>,
    pub status: String,
}

#[derive(Debug, Clone)]
struct ItemFingerprintRow {
    item_id: i64,
    item_type: String,
    code_norm: Option<String>,
    name_norm: Option<String>,
    pinyin_norm: Option<String>,
    alias_norm: Option<String>,
    category_norm: Option<String>,
    fingerprint: String,
}

#[derive(Debug, Clone)]
struct MergeSource {
    item_id: Option<i64>,
    imported_row_id: Option<i64>,
    input: DuplicateInput,
}

pub fn rebuild_fingerprints(database: &Database, item_type: Option<&str>) -> AppResult<i64> {
    database.with_connection(|connection| {
        let rows = load_fingerprint_rows(connection, item_type)?;
        for row in &rows {
            connection.execute(
                "INSERT INTO knowledge_fingerprints
                 (item_id, type, code_norm, name_norm, pinyin_norm, alias_norm, fingerprint)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(item_id) DO UPDATE SET
                   type = excluded.type,
                   code_norm = excluded.code_norm,
                   name_norm = excluded.name_norm,
                   pinyin_norm = excluded.pinyin_norm,
                   alias_norm = excluded.alias_norm,
                   fingerprint = excluded.fingerprint",
                params![
                    row.item_id,
                    row.item_type,
                    row.code_norm,
                    row.name_norm,
                    row.pinyin_norm,
                    row.alias_norm,
                    row.fingerprint
                ],
            )?;
        }
        Ok(rows.len() as i64)
    })
}

pub fn detect_duplicates(
    database: &Database,
    batch_id: Option<i64>,
    item_type: Option<&str>,
) -> AppResult<i64> {
    database.with_connection(|connection| {
        if let Some(batch_id) = batch_id {
            detect_import_row_duplicates(connection, batch_id, item_type)
        } else {
            detect_item_duplicates(connection, item_type)
        }
    })
}

pub fn list_candidates(
    database: &Database,
    status: Option<&str>,
    page: u32,
    page_size: u32,
) -> AppResult<(i64, Vec<DuplicateCandidateDetail>)> {
    let offset = (page.saturating_sub(1) * page_size) as i64;
    database.with_connection(|connection| {
        let total = if let Some(status) = status {
            connection.query_row(
                "SELECT COUNT(1) FROM duplicate_candidates WHERE status = ?1",
                params![status],
                |row| row.get(0),
            )?
        } else {
            connection.query_row("SELECT COUNT(1) FROM duplicate_candidates", [], |row| {
                row.get(0)
            })?
        };

        let sql = if status.is_some() {
            "SELECT dc.id, dc.batch_id, dc.existing_item_id, dc.duplicate_item_id,
                    dc.imported_row_id, existing.name, duplicate.name,
                    COALESCE(
                      json_extract(imported.normalized_json, '$.name'),
                      json_extract(imported.mapped_json, '$.name'),
                      json_extract(imported.raw_json, '$.name')
                    ) AS imported_name,
                    dc.match_type, dc.match_score, dc.reason, dc.status, dc.created_at
             FROM duplicate_candidates dc
             LEFT JOIN knowledge_items existing ON existing.id = dc.existing_item_id
             LEFT JOIN knowledge_items duplicate ON duplicate.id = dc.duplicate_item_id
             LEFT JOIN data_import_rows imported ON imported.id = dc.imported_row_id
             WHERE dc.status = ?1
             ORDER BY dc.created_at DESC, dc.id DESC
             LIMIT ?2 OFFSET ?3"
        } else {
            "SELECT dc.id, dc.batch_id, dc.existing_item_id, dc.duplicate_item_id,
                    dc.imported_row_id, existing.name, duplicate.name,
                    COALESCE(
                      json_extract(imported.normalized_json, '$.name'),
                      json_extract(imported.mapped_json, '$.name'),
                      json_extract(imported.raw_json, '$.name')
                    ) AS imported_name,
                    dc.match_type, dc.match_score, dc.reason, dc.status, dc.created_at
             FROM duplicate_candidates dc
             LEFT JOIN knowledge_items existing ON existing.id = dc.existing_item_id
             LEFT JOIN knowledge_items duplicate ON duplicate.id = dc.duplicate_item_id
             LEFT JOIN data_import_rows imported ON imported.id = dc.imported_row_id
             ORDER BY dc.created_at DESC, dc.id DESC
             LIMIT ?1 OFFSET ?2"
        };

        let mut statement = connection.prepare(sql)?;
        let rows = if let Some(status) = status {
            statement.query_map(params![status, page_size, offset], map_candidate_detail)?
        } else {
            statement.query_map(params![page_size, offset], map_candidate_detail)?
        };
        let candidates = rows.collect::<Result<Vec<_>, _>>()?;
        Ok((total, candidates))
    })
}

pub fn merge_candidate(
    database: &Database,
    candidate_id: i64,
    strategy: &str,
) -> AppResult<MergeResult> {
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        let candidate = load_candidate_for_merge(&transaction, candidate_id)?;
        if candidate.status != "pending" {
            return Err(AppError::InvalidInput("只能处理 pending 状态的重复候选".to_string()));
        }
        let existing_item_id = candidate
            .existing_item_id
            .ok_or_else(|| AppError::InvalidInput("重复候选缺少 existing_item_id".to_string()))?;

        let source = load_merge_source(
            &transaction,
            candidate.duplicate_item_id,
            candidate.imported_row_id,
        )?;

        if strategy == "save_as_new" {
            let created_item_id = if source.imported_row_id.is_some() {
                Some(insert_item_from_input(&transaction, &source.input)?)
            } else {
                source.item_id
            };
            transaction.execute(
                "UPDATE duplicate_candidates SET status = 'saved_as_new' WHERE id = ?1",
                params![candidate_id],
            )?;
            transaction.commit()?;
            return Ok(MergeResult {
                existing_item_id,
                created_item_id,
                merge_record_id: None,
                status: "saved_as_new".to_string(),
            });
        }

        let before_json = load_item_snapshot(&transaction, existing_item_id)?;
        if strategy != "keep_existing" {
            apply_item_merge(&transaction, existing_item_id, &source.input, strategy)?;
            apply_detail_merge(&transaction, existing_item_id, &source.input, strategy)?;
            upsert_fingerprint_for_item(&transaction, existing_item_id)?;
            if let Some(source_item_id) = source.item_id {
                transaction.execute(
                    "DELETE FROM knowledge_items WHERE id = ?1",
                    params![source_item_id],
                )?;
            }
        }
        let after_json = load_item_snapshot(&transaction, existing_item_id)?;

        let merge_record_id = transaction.query_row(
            "INSERT INTO merge_records
             (existing_item_id, imported_row_id, merge_strategy, before_json, after_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))
             RETURNING id",
            params![
                existing_item_id,
                candidate.imported_row_id,
                strategy,
                before_json.to_string(),
                after_json.to_string()
            ],
            |row| row.get(0),
        )?;
        transaction.execute(
            "UPDATE duplicate_candidates SET status = ?2 WHERE id = ?1",
            params![candidate_id, if strategy == "keep_existing" { "kept" } else { "merged" }],
        )?;
        transaction.commit()?;

        Ok(MergeResult {
            existing_item_id,
            created_item_id: None,
            merge_record_id: Some(merge_record_id),
            status: if strategy == "keep_existing" { "kept" } else { "merged" }.to_string(),
        })
    })
}

#[allow(dead_code)]
pub fn upsert_fingerprint(database: &Database, item_id: i64) -> AppResult<()> {
    database.with_connection(|connection| upsert_fingerprint_for_item(connection, item_id))
}

fn detect_item_duplicates(connection: &Connection, item_type: Option<&str>) -> AppResult<i64> {
    let rows = load_fingerprint_rows(connection, item_type)?;
    let mut created = 0_i64;
    created += insert_group_matches(connection, &rows, "type_code_exact", 1.0, |row| {
        row.code_norm
            .as_ref()
            .map(|value| format!("{}|{}", row.item_type, value))
    })?;
    created += insert_group_matches(connection, &rows, "type_name_exact", 0.98, |row| {
        row.name_norm
            .as_ref()
            .map(|value| format!("{}|{}", row.item_type, value))
    })?;
    created += insert_group_matches(
        connection,
        &rows,
        "pinyin_category_suspect",
        0.78,
        |row| match (&row.pinyin_norm, &row.category_norm) {
            (Some(pinyin), Some(category)) => {
                Some(format!("{}|{}|{}", row.item_type, pinyin, category))
            }
            _ => None,
        },
    )?;
    created += insert_group_matches(connection, &rows, "fingerprint_match", 0.95, |row| {
        Some(row.fingerprint.clone())
    })?;
    created += insert_alias_matches(connection, &rows)?;
    Ok(created)
}

fn detect_import_row_duplicates(
    connection: &Connection,
    batch_id: i64,
    item_type: Option<&str>,
) -> AppResult<i64> {
    let mut created = 0_i64;
    let mut statement = connection.prepare(
        "SELECT id, normalized_json, mapped_json, raw_json
         FROM data_import_rows
         WHERE batch_id = ?1",
    )?;
    let rows = statement.query_map(params![batch_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;

    for row in rows {
        let (row_id, normalized, mapped, raw) = row?;
        let input = parse_import_input(
            normalized
                .as_deref()
                .or(mapped.as_deref())
                .or(raw.as_deref()),
        )?;
        if item_type.is_some_and(|kind| kind != input.item_type) {
            continue;
        }
        let fingerprint = build_fingerprint_from_input(&input);
        let matches = find_import_matches(connection, &input, &fingerprint)?;
        for (item_id, match_type, score, reason) in matches {
            created += insert_candidate(
                connection,
                Some(batch_id),
                Some(item_id),
                None,
                Some(row_id),
                &match_type,
                score,
                &reason,
            )?;
        }
    }
    Ok(created)
}

fn insert_group_matches(
    connection: &Connection,
    rows: &[ItemFingerprintRow],
    match_type: &str,
    score: f64,
    key: impl Fn(&ItemFingerprintRow) -> Option<String>,
) -> AppResult<i64> {
    let mut groups: HashMap<String, Vec<&ItemFingerprintRow>> = HashMap::new();
    for row in rows {
        if let Some(key) = key(row) {
            groups.entry(key).or_default().push(row);
        }
    }
    let mut created = 0_i64;
    for group in groups.values().filter(|group| group.len() > 1) {
        for left_index in 0..group.len() {
            for right in group.iter().skip(left_index + 1) {
                created += insert_candidate(
                    connection,
                    None,
                    Some(group[left_index].item_id),
                    Some(right.item_id),
                    None,
                    match_type,
                    score,
                    &format!(
                        "{} 命中：{} 与 {}",
                        match_type, group[left_index].item_id, right.item_id
                    ),
                )?;
            }
        }
    }
    Ok(created)
}

fn insert_alias_matches(connection: &Connection, rows: &[ItemFingerprintRow]) -> AppResult<i64> {
    let mut created = 0_i64;
    for left in rows {
        let Some(left_name) = &left.name_norm else {
            continue;
        };
        for right in rows {
            if left.item_id >= right.item_id || left.item_type != right.item_type {
                continue;
            }
            if alias_contains(right.alias_norm.as_deref(), left_name)
                || right.name_norm.as_deref().is_some_and(|right_name| {
                    alias_contains(left.alias_norm.as_deref(), right_name)
                })
            {
                created += insert_candidate(
                    connection,
                    None,
                    Some(left.item_id),
                    Some(right.item_id),
                    None,
                    "name_alias_match",
                    0.9,
                    &format!("名称命中别名：{} 与 {}", left.item_id, right.item_id),
                )?;
            }
        }
    }
    Ok(created)
}

fn find_import_matches(
    connection: &Connection,
    input: &DuplicateInput,
    fingerprint: &KnowledgeFingerprint,
) -> AppResult<Vec<(i64, String, f64, String)>> {
    let mut matches = Vec::new();
    let mut seen = HashSet::new();
    collect_matches_by_field(
        connection,
        &mut matches,
        &mut seen,
        "type_code_exact",
        1.0,
        "code_norm",
        &input.item_type,
        fingerprint.code_norm.as_deref(),
    )?;
    collect_matches_by_field(
        connection,
        &mut matches,
        &mut seen,
        "type_name_exact",
        0.98,
        "name_norm",
        &input.item_type,
        fingerprint.name_norm.as_deref(),
    )?;
    collect_matches_by_field(
        connection,
        &mut matches,
        &mut seen,
        "fingerprint_match",
        0.95,
        "fingerprint",
        &input.item_type,
        Some(&fingerprint.fingerprint),
    )?;
    if let (Some(pinyin), Some(category)) = (
        &fingerprint.pinyin_norm,
        input.category.as_deref().map(normalize_text),
    ) {
        let mut statement = connection.prepare(
            "SELECT kf.item_id
             FROM knowledge_fingerprints kf
             JOIN knowledge_items ki ON ki.id = kf.item_id
             WHERE kf.type = ?1 AND kf.pinyin_norm = ?2
               AND lower(replace(COALESCE(ki.category, ''), ' ', '')) = ?3",
        )?;
        let rows =
            statement.query_map(params![input.item_type, pinyin, category], |row| row.get(0))?;
        for row in rows {
            let item_id = row?;
            if seen.insert(("pinyin_category_suspect".to_string(), item_id)) {
                matches.push((
                    item_id,
                    "pinyin_category_suspect".to_string(),
                    0.78,
                    "拼音与分类疑似匹配".to_string(),
                ));
            }
        }
    }
    if let Some(name_norm) = &fingerprint.name_norm {
        let mut statement = connection.prepare(
            "SELECT item_id FROM knowledge_fingerprints
             WHERE type = ?1 AND alias_norm IS NOT NULL AND alias_norm LIKE ?2",
        )?;
        let pattern = format!("%|{}|%", name_norm);
        let rows = statement.query_map(params![input.item_type, pattern], |row| row.get(0))?;
        for row in rows {
            let item_id = row?;
            if seen.insert(("name_alias_match".to_string(), item_id)) {
                matches.push((
                    item_id,
                    "name_alias_match".to_string(),
                    0.9,
                    "名称命中已有条目别名".to_string(),
                ));
            }
        }
    }
    Ok(matches)
}

fn collect_matches_by_field(
    connection: &Connection,
    matches: &mut Vec<(i64, String, f64, String)>,
    seen: &mut HashSet<(String, i64)>,
    match_type: &str,
    score: f64,
    field: &str,
    item_type: &str,
    value: Option<&str>,
) -> AppResult<()> {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let sql =
        format!("SELECT item_id FROM knowledge_fingerprints WHERE type = ?1 AND {field} = ?2");
    let mut statement = connection.prepare(&sql)?;
    let rows = statement.query_map(params![item_type, value], |row| row.get(0))?;
    for row in rows {
        let item_id = row?;
        if seen.insert((match_type.to_string(), item_id)) {
            matches.push((
                item_id,
                match_type.to_string(),
                score,
                format!("{match_type} 命中"),
            ));
        }
    }
    Ok(())
}

fn insert_candidate(
    connection: &Connection,
    batch_id: Option<i64>,
    existing_item_id: Option<i64>,
    duplicate_item_id: Option<i64>,
    imported_row_id: Option<i64>,
    match_type: &str,
    match_score: f64,
    reason: &str,
) -> AppResult<i64> {
    let duplicate_exists: i64 = connection.query_row(
        "SELECT COUNT(1)
         FROM duplicate_candidates
         WHERE COALESCE(batch_id, -1) = COALESCE(?1, -1)
           AND COALESCE(existing_item_id, -1) = COALESCE(?2, -1)
           AND COALESCE(duplicate_item_id, -1) = COALESCE(?3, -1)
           AND COALESCE(imported_row_id, -1) = COALESCE(?4, -1)
           AND match_type = ?5
           AND status = 'pending'",
        params![
            batch_id,
            existing_item_id,
            duplicate_item_id,
            imported_row_id,
            match_type
        ],
        |row| row.get(0),
    )?;
    if duplicate_exists > 0 {
        return Ok(0);
    }
    connection.execute(
        "INSERT INTO duplicate_candidates
         (batch_id, existing_item_id, duplicate_item_id, imported_row_id, match_type, match_score, reason, status, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', datetime('now'))",
        params![
            batch_id,
            existing_item_id,
            duplicate_item_id,
            imported_row_id,
            match_type,
            match_score,
            reason
        ],
    )?;
    Ok(1)
}

fn load_fingerprint_rows(
    connection: &Connection,
    item_type: Option<&str>,
) -> AppResult<Vec<ItemFingerprintRow>> {
    let sql = if item_type.is_some() {
        "SELECT id, type, code, name, alias, pinyin, category FROM knowledge_items WHERE type = ?1"
    } else {
        "SELECT id, type, code, name, alias, pinyin, category FROM knowledge_items"
    };
    let mut statement = connection.prepare(sql)?;
    let rows = if let Some(item_type) = item_type {
        statement.query_map(params![item_type], map_fingerprint_source_row)?
    } else {
        statement.query_map([], map_fingerprint_source_row)?
    };
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn map_fingerprint_source_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ItemFingerprintRow> {
    let item_id = row.get(0)?;
    let item_type: String = row.get(1)?;
    let code: Option<String> = row.get(2)?;
    let name: String = row.get(3)?;
    let alias: Option<String> = row.get(4)?;
    let pinyin: Option<String> = row.get(5)?;
    let category: Option<String> = row.get(6)?;
    let input = DuplicateInput {
        item_type: item_type.clone(),
        code,
        name,
        alias,
        pinyin,
        category,
        summary: None,
        content: None,
        source_note: None,
        tags: None,
        detail: Map::new(),
    };
    let fingerprint = build_fingerprint_from_input(&input);
    Ok(ItemFingerprintRow {
        item_id,
        item_type,
        code_norm: fingerprint.code_norm,
        name_norm: fingerprint.name_norm,
        pinyin_norm: fingerprint.pinyin_norm,
        alias_norm: fingerprint.alias_norm,
        category_norm: input.category.as_deref().map(normalize_text),
        fingerprint: fingerprint.fingerprint,
    })
}

fn build_fingerprint_from_input(input: &DuplicateInput) -> KnowledgeFingerprint {
    let code_norm = input
        .code
        .as_deref()
        .map(normalize_code)
        .filter(|value| !value.is_empty());
    let name_norm = Some(normalize_text(&input.name)).filter(|value| !value.is_empty());
    let pinyin_norm = input
        .pinyin
        .as_deref()
        .map(normalize_pinyin)
        .filter(|value| !value.is_empty());
    let alias_norm = input
        .alias
        .as_deref()
        .map(normalize_aliases)
        .filter(|value| value != "||");
    let fingerprint = if let Some(code) = &code_norm {
        format!("{}|code|{}", input.item_type, code)
    } else {
        format!(
            "{}|name|{}|pinyin|{}|category|{}",
            input.item_type,
            name_norm.as_deref().unwrap_or_default(),
            pinyin_norm.as_deref().unwrap_or_default(),
            input
                .category
                .as_deref()
                .map(normalize_text)
                .unwrap_or_default()
        )
    };

    KnowledgeFingerprint {
        item_id: 0,
        item_type: input.item_type.clone(),
        code_norm,
        name_norm,
        pinyin_norm,
        alias_norm,
        fingerprint,
    }
}

fn upsert_fingerprint_for_item(connection: &Connection, item_id: i64) -> AppResult<()> {
    let row = connection
        .query_row(
            "SELECT id, type, code, name, alias, pinyin, category FROM knowledge_items WHERE id = ?1",
            params![item_id],
            map_fingerprint_source_row,
        )
        .optional()?;
    if let Some(row) = row {
        connection.execute(
            "INSERT INTO knowledge_fingerprints
             (item_id, type, code_norm, name_norm, pinyin_norm, alias_norm, fingerprint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(item_id) DO UPDATE SET
               type = excluded.type,
               code_norm = excluded.code_norm,
               name_norm = excluded.name_norm,
               pinyin_norm = excluded.pinyin_norm,
               alias_norm = excluded.alias_norm,
               fingerprint = excluded.fingerprint",
            params![
                item_id,
                row.item_type,
                row.code_norm,
                row.name_norm,
                row.pinyin_norm,
                row.alias_norm,
                row.fingerprint
            ],
        )?;
    }
    Ok(())
}

fn parse_import_input(json_text: Option<&str>) -> AppResult<DuplicateInput> {
    let text =
        json_text.ok_or_else(|| AppError::InvalidInput("导入行缺少 JSON 数据".to_string()))?;
    let value: Value = serde_json::from_str(text)
        .map_err(|err| AppError::InvalidInput(format!("导入行 JSON 无法解析: {err}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| AppError::InvalidInput("导入行 JSON 必须是对象".to_string()))?;
    let name = string_field(object, "name")
        .ok_or_else(|| AppError::InvalidInput("导入行缺少 name".to_string()))?;
    let item_type = string_field(object, "type")
        .or_else(|| string_field(object, "itemType"))
        .unwrap_or_else(|| "unknown".to_string());
    let mut detail = object.clone();
    for key in [
        "id",
        "type",
        "itemType",
        "code",
        "name",
        "alias",
        "pinyin",
        "category",
        "summary",
        "content",
        "sourceNote",
        "source_note",
        "tags",
    ] {
        detail.remove(key);
    }
    Ok(DuplicateInput {
        item_type,
        code: string_field(object, "code"),
        name,
        alias: string_field(object, "alias"),
        pinyin: string_field(object, "pinyin"),
        category: string_field(object, "category"),
        summary: string_field(object, "summary"),
        content: string_field(object, "content"),
        source_note: string_field(object, "sourceNote")
            .or_else(|| string_field(object, "source_note")),
        tags: string_field(object, "tags"),
        detail,
    })
}

fn load_merge_source(
    connection: &Connection,
    duplicate_item_id: Option<i64>,
    imported_row_id: Option<i64>,
) -> AppResult<MergeSource> {
    if let Some(item_id) = duplicate_item_id {
        let input = connection.query_row(
            "SELECT type, code, name, alias, pinyin, category, summary, content, source_note, tags
             FROM knowledge_items WHERE id = ?1",
            params![item_id],
            |row| {
                Ok(DuplicateInput {
                    item_type: row.get(0)?,
                    code: row.get(1)?,
                    name: row.get(2)?,
                    alias: row.get(3)?,
                    pinyin: row.get(4)?,
                    category: row.get(5)?,
                    summary: row.get(6)?,
                    content: row.get(7)?,
                    source_note: row.get(8)?,
                    tags: row.get(9)?,
                    detail: Map::new(),
                })
            },
        )?;
        return Ok(MergeSource {
            item_id: Some(item_id),
            imported_row_id: None,
            input,
        });
    }
    if let Some(row_id) = imported_row_id {
        let json_text: Option<String> = connection.query_row(
            "SELECT COALESCE(normalized_json, mapped_json, raw_json) FROM data_import_rows WHERE id = ?1",
            params![row_id],
            |row| row.get(0),
        )?;
        return Ok(MergeSource {
            item_id: None,
            imported_row_id: Some(row_id),
            input: parse_import_input(json_text.as_deref())?,
        });
    }
    Err(AppError::InvalidInput(
        "重复候选缺少可合并的新数据".to_string(),
    ))
}

fn insert_item_from_input(connection: &Connection, input: &DuplicateInput) -> AppResult<i64> {
    let item_id = connection.query_row(
        "INSERT INTO knowledge_items
         (type, code, name, alias, pinyin, category, summary, content, source_note, tags,
          data_status, completeness_status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'imported', 'partial', datetime('now'), datetime('now'))
         RETURNING id",
        params![
            input.item_type,
            input.code,
            input.name,
            input.alias,
            input.pinyin,
            input.category,
            input.summary,
            input.content,
            input.source_note,
            input.tags
        ],
        |row| row.get(0),
    )?;
    apply_detail_merge(connection, item_id, input, "overwrite")?;
    upsert_fingerprint_for_item(connection, item_id)?;
    Ok(item_id)
}

fn apply_item_merge(
    connection: &Connection,
    existing_item_id: i64,
    source: &DuplicateInput,
    strategy: &str,
) -> AppResult<()> {
    let current = load_item_values(connection, existing_item_id)?;
    let merged = merge_values(current, source, strategy);
    connection.execute(
        "UPDATE knowledge_items
         SET code = ?2, name = ?3, alias = ?4, pinyin = ?5, category = ?6,
             summary = ?7, content = ?8, source_note = ?9, tags = ?10,
             content_version = content_version + 1, updated_at = datetime('now')
         WHERE id = ?1",
        params![
            existing_item_id,
            merged.code,
            merged.name,
            merged.alias,
            merged.pinyin,
            merged.category,
            merged.summary,
            merged.content,
            merged.source_note,
            merged.tags
        ],
    )?;
    Ok(())
}

fn load_item_values(connection: &Connection, item_id: i64) -> AppResult<DuplicateInput> {
    connection
        .query_row(
            "SELECT type, code, name, alias, pinyin, category, summary, content, source_note, tags
         FROM knowledge_items WHERE id = ?1",
            params![item_id],
            |row| {
                Ok(DuplicateInput {
                    item_type: row.get(0)?,
                    code: row.get(1)?,
                    name: row.get(2)?,
                    alias: row.get(3)?,
                    pinyin: row.get(4)?,
                    category: row.get(5)?,
                    summary: row.get(6)?,
                    content: row.get(7)?,
                    source_note: row.get(8)?,
                    tags: row.get(9)?,
                    detail: Map::new(),
                })
            },
        )
        .map_err(Into::into)
}

fn merge_values(
    mut current: DuplicateInput,
    source: &DuplicateInput,
    strategy: &str,
) -> DuplicateInput {
    if strategy == "overwrite" {
        current.code = source.code.clone().or(current.code);
        current.name = source.name.clone();
        current.alias = source.alias.clone().or(current.alias);
        current.pinyin = source.pinyin.clone().or(current.pinyin);
        current.category = source.category.clone().or(current.category);
        current.summary = source.summary.clone().or(current.summary);
        current.content = source.content.clone().or(current.content);
        current.source_note = source.source_note.clone().or(current.source_note);
        current.tags = source.tags.clone().or(current.tags);
    } else if strategy == "fill_empty" {
        current.code = fill_empty(current.code, source.code.clone());
        current.alias = fill_empty(current.alias, source.alias.clone());
        current.pinyin = fill_empty(current.pinyin, source.pinyin.clone());
        current.category = fill_empty(current.category, source.category.clone());
        current.summary = fill_empty(current.summary, source.summary.clone());
        current.content = fill_empty(current.content, source.content.clone());
        current.source_note = fill_empty(current.source_note, source.source_note.clone());
        current.tags = fill_empty(current.tags, source.tags.clone());
    } else if strategy == "merge_tags" {
        current.tags = Some(merge_tag_text(
            current.tags.as_deref(),
            source.tags.as_deref(),
        ));
        current.alias = Some(merge_tag_text(
            current.alias.as_deref(),
            source.alias.as_deref(),
        ));
    }
    current
}

fn apply_detail_merge(
    connection: &Connection,
    item_id: i64,
    source: &DuplicateInput,
    strategy: &str,
) -> AppResult<()> {
    let Some((table, columns)) = detail_table_columns(&source.item_type) else {
        return Ok(());
    };
    if source.detail.is_empty() && strategy != "overwrite" {
        return Ok(());
    }
    connection.execute(
        &format!("INSERT OR IGNORE INTO {table} (item_id) VALUES (?1)"),
        params![item_id],
    )?;
    for column in columns {
        let Some(new_value) = source.detail.get(*column).and_then(value_to_string) else {
            continue;
        };
        if strategy == "overwrite" {
            connection.execute(
                &format!("UPDATE {table} SET {column} = ?2 WHERE item_id = ?1"),
                params![item_id, new_value],
            )?;
        } else if strategy == "fill_empty" {
            connection.execute(
                &format!(
                    "UPDATE {table} SET {column} = ?2
                     WHERE item_id = ?1 AND ({column} IS NULL OR trim({column}) = '')"
                ),
                params![item_id, new_value],
            )?;
        }
    }
    Ok(())
}

fn load_item_snapshot(connection: &Connection, item_id: i64) -> AppResult<Value> {
    let item = connection.query_row(
        "SELECT id, type, code, name, alias, pinyin, category, summary, content, source_note,
                tags, data_status, completeness_status, content_version, is_favorite, created_at, updated_at
         FROM knowledge_items WHERE id = ?1",
        params![item_id],
        |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "type": row.get::<_, String>(1)?,
                "code": row.get::<_, Option<String>>(2)?,
                "name": row.get::<_, String>(3)?,
                "alias": row.get::<_, Option<String>>(4)?,
                "pinyin": row.get::<_, Option<String>>(5)?,
                "category": row.get::<_, Option<String>>(6)?,
                "summary": row.get::<_, Option<String>>(7)?,
                "content": row.get::<_, Option<String>>(8)?,
                "sourceNote": row.get::<_, Option<String>>(9)?,
                "tags": row.get::<_, Option<String>>(10)?,
                "dataStatus": row.get::<_, String>(11)?,
                "completenessStatus": row.get::<_, String>(12)?,
                "contentVersion": row.get::<_, i64>(13)?,
                "isFavorite": row.get::<_, i64>(14)? != 0,
                "createdAt": row.get::<_, String>(15)?,
                "updatedAt": row.get::<_, String>(16)?,
            }))
        },
    )?;
    Ok(item)
}

#[derive(Debug)]
struct CandidateForMerge {
    existing_item_id: Option<i64>,
    duplicate_item_id: Option<i64>,
    imported_row_id: Option<i64>,
    status: String,
}

fn load_candidate_for_merge(
    connection: &Connection,
    candidate_id: i64,
) -> AppResult<CandidateForMerge> {
    connection
        .query_row(
            "SELECT existing_item_id, duplicate_item_id, imported_row_id, status
             FROM duplicate_candidates WHERE id = ?1",
            params![candidate_id],
            |row| {
                Ok(CandidateForMerge {
                    existing_item_id: row.get(0)?,
                    duplicate_item_id: row.get(1)?,
                    imported_row_id: row.get(2)?,
                    status: row.get(3)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::InvalidInput(format!("重复候选不存在: {candidate_id}")))
}

fn map_candidate_detail(row: &rusqlite::Row<'_>) -> rusqlite::Result<DuplicateCandidateDetail> {
    Ok(DuplicateCandidateDetail {
        id: row.get(0)?,
        batch_id: row.get(1)?,
        existing_item_id: row.get(2)?,
        duplicate_item_id: row.get(3)?,
        imported_row_id: row.get(4)?,
        existing_name: row.get(5)?,
        duplicate_name: row.get(6)?,
        imported_name: row.get(7)?,
        match_type: row.get(8)?,
        match_score: row.get(9)?,
        reason: row.get(10)?,
        status: row.get(11)?,
        created_at: row.get(12)?,
    })
}

fn normalize_code(value: &str) -> String {
    normalize_text(value)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_uppercase()
}

fn normalize_pinyin(value: &str) -> String {
    normalize_text(value)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect()
}

fn normalize_aliases(value: &str) -> String {
    let mut parts = value
        .split([',', '，', ';', '；', '|', '/', '、', ' '])
        .map(normalize_text)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    parts.sort();
    parts.dedup();
    format!("|{}|", parts.join("|"))
}

fn normalize_text(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(normalize_char)
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
}

fn normalize_char(ch: char) -> char {
    match ch {
        '\u{3000}' => ' ',
        'Ａ'..='Ｚ' | 'ａ'..='ｚ' | '０'..='９' => {
            char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch)
        }
        _ => ch,
    }
}

fn alias_contains(alias_norm: Option<&str>, name_norm: &str) -> bool {
    alias_norm.is_some_and(|aliases| aliases.contains(&format!("|{name_norm}|")))
}

fn string_field(object: &Map<String, Value>, key: &str) -> Option<String> {
    object
        .get(key)
        .and_then(value_to_string)
        .filter(|value| !value.trim().is_empty())
}

fn value_to_string(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn fill_empty(current: Option<String>, source: Option<String>) -> Option<String> {
    if current
        .as_deref()
        .is_none_or(|value| value.trim().is_empty())
    {
        source.or(current)
    } else {
        current
    }
}

fn merge_tag_text(left: Option<&str>, right: Option<&str>) -> String {
    let mut seen = HashSet::new();
    let mut values = Vec::new();
    for value in [left, right].into_iter().flatten() {
        for part in value.split([',', '，', ';', '；', '|', '/', '、']) {
            let normalized = part.trim();
            if !normalized.is_empty() && seen.insert(normalize_text(normalized)) {
                values.push(normalized.to_string());
            }
        }
    }
    values.join("，")
}

fn detail_table_columns(item_type: &str) -> Option<(&'static str, &'static [&'static str])> {
    match item_type {
        "herb" => Some((
            "herb_details",
            &[
                "nature_flavor",
                "meridians",
                "effects",
                "indications",
                "dosage",
                "contraindications",
                "compatibility",
                "notes",
            ],
        )),
        "formula" => Some((
            "formula_details",
            &[
                "source_text",
                "composition",
                "usage",
                "effects",
                "indications",
                "explanation",
                "modifications",
                "contraindications",
                "notes",
            ],
        )),
        "meridian" => Some((
            "meridian_details",
            &[
                "meridian_code",
                "category",
                "yin_yang",
                "hand_foot",
                "organ_relation",
                "paired_meridian",
                "pathway_text",
                "main_indications",
                "notes",
            ],
        )),
        "acupoint" => Some((
            "acupoint_details",
            &[
                "acupoint_code",
                "body_region",
                "body_subregion",
                "side_type",
                "standard_location",
                "locating_method",
                "bone_cun",
                "anatomy",
                "functions",
                "indications",
                "needling_summary",
                "moxibustion_summary",
                "massage_summary",
                "contraindications",
                "precautions",
                "risk_level",
            ],
        )),
        "syndrome" => Some((
            "syndrome_details",
            &[
                "symptoms",
                "tongue",
                "pulse",
                "pathogenesis",
                "treatment_principle",
                "notes",
            ],
        )),
        "disease" => Some((
            "disease_details",
            &[
                "symptoms",
                "common_syndromes",
                "care_advice",
                "medical_warning",
                "notes",
            ],
        )),
        _ => None,
    }
}
#[allow(dead_code)]

pub fn calculate_multi_dimensional_similarity(
    name1: &str,
    pinyin1: Option<&str>,
    code1: Option<&str>,
    name2: Option<&str>,
    pinyin2: Option<&str>,
    code2: Option<&str>,
) -> f64 {
    let mut total_weight = 0.0;
    let mut weighted_score = 0.0;

    if let Some(n2) = name2 {
        let name_sim = jaro_winkler_similarity(name1, n2);
        weighted_score += name_sim * 0.5;
        total_weight += 0.5;
    }

    if let (Some(p1), Some(p2)) = (pinyin1, pinyin2) {
        let pinyin_sim = jaro_winkler_similarity(p1, p2);
        weighted_score += pinyin_sim * 0.3;
        total_weight += 0.3;
    }

    if let (Some(c1), Some(c2)) = (code1, code2) {
        let code_match = if c1 == c2 { 1.0 } else { 0.0 };
        weighted_score += code_match * 0.2;
        total_weight += 0.2;
    }

    if total_weight > 0.0 {
        weighted_score / total_weight
    } else {
        0.0
    }
}
#[allow(dead_code)]

fn jaro_winkler_similarity(s1: &str, s2: &str) -> f64 {
    if s1 == s2 {
        return 1.0;
    }
    if s1.is_empty() || s2.is_empty() {
        return 0.0;
    }

    let len1 = s1.chars().count();
    let len2 = s2.chars().count();
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

    let prefix_len = s1_chars
        .iter()
        .zip(s2_chars.iter())
        .take(4)
        .take_while(|(a, b)| a == b)
        .count();

    jaro + (prefix_len as f64 * 0.1 * (1.0 - jaro))
}
