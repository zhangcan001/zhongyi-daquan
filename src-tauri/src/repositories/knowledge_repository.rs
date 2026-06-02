use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::knowledge::{KnowledgeInput, KnowledgeItem, KnowledgeListRequest};
use rusqlite::{params, Connection, OptionalExtension};

pub fn list(
    database: &Database,
    request: &KnowledgeListRequest,
) -> AppResult<(i64, Vec<KnowledgeItem>)> {
    let page = request.page.unwrap_or(1).max(1);
    let page_size = request.page_size.unwrap_or(50).clamp(1, 200);
    let offset = (page.saturating_sub(1) * page_size) as i64;
    database.with_connection(|connection| {
        let mut filters = Vec::new();
        let mut values = Vec::new();
        if let Some(item_type) = clean_opt(request.item_type.as_deref()) {
            filters.push("type = ?".to_string());
            values.push(item_type);
        }
        if let Some(status) = clean_opt(request.data_status.as_deref()) {
            filters.push("data_status = ?".to_string());
            values.push(status);
        }
        if request.favorite_only.unwrap_or(false) {
            filters.push("is_favorite = 1".to_string());
        }
        if let Some(query) = clean_opt(request.query.as_deref()) {
            filters.push(
                "(name LIKE ? OR code LIKE ? OR alias LIKE ? OR pinyin LIKE ? OR category LIKE ? OR summary LIKE ? OR content LIKE ? OR source_note LIKE ? OR tags LIKE ? OR detail LIKE ?)"
                    .to_string(),
            );
            let like = format!("%{query}%");
            values.extend([
                like.clone(),
                like.clone(),
                like.clone(),
                like.clone(),
                like.clone(),
                like.clone(),
                like.clone(),
                like.clone(),
                like.clone(),
                like,
            ]);
        }

        let where_sql = if filters.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", filters.join(" AND "))
        };
        let count_sql = format!("SELECT COUNT(1) FROM knowledge_items{where_sql}");
        let total: i64 = connection.query_row(
            &count_sql,
            rusqlite::params_from_iter(values.iter()),
            |row| row.get(0),
        )?;

        let list_sql = format!(
            "SELECT id, type, code, name, alias, pinyin, category, summary, content,
                    source_note, tags, data_status, completeness_status, content_version,
                    is_favorite, detail, import_batch_id, source_package, created_at, updated_at
             FROM knowledge_items{where_sql}
             ORDER BY updated_at DESC, id DESC
             LIMIT ? OFFSET ?"
        );
        let mut list_values = values;
        list_values.push(page_size.to_string());
        list_values.push(offset.to_string());
        let mut statement = connection.prepare(&list_sql)?;
        let rows =
            statement.query_map(rusqlite::params_from_iter(list_values.iter()), map_item_row)?;
        Ok((total, rows.collect::<Result<Vec<_>, _>>()?))
    })
}

pub fn get_by_id(connection: &Connection, item_id: i64) -> AppResult<Option<KnowledgeItem>> {
    connection
        .query_row(
            "SELECT id, type, code, name, alias, pinyin, category, summary, content,
                    source_note, tags, data_status, completeness_status, content_version,
                    is_favorite, detail, import_batch_id, source_package, created_at, updated_at
             FROM knowledge_items
             WHERE id = ?1",
            params![item_id],
            map_item_row,
        )
        .optional()
        .map_err(Into::into)
}

pub fn insert_tx(connection: &Connection, input: &KnowledgeInput) -> AppResult<i64> {
    connection.execute(
        "INSERT INTO knowledge_items
         (type, code, name, alias, pinyin, category, summary, content, source_note, tags,
          data_status, completeness_status, content_version, is_favorite, detail, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, ?13, ?14, datetime('now'), datetime('now'))",
        params![
            input.item_type,
            empty_to_none(input.code.as_deref()),
            input.name.trim(),
            empty_to_none(input.alias.as_deref()),
            empty_to_none(input.pinyin.as_deref()),
            empty_to_none(input.category.as_deref()),
            empty_to_none(input.summary.as_deref()),
            empty_to_none(input.content.as_deref()),
            empty_to_none(input.source_note.as_deref()),
            empty_to_none(input.tags.as_deref()),
            input.data_status,
            input.completeness_status,
            i64::from(input.is_favorite),
            normalize_detail_json(&input.detail)
        ],
    )?;
    Ok(connection.last_insert_rowid())
}

pub fn update_tx(connection: &Connection, item_id: i64, input: &KnowledgeInput) -> AppResult<()> {
    connection.execute(
        "UPDATE knowledge_items
         SET type = ?1,
             code = ?2,
             name = ?3,
             alias = ?4,
             pinyin = ?5,
             category = ?6,
             summary = ?7,
             content = ?8,
             source_note = ?9,
             tags = ?10,
             data_status = ?11,
             completeness_status = ?12,
             is_favorite = ?13,
             detail = ?14,
             content_version = content_version + 1,
             updated_at = datetime('now')
         WHERE id = ?15",
        params![
            input.item_type,
            empty_to_none(input.code.as_deref()),
            input.name.trim(),
            empty_to_none(input.alias.as_deref()),
            empty_to_none(input.pinyin.as_deref()),
            empty_to_none(input.category.as_deref()),
            empty_to_none(input.summary.as_deref()),
            empty_to_none(input.content.as_deref()),
            empty_to_none(input.source_note.as_deref()),
            empty_to_none(input.tags.as_deref()),
            input.data_status,
            input.completeness_status,
            i64::from(input.is_favorite),
            normalize_detail_json(&input.detail),
            item_id
        ],
    )?;
    Ok(())
}

pub fn set_favorite_tx(connection: &Connection, item_id: i64, is_favorite: bool) -> AppResult<()> {
    connection.execute(
        "UPDATE knowledge_items
         SET is_favorite = ?1, updated_at = datetime('now')
         WHERE id = ?2",
        params![i64::from(is_favorite), item_id],
    )?;
    Ok(())
}

pub fn delete_tx(connection: &Connection, item_id: i64) -> AppResult<()> {
    connection.execute(
        "DELETE FROM knowledge_items WHERE id = ?1",
        params![item_id],
    )?;
    Ok(())
}

fn map_item_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeItem> {
    Ok(KnowledgeItem {
        id: row.get(0)?,
        item_type: row.get(1)?,
        code: row.get(2)?,
        name: row.get(3)?,
        alias: row.get(4)?,
        pinyin: row.get(5)?,
        category: row.get(6)?,
        summary: row.get(7)?,
        content: row.get(8)?,
        source_note: row.get(9)?,
        tags: row.get(10)?,
        data_status: row.get(11)?,
        completeness_status: row.get(12)?,
        content_version: row.get(13)?,
        is_favorite: row.get::<_, i64>(14)? == 1,
        detail: row
            .get::<_, Option<String>>(15)?
            .and_then(|text| serde_json::from_str(&text).ok()),
        import_batch_id: row.get(16)?,
        source_package: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

fn clean_opt(value: Option<&str>) -> Option<String> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn empty_to_none(value: Option<&str>) -> Option<&str> {
    value.and_then(|text| {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn normalize_detail_json(detail: &serde_json::Value) -> String {
    match detail {
        serde_json::Value::Object(_) => detail.to_string(),
        serde_json::Value::String(text) => serde_json::from_str::<serde_json::Value>(text)
            .map(|value| value.to_string())
            .unwrap_or_else(|_| {
                serde_json::json!({ "raw_detail": text, "parse_error": true }).to_string()
            }),
        serde_json::Value::Null => "{}".to_string(),
        other => other.to_string(),
    }
}

/// 从版本快照恢复知识条目（用于版本回滚）
pub fn update_from_snapshot(
    connection: &Connection,
    item_id: i64,
    snapshot: &serde_json::Value,
) -> AppResult<()> {
    let obj = snapshot
        .as_object()
        .ok_or_else(|| crate::errors::AppError::InvalidInput("快照格式错误".to_string()))?;

    // 提取字段
    let name = obj
        .get("name")
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| crate::errors::AppError::InvalidInput("缺少 name 字段".to_string()))?;

    let item_type = obj
        .get("type")
        .or_else(|| obj.get("item_type"))
        .and_then(|v: &serde_json::Value| v.as_str())
        .ok_or_else(|| crate::errors::AppError::InvalidInput("缺少 type 字段".to_string()))?;

    let code = obj.get("code").and_then(|v: &serde_json::Value| v.as_str());
    let alias = obj
        .get("alias")
        .and_then(|v: &serde_json::Value| v.as_str());
    let pinyin = obj
        .get("pinyin")
        .and_then(|v: &serde_json::Value| v.as_str());
    let category = obj
        .get("category")
        .and_then(|v: &serde_json::Value| v.as_str());
    let summary = obj
        .get("summary")
        .and_then(|v: &serde_json::Value| v.as_str());
    let content = obj
        .get("content")
        .and_then(|v: &serde_json::Value| v.as_str());
    let source_note = obj
        .get("source_note")
        .and_then(|v: &serde_json::Value| v.as_str());
    let tags = obj.get("tags").and_then(|v: &serde_json::Value| v.as_str());
    let data_status = obj
        .get("data_status")
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap_or("draft");
    let completeness_status = obj
        .get("completeness_status")
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap_or("partial");

    connection.execute(
        "UPDATE knowledge_items
         SET type = ?1, code = ?2, name = ?3, alias = ?4, pinyin = ?5,
             category = ?6, summary = ?7, content = ?8, source_note = ?9,
             tags = ?10, data_status = ?11, completeness_status = ?12,
             updated_at = datetime('now')
         WHERE id = ?13",
        params![
            item_type,
            code,
            name,
            alias,
            pinyin,
            category,
            summary,
            content,
            source_note,
            tags,
            data_status,
            completeness_status,
            item_id
        ],
    )?;

    Ok(())
}
#[allow(dead_code)]
pub fn get_item(
    database: &Database,
    item_id: i64,
) -> AppResult<serde_json::Map<String, serde_json::Value>> {
    use serde_json::{Map, Value};

    database.with_connection(|connection| {
        let mut map = Map::new();

        // 获取主表数据
        connection.query_row(
            "SELECT type, code, name, alias, pinyin, category, summary, content, source_note, tags,
                    data_status, completeness_status, content_version, is_favorite
             FROM knowledge_items WHERE id = ?1",
            [item_id],
            |row| {
                map.insert("id".to_string(), Value::Number(item_id.into()));
                map.insert("type".to_string(), Value::String(row.get(0)?));
                if let Ok(code) = row.get::<_, Option<String>>(1) {
                    map.insert(
                        "code".to_string(),
                        code.map(Value::String).unwrap_or(Value::Null),
                    );
                }
                map.insert("name".to_string(), Value::String(row.get(2)?));
                if let Ok(alias) = row.get::<_, Option<String>>(3) {
                    map.insert(
                        "alias".to_string(),
                        alias.map(Value::String).unwrap_or(Value::Null),
                    );
                }
                if let Ok(pinyin) = row.get::<_, Option<String>>(4) {
                    map.insert(
                        "pinyin".to_string(),
                        pinyin.map(Value::String).unwrap_or(Value::Null),
                    );
                }
                if let Ok(category) = row.get::<_, Option<String>>(5) {
                    map.insert(
                        "category".to_string(),
                        category.map(Value::String).unwrap_or(Value::Null),
                    );
                }
                if let Ok(summary) = row.get::<_, Option<String>>(6) {
                    map.insert(
                        "summary".to_string(),
                        summary.map(Value::String).unwrap_or(Value::Null),
                    );
                }
                if let Ok(content) = row.get::<_, Option<String>>(7) {
                    map.insert(
                        "content".to_string(),
                        content.map(Value::String).unwrap_or(Value::Null),
                    );
                }
                if let Ok(source_note) = row.get::<_, Option<String>>(8) {
                    map.insert(
                        "source_note".to_string(),
                        source_note.map(Value::String).unwrap_or(Value::Null),
                    );
                }
                if let Ok(tags) = row.get::<_, Option<String>>(9) {
                    map.insert(
                        "tags".to_string(),
                        tags.map(Value::String).unwrap_or(Value::Null),
                    );
                }
                map.insert("data_status".to_string(), Value::String(row.get(10)?));
                map.insert(
                    "completeness_status".to_string(),
                    Value::String(row.get(11)?),
                );
                Ok(())
            },
        )?;

        Ok(map)
    })
}

/// 批量获取知识条目（用于导出，避免 N+1 查询）
pub fn get_items_batch(
    database: &Database,
    item_ids: &[i64],
) -> AppResult<Vec<serde_json::Map<String, serde_json::Value>>> {
    use serde_json::{Map, Value};

    if item_ids.is_empty() {
        return Ok(Vec::new());
    }

    database.with_connection(|connection| {
        let placeholders = item_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "SELECT id, type, code, name, alias, pinyin, category, summary, content, source_note, tags,
                    data_status, completeness_status, content_version, is_favorite
             FROM knowledge_items WHERE id IN ({})",
            placeholders
        );

        let mut statement = connection.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> = item_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();

        let rows = statement.query_map(params.as_slice(), |row| {
            let mut map = Map::new();
            map.insert("id".to_string(), Value::Number(row.get::<_, i64>(0)?.into()));
            map.insert("type".to_string(), Value::String(row.get(1)?));
            if let Ok(Some(code)) = row.get::<_, Option<String>>(2) {
                map.insert("code".to_string(), Value::String(code));
            }
            map.insert("name".to_string(), Value::String(row.get(3)?));
            if let Ok(Some(alias)) = row.get::<_, Option<String>>(4) {
                map.insert("alias".to_string(), Value::String(alias));
            }
            if let Ok(Some(pinyin)) = row.get::<_, Option<String>>(5) {
                map.insert("pinyin".to_string(), Value::String(pinyin));
            }
            if let Ok(Some(category)) = row.get::<_, Option<String>>(6) {
                map.insert("category".to_string(), Value::String(category));
            }
            if let Ok(Some(summary)) = row.get::<_, Option<String>>(7) {
                map.insert("summary".to_string(), Value::String(summary));
            }
            if let Ok(Some(content)) = row.get::<_, Option<String>>(8) {
                map.insert("content".to_string(), Value::String(content));
            }
            if let Ok(Some(source_note)) = row.get::<_, Option<String>>(9) {
                map.insert("source_note".to_string(), Value::String(source_note));
            }
            if let Ok(Some(tags)) = row.get::<_, Option<String>>(10) {
                map.insert("tags".to_string(), Value::String(tags));
            }
            map.insert("data_status".to_string(), Value::String(row.get(11)?));
            map.insert("completeness_status".to_string(), Value::String(row.get(12)?));
            Ok(map)
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}
