use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::relation::{KnowledgeRelationView, RelationSuggestionDetail};
use rusqlite::{params, Connection, OptionalExtension};

#[derive(Debug, Clone)]
pub struct RelationSuggestionInput {
    pub source_item_id: i64,
    pub target_item_id: i64,
    pub relation_type: String,
    pub confidence: f64,
    pub reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RelationSource {
    pub item_id: i64,
    pub item_type: String,
    pub name: String,
    pub code: Option<String>,
    pub alias: Option<String>,
    pub category: Option<String>,
    pub content: String,
    pub meridian_item_id: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RelationTarget {
    pub item_id: i64,
    pub item_type: String,
    pub name: String,
    pub code: Option<String>,
    pub alias: Option<String>,
}

pub fn load_sources(
    database: &Database,
    item_type: Option<&str>,
    source_item_id: Option<i64>,
) -> AppResult<Vec<RelationSource>> {
    database.with_connection(|connection| load_sources_inner(connection, item_type, source_item_id))
}

pub fn load_targets(database: &Database, item_type: &str) -> AppResult<Vec<RelationTarget>> {
    database.with_connection(|connection| {
        let mut statement = connection
            .prepare("SELECT id, type, name, code, alias FROM knowledge_items WHERE type = ?1")?;
        let rows = statement.query_map(params![item_type], |row| {
            Ok(RelationTarget {
                item_id: row.get(0)?,
                item_type: row.get(1)?,
                name: row.get(2)?,
                code: row.get(3)?,
                alias: row.get(4)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}

pub fn list_relations_for_item(
    database: &Database,
    item_id: i64,
) -> AppResult<Vec<KnowledgeRelationView>> {
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT kr.id,
                    kr.source_item_id,
                    kr.target_item_id,
                    related.id,
                    related.type,
                    related.name,
                    related.code,
                    related.category,
                    kr.relation_type,
                    CASE WHEN kr.source_item_id = ?1 THEN 'outgoing' ELSE 'incoming' END,
                    kr.note
             FROM knowledge_relations kr
             JOIN knowledge_items related
               ON related.id = CASE
                 WHEN kr.source_item_id = ?1 THEN kr.target_item_id
                 ELSE kr.source_item_id
               END
             WHERE kr.source_item_id = ?1 OR kr.target_item_id = ?1
             ORDER BY kr.relation_type COLLATE NOCASE,
                      related.type COLLATE NOCASE,
                      related.name COLLATE NOCASE",
        )?;
        let rows = statement.query_map(params![item_id], map_relation_view)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}

pub fn insert_suggestions(
    database: &Database,
    suggestions: &[RelationSuggestionInput],
) -> AppResult<i64> {
    database.with_connection(|connection| {
        let mut created = 0_i64;
        for suggestion in suggestions {
            let exists: i64 = connection.query_row(
                "SELECT COUNT(1)
                 FROM relation_suggestions
                 WHERE source_item_id = ?1 AND target_item_id = ?2
                   AND relation_type = ?3 AND status = 'pending'",
                params![
                    suggestion.source_item_id,
                    suggestion.target_item_id,
                    suggestion.relation_type
                ],
                |row| row.get(0),
            )?;
            let relation_exists: i64 = connection.query_row(
                "SELECT COUNT(1)
                 FROM knowledge_relations
                 WHERE source_item_id = ?1 AND target_item_id = ?2 AND relation_type = ?3",
                params![
                    suggestion.source_item_id,
                    suggestion.target_item_id,
                    suggestion.relation_type
                ],
                |row| row.get(0),
            )?;
            if exists == 0 && relation_exists == 0 {
                connection.execute(
                    "INSERT INTO relation_suggestions
                     (source_item_id, target_item_id, relation_type, confidence, reason, status, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, 'pending', datetime('now'))",
                    params![
                        suggestion.source_item_id,
                        suggestion.target_item_id,
                        suggestion.relation_type,
                        suggestion.confidence,
                        suggestion.reason
                    ],
                )?;
                created += 1;
            }
        }
        Ok(created)
    })
}

pub fn list_suggestions(
    database: &Database,
    status: Option<&str>,
    page: u32,
    page_size: u32,
) -> AppResult<(i64, Vec<RelationSuggestionDetail>)> {
    let offset = (page.saturating_sub(1) * page_size) as i64;
    database.with_connection(|connection| {
        let total = if let Some(status) = status {
            connection.query_row(
                "SELECT COUNT(1) FROM relation_suggestions WHERE status = ?1",
                params![status],
                |row| row.get(0),
            )?
        } else {
            connection.query_row("SELECT COUNT(1) FROM relation_suggestions", [], |row| {
                row.get(0)
            })?
        };

        let sql = if status.is_some() {
            "SELECT rs.id, rs.source_item_id, rs.target_item_id, source.name, target.name,
                    rs.relation_type, rs.confidence, rs.reason, rs.status, rs.created_at
             FROM relation_suggestions rs
             LEFT JOIN knowledge_items source ON source.id = rs.source_item_id
             LEFT JOIN knowledge_items target ON target.id = rs.target_item_id
             WHERE rs.status = ?1
             ORDER BY rs.created_at DESC, rs.id DESC
             LIMIT ?2 OFFSET ?3"
        } else {
            "SELECT rs.id, rs.source_item_id, rs.target_item_id, source.name, target.name,
                    rs.relation_type, rs.confidence, rs.reason, rs.status, rs.created_at
             FROM relation_suggestions rs
             LEFT JOIN knowledge_items source ON source.id = rs.source_item_id
             LEFT JOIN knowledge_items target ON target.id = rs.target_item_id
             ORDER BY rs.created_at DESC, rs.id DESC
             LIMIT ?1 OFFSET ?2"
        };
        let mut statement = connection.prepare(sql)?;
        let rows = if let Some(status) = status {
            statement.query_map(params![status, page_size, offset], map_suggestion_detail)?
        } else {
            statement.query_map(params![page_size, offset], map_suggestion_detail)?
        };
        let suggestions = rows.collect::<Result<Vec<_>, _>>()?;
        Ok((total, suggestions))
    })
}

pub fn accept_suggestion(database: &Database, suggestion_id: i64) -> AppResult<i64> {
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        let suggestion = transaction
            .query_row(
                "SELECT source_item_id, target_item_id, relation_type, reason, status
                 FROM relation_suggestions WHERE id = ?1",
                params![suggestion_id],
                |row| {
                    Ok((
                        row.get::<_, Option<i64>>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| AppError::InvalidInput(format!("关系建议不存在: {suggestion_id}")))?;
        if suggestion.4 != "pending" {
            return Err(AppError::InvalidInput(
                "只能接受 pending 状态的关系建议".to_string(),
            ));
        }
        let source_item_id = suggestion
            .0
            .ok_or_else(|| AppError::InvalidInput("关系建议缺少 source_item_id".to_string()))?;
        let target_item_id = suggestion
            .1
            .ok_or_else(|| AppError::InvalidInput("关系建议缺少 target_item_id".to_string()))?;
        let existing_relation_id = transaction
            .query_row(
                "SELECT id FROM knowledge_relations
                 WHERE source_item_id = ?1 AND target_item_id = ?2 AND relation_type = ?3",
                params![source_item_id, target_item_id, suggestion.2],
                |row| row.get(0),
            )
            .optional()?;
        let relation_id = if let Some(relation_id) = existing_relation_id {
            relation_id
        } else {
            transaction.query_row(
                "INSERT INTO knowledge_relations
                 (source_item_id, target_item_id, relation_type, note)
                 VALUES (?1, ?2, ?3, ?4)
                 RETURNING id",
                params![source_item_id, target_item_id, suggestion.2, suggestion.3],
                |row| row.get(0),
            )?
        };
        transaction.execute(
            "UPDATE relation_suggestions SET status = 'accepted' WHERE id = ?1",
            params![suggestion_id],
        )?;
        update_relation_count_cache_tx(&transaction, source_item_id, &suggestion.2)?;
        update_relation_count_cache_tx(&transaction, target_item_id, &suggestion.2)?;
        transaction.commit()?;
        Ok(relation_id)
    })
}

pub fn reject_suggestion(database: &Database, suggestion_id: i64) -> AppResult<()> {
    database.with_connection(|connection| {
        let changed = connection.execute(
            "UPDATE relation_suggestions SET status = 'rejected'
             WHERE id = ?1 AND status = 'pending'",
            params![suggestion_id],
        )?;
        if changed == 0 {
            return Err(AppError::InvalidInput(
                "未找到 pending 状态的关系建议".to_string(),
            ));
        }
        Ok(())
    })
}

fn load_sources_inner(
    connection: &Connection,
    item_type: Option<&str>,
    source_item_id: Option<i64>,
) -> AppResult<Vec<RelationSource>> {
    let mut conditions = Vec::new();
    let mut args: Vec<String> = Vec::new();
    if let Some(item_type) = item_type {
        conditions.push("ki.type = ?".to_string());
        args.push(item_type.to_string());
    }
    if let Some(source_item_id) = source_item_id {
        conditions.push("ki.id = ?".to_string());
        args.push(source_item_id.to_string());
    }
    let where_sql = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };
    let sql = format!(
        "SELECT ki.id, ki.type, ki.name, ki.code, ki.alias, ki.category,
                trim(
                  COALESCE(ki.content, '') || ' ' || COALESCE(ki.summary, '') || ' ' ||
                  COALESCE(fd.composition, '') || ' ' || COALESCE(fd.indications, '') || ' ' ||
                  COALESCE(ad.indications, '') || ' ' || COALESCE(ad.functions, '') || ' ' ||
                  COALESCE(dd.symptoms, '') || ' ' || COALESCE(dd.common_syndromes, '') || ' ' ||
                  COALESCE(dd.care_advice, '') || ' ' || COALESCE(dd.notes, '')
                ) AS suggest_text,
                ad.meridian_item_id
         FROM knowledge_items ki
         LEFT JOIN formula_details fd ON fd.item_id = ki.id
         LEFT JOIN acupoint_details ad ON ad.item_id = ki.id
         LEFT JOIN disease_details dd ON dd.item_id = ki.id
         {where_sql}"
    );
    let mut statement = connection.prepare(&sql)?;
    let rows = match args.len() {
        0 => statement.query_map([], map_source_row)?,
        1 => statement.query_map(params![args[0]], map_source_row)?,
        _ => statement.query_map(params![args[0], args[1]], map_source_row)?,
    };
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn update_relation_count_cache_tx(
    connection: &Connection,
    item_id: i64,
    relation_type: &str,
) -> AppResult<()> {
    connection.execute(
        "INSERT INTO relation_count_cache (item_id, relation_type, count, updated_at)
         VALUES (
           ?1,
           ?2,
           (
             SELECT COUNT(1)
             FROM (
               SELECT id FROM knowledge_relations
               WHERE source_item_id = ?1 AND relation_type = ?2
               UNION ALL
               SELECT id FROM knowledge_relations
               WHERE target_item_id = ?1 AND relation_type = ?2
             )
           ),
           datetime('now')
         )
         ON CONFLICT(item_id, relation_type) DO UPDATE SET
           count = excluded.count,
           updated_at = excluded.updated_at",
        params![item_id, relation_type],
    )?;
    Ok(())
}

fn map_source_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelationSource> {
    Ok(RelationSource {
        item_id: row.get(0)?,
        item_type: row.get(1)?,
        name: row.get(2)?,
        code: row.get(3)?,
        alias: row.get(4)?,
        category: row.get(5)?,
        content: row.get(6)?,
        meridian_item_id: row.get(7)?,
    })
}

fn map_relation_view(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeRelationView> {
    Ok(KnowledgeRelationView {
        id: row.get(0)?,
        source_item_id: row.get(1)?,
        target_item_id: row.get(2)?,
        related_item_id: row.get(3)?,
        related_item_type: row.get(4)?,
        related_name: row.get(5)?,
        related_code: row.get(6)?,
        related_category: row.get(7)?,
        relation_type: row.get(8)?,
        direction: row.get(9)?,
        note: row.get(10)?,
    })
}

fn map_suggestion_detail(row: &rusqlite::Row<'_>) -> rusqlite::Result<RelationSuggestionDetail> {
    Ok(RelationSuggestionDetail {
        id: row.get(0)?,
        source_item_id: row.get(1)?,
        target_item_id: row.get(2)?,
        source_name: row.get(3)?,
        target_name: row.get(4)?,
        relation_type: row.get(5)?,
        confidence: row.get(6)?,
        reason: row.get(7)?,
        status: row.get(8)?,
        created_at: row.get(9)?,
    })
}
