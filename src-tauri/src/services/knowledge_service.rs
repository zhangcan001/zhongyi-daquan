use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::knowledge::{
    DashboardStats, FavoriteItem, KnowledgeAnnotation, KnowledgeDetailResponse, KnowledgeInput,
    KnowledgeListRequest, KnowledgeListResponse, RecentView, UserNote,
};
use crate::repositories::{detail_repository, knowledge_repository, version_repository};
use crate::services::search_index_service;
use rusqlite::{params, OptionalExtension};
use serde_json::json;

const KNOWLEDGE_TYPES: &[&str] = &[
    "herb",
    "formula",
    "meridian",
    "acupoint",
    "acupuncture",
    "syndrome",
    "disease",
    "theory",
    "note",
];
const DATA_STATUSES: &[&str] = &[
    "draft",
    "imported",
    "needs_fix",
    "validated",
    "ready",
    "archived",
];
const COMPLETENESS_STATUSES: &[&str] = &["empty", "partial", "complete"];

pub fn list(
    database: &Database,
    request: KnowledgeListRequest,
) -> AppResult<KnowledgeListResponse> {
    let page = request.page.unwrap_or(1).max(1);
    let page_size = request.page_size.unwrap_or(50).clamp(1, 200);
    let (total, items) = knowledge_repository::list(database, &request)?;
    Ok(KnowledgeListResponse {
        total,
        page,
        page_size,
        items,
    })
}

pub fn get(database: &Database, item_id: i64) -> AppResult<KnowledgeDetailResponse> {
    let (item, detail) = database.with_connection(|connection| {
        let item = knowledge_repository::get_by_id(connection, item_id)?
            .ok_or_else(|| AppError::InvalidInput(format!("知识条目不存在: {item_id}")))?;
        let detail = detail_repository::get_detail(connection, &item.item_type, item_id)?;
        Ok((item, detail))
    })?;

    let versions = version_repository::list_versions(database, item_id)?;

    Ok(KnowledgeDetailResponse {
        item,
        detail,
        annotations: list_annotations(database, item_id)?,
        notes: list_notes_for_item(database, item_id)?,
        versions,
    })
}

pub fn create(database: &Database, input: KnowledgeInput) -> AppResult<KnowledgeDetailResponse> {
    validate_input(&input)?;
    let item_id = database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        let item_id = knowledge_repository::insert_tx(&transaction, &input)?;
        detail_repository::upsert_detail_tx(
            &transaction,
            &input.item_type,
            item_id,
            &input.detail,
        )?;
        transaction.commit()?;
        Ok(item_id)
    })?;
    let _ = search_index_service::index_knowledge_item(database, item_id);
    get(database, item_id)
}

pub fn update(
    database: &Database,
    item_id: i64,
    input: KnowledgeInput,
) -> AppResult<KnowledgeDetailResponse> {
    validate_input(&input)?;
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        let current = knowledge_repository::get_by_id(&transaction, item_id)?
            .ok_or_else(|| AppError::InvalidInput(format!("知识条目不存在: {item_id}")))?;
        let current_detail =
            detail_repository::get_detail(&transaction, &current.item_type, item_id)?;
        let snapshot = json!({
            "item": current,
            "detail": current_detail
        })
        .to_string();
        version_repository::insert_version_tx(
            &transaction,
            item_id,
            current.content_version,
            &snapshot,
            Some("手动编辑前自动保存"),
        )?;
        knowledge_repository::update_tx(&transaction, item_id, &input)?;
        if current.item_type != input.item_type {
            detail_repository::delete_detail_tx(&transaction, &current.item_type, item_id)?;
        }
        detail_repository::upsert_detail_tx(
            &transaction,
            &input.item_type,
            item_id,
            &input.detail,
        )?;
        transaction.commit()?;
        Ok(())
    })?;
    let _ = search_index_service::index_knowledge_item(database, item_id);
    get(database, item_id)
}

pub fn set_favorite(
    database: &Database,
    item_id: i64,
    is_favorite: bool,
) -> AppResult<KnowledgeDetailResponse> {
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        knowledge_repository::set_favorite_tx(&transaction, item_id, is_favorite)?;
        sync_favorite_tx(&transaction, item_id, is_favorite)?;
        transaction.commit()?;
        Ok(())
    })?;
    let _ = search_index_service::index_knowledge_item(database, item_id);
    get(database, item_id)
}

pub fn toggle_favorite(database: &Database, item_id: i64) -> AppResult<KnowledgeDetailResponse> {
    let next = database.with_connection(|connection| {
        let current: i64 = connection.query_row(
            "SELECT is_favorite FROM knowledge_items WHERE id = ?1",
            params![item_id],
            |row| row.get(0),
        )?;
        Ok(current != 1)
    })?;
    set_favorite(database, item_id, next)
}

pub fn list_favorites(database: &Database) -> AppResult<Vec<FavoriteItem>> {
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, item_id, item_name, item_type, category, created_at
             FROM user_favorites
             ORDER BY created_at DESC
             LIMIT 50",
        )?;
        let rows = statement.query_map([], map_favorite_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}

pub fn record_recent_view(database: &Database, item_id: i64) -> AppResult<RecentView> {
    database.with_connection(|connection| {
        let item = knowledge_repository::get_by_id(connection, item_id)?
            .ok_or_else(|| AppError::InvalidInput(format!("知识条目不存在: {item_id}")))?;
        connection.execute(
            "INSERT INTO recent_views (item_id, item_name, item_type, category, viewed_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))
             ON CONFLICT(item_id) DO UPDATE SET
               item_name = excluded.item_name,
               item_type = excluded.item_type,
               category = excluded.category,
               viewed_at = excluded.viewed_at",
            params![item_id, item.name, item.item_type, item.category],
        )?;
        load_recent_view(connection, item_id)
    })
}

pub fn list_recent_views(database: &Database, limit: Option<i64>) -> AppResult<Vec<RecentView>> {
    let limit = limit.unwrap_or(10).clamp(1, 50);
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, item_id, item_name, item_type, category, viewed_at
             FROM recent_views
             ORDER BY viewed_at DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit], map_recent_view_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}

pub fn save_user_note(database: &Database, item_id: i64, note_text: String) -> AppResult<UserNote> {
    let note_text = note_text.trim().to_string();
    if note_text.is_empty() {
        return Err(AppError::InvalidInput("备注内容不能为空".to_string()));
    }
    database.with_connection(|connection| {
        let exists: Option<i64> = connection
            .query_row(
                "SELECT id FROM knowledge_items WHERE id = ?1",
                params![item_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            return Err(AppError::InvalidInput(format!("知识条目不存在: {item_id}")));
        }
        connection.execute(
            "INSERT INTO user_notes (item_id, note_text, created_at, updated_at)
             VALUES (?1, ?2, datetime('now'), datetime('now'))
             ON CONFLICT(item_id) DO UPDATE SET
               note_text = excluded.note_text,
               updated_at = excluded.updated_at",
            params![item_id, note_text],
        )?;
        load_note_for_item(connection, item_id)
    })
}

pub fn delete_user_note(database: &Database, note_id: i64) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute("DELETE FROM user_notes WHERE id = ?1", params![note_id])?;
        Ok(())
    })
}

pub fn dashboard_stats(database: &Database) -> AppResult<DashboardStats> {
    database.with_connection(|connection| {
        Ok(DashboardStats {
            knowledge_count: count_table(connection, "knowledge_items")?,
            annotation_count: count_table(connection, "knowledge_annotations")?,
            import_run_count: count_table(connection, "import_runs")?,
            favorite_count: count_table(connection, "user_favorites")?,
            recent_view_count: count_table(connection, "recent_views")?,
        })
    })
}

pub fn delete(database: &Database, item_id: i64) -> AppResult<()> {
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        let current = knowledge_repository::get_by_id(&transaction, item_id)?
            .ok_or_else(|| AppError::InvalidInput(format!("知识条目不存在: {item_id}")))?;
        let current_detail =
            detail_repository::get_detail(&transaction, &current.item_type, item_id)?;
        let snapshot = json!({ "item": current, "detail": current_detail }).to_string();
        version_repository::insert_version_tx(
            &transaction,
            item_id,
            current.content_version,
            &snapshot,
            Some("删除前自动保存"),
        )?;
        knowledge_repository::delete_tx(&transaction, item_id)?;
        transaction.commit()?;
        Ok(())
    })?;
    let _ = search_index_service::delete_knowledge_item_index(database, item_id);
    Ok(())
}

fn validate_input(input: &KnowledgeInput) -> AppResult<()> {
    if !KNOWLEDGE_TYPES.contains(&input.item_type.as_str()) {
        return Err(AppError::InvalidInput(format!(
            "不支持的知识类型: {}",
            input.item_type
        )));
    }
    if input.name.trim().is_empty() {
        return Err(AppError::InvalidInput("名称不能为空".to_string()));
    }
    if !DATA_STATUSES.contains(&input.data_status.as_str()) {
        return Err(AppError::InvalidInput(format!(
            "不支持的数据状态: {}",
            input.data_status
        )));
    }
    if !COMPLETENESS_STATUSES.contains(&input.completeness_status.as_str()) {
        return Err(AppError::InvalidInput(format!(
            "不支持的完整度状态: {}",
            input.completeness_status
        )));
    }
    Ok(())
}

pub fn batch_delete(database: &Database, item_ids: Vec<i64>) -> AppResult<BatchOperationResult> {
    let mut success_count = 0;
    let mut failed_ids = Vec::new();

    for item_id in item_ids {
        match delete(database, item_id) {
            Ok(_) => success_count += 1,
            Err(_) => failed_ids.push(item_id),
        }
    }

    Ok(BatchOperationResult {
        success_count,
        failed_count: failed_ids.len(),
        failed_ids,
    })
}

pub fn batch_update_status(
    database: &Database,
    item_ids: Vec<i64>,
    data_status: String,
) -> AppResult<BatchOperationResult> {
    if !DATA_STATUSES.contains(&data_status.as_str()) {
        return Err(AppError::InvalidInput(format!(
            "不支持的数据状态: {}",
            data_status
        )));
    }

    let mut success_count = 0;
    let mut failed_ids = Vec::new();

    database.with_connection(|connection| {
        for item_id in item_ids {
            let result = connection.execute(
                "UPDATE knowledge_items SET data_status = ?1, updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params![data_status, item_id],
            );

            match result {
                Ok(_) => success_count += 1,
                Err(_) => failed_ids.push(item_id),
            }
        }
        Ok(())
    })?;

    Ok(BatchOperationResult {
        success_count,
        failed_count: failed_ids.len(),
        failed_ids,
    })
}

pub fn batch_add_tags(
    database: &Database,
    item_ids: Vec<i64>,
    tags_to_add: Vec<String>,
) -> AppResult<BatchOperationResult> {
    let mut success_count = 0;
    let mut failed_ids = Vec::new();

    database.with_connection(|connection| {
        for item_id in &item_ids {
            let current_tags: Option<String> = connection
                .query_row(
                    "SELECT tags FROM knowledge_items WHERE id = ?1",
                    [item_id],
                    |row| row.get(0),
                )
                .ok();

            let mut tag_set: std::collections::HashSet<String> = current_tags
                .unwrap_or_default()
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string())
                .collect();

            for tag in &tags_to_add {
                tag_set.insert(tag.trim().to_string());
            }

            let new_tags: Vec<String> = tag_set.into_iter().collect();
            let new_tags_str = new_tags.join(",");

            let result = connection.execute(
                "UPDATE knowledge_items SET tags = ?1, updated_at = datetime('now') WHERE id = ?2",
                rusqlite::params![new_tags_str, item_id],
            );

            match result {
                Ok(_) => success_count += 1,
                Err(_) => failed_ids.push(*item_id),
            }
        }
        Ok(())
    })?;

    Ok(BatchOperationResult {
        success_count,
        failed_count: failed_ids.len(),
        failed_ids,
    })
}

#[derive(Debug, serde::Serialize)]
pub struct BatchOperationResult {
    pub success_count: usize,
    pub failed_count: usize,
    pub failed_ids: Vec<i64>,
}

fn list_annotations(database: &Database, item_id: i64) -> AppResult<Vec<KnowledgeAnnotation>> {
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, knowledge_item_id, annotation_type, source_title, source_note,
                    content, detail_json, tags_json, created_at, updated_at
             FROM knowledge_annotations
             WHERE knowledge_item_id = ?1
             ORDER BY created_at DESC, id DESC",
        )?;
        let rows = statement.query_map(params![item_id], map_annotation_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}

fn list_notes_for_item(database: &Database, item_id: i64) -> AppResult<Vec<UserNote>> {
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, item_id, note_text, created_at, updated_at
             FROM user_notes
             WHERE item_id = ?1
             ORDER BY updated_at DESC",
        )?;
        let rows = statement.query_map(params![item_id], map_note_row)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}

fn sync_favorite_tx(
    connection: &rusqlite::Connection,
    item_id: i64,
    is_favorite: bool,
) -> AppResult<()> {
    if is_favorite {
        connection.execute(
            "INSERT INTO user_favorites (item_id, item_name, item_type, category, created_at)
             SELECT id, name, type, category, datetime('now')
             FROM knowledge_items
             WHERE id = ?1
             ON CONFLICT(item_id) DO UPDATE SET
               item_name = excluded.item_name,
               item_type = excluded.item_type,
               category = excluded.category",
            params![item_id],
        )?;
    } else {
        connection.execute(
            "DELETE FROM user_favorites WHERE item_id = ?1",
            params![item_id],
        )?;
    }
    Ok(())
}

fn load_recent_view(connection: &rusqlite::Connection, item_id: i64) -> AppResult<RecentView> {
    connection
        .query_row(
            "SELECT id, item_id, item_name, item_type, category, viewed_at
             FROM recent_views
             WHERE item_id = ?1",
            params![item_id],
            map_recent_view_row,
        )
        .map_err(Into::into)
}

fn load_note_for_item(connection: &rusqlite::Connection, item_id: i64) -> AppResult<UserNote> {
    connection
        .query_row(
            "SELECT id, item_id, note_text, created_at, updated_at
             FROM user_notes
             WHERE item_id = ?1",
            params![item_id],
            map_note_row,
        )
        .map_err(Into::into)
}

fn count_table(connection: &rusqlite::Connection, table_name: &str) -> AppResult<i64> {
    let sql = format!("SELECT COUNT(1) FROM {table_name}");
    connection
        .query_row(&sql, [], |row| row.get(0))
        .map_err(Into::into)
}

fn map_annotation_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeAnnotation> {
    let detail_text: Option<String> = row.get(6)?;
    Ok(KnowledgeAnnotation {
        id: row.get(0)?,
        knowledge_item_id: row.get(1)?,
        annotation_type: row.get(2)?,
        source_title: row.get(3)?,
        source_note: row.get(4)?,
        content: row.get(5)?,
        detail: detail_text.and_then(|text| serde_json::from_str(&text).ok()),
        tags: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn map_recent_view_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RecentView> {
    Ok(RecentView {
        id: row.get(0)?,
        item_id: row.get(1)?,
        item_name: row.get(2)?,
        item_type: row.get(3)?,
        category: row.get(4)?,
        viewed_at: row.get(5)?,
    })
}

fn map_favorite_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FavoriteItem> {
    Ok(FavoriteItem {
        id: row.get(0)?,
        item_id: row.get(1)?,
        item_name: row.get(2)?,
        item_type: row.get(3)?,
        category: row.get(4)?,
        created_at: row.get(5)?,
    })
}

fn map_note_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserNote> {
    Ok(UserNote {
        id: row.get(0)?,
        item_id: row.get(1)?,
        note_text: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn detail_recent_favorite_and_notes_work() {
        let data_dir = temp_data_dir("knowledge-ux");
        let database = Database::initialize(&data_dir).expect("database initializes");
        let item_id = seed_item(&database, "herb", "人参", Some("补气药"));

        database
            .with_connection(|connection| {
                connection
                    .execute(
                        "INSERT INTO knowledge_annotations
                         (knowledge_item_id, annotation_type, source_title, source_note, content, detail_json, tags_json, created_at, updated_at)
                         VALUES (?1, 'source_annotation', '3人纪-神农本草经.pdf', 'PDF页码26-27', '人参倪注资料', '{}', '倪注', datetime('now'), datetime('now'))",
                        params![item_id],
                    )
                    .unwrap();
                Ok(())
            })
            .unwrap();

        let detail = get(&database, item_id).unwrap();
        assert_eq!(detail.annotations.len(), 1);
        assert_eq!(
            detail.annotations[0].source_note.as_deref(),
            Some("PDF页码26-27")
        );

        record_recent_view(&database, item_id).unwrap();
        let recent = list_recent_views(&database, Some(5)).unwrap();
        assert_eq!(recent[0].item_name, "人参");

        let favorited = toggle_favorite(&database, item_id).unwrap();
        assert!(favorited.item.is_favorite);
        assert_eq!(list_favorites(&database).unwrap().len(), 1);
        let unfavorited = toggle_favorite(&database, item_id).unwrap();
        assert!(!unfavorited.item.is_favorite);
        assert!(list_favorites(&database).unwrap().is_empty());

        let note = save_user_note(&database, item_id, "学习备注".to_string()).unwrap();
        assert_eq!(
            get(&database, item_id).unwrap().notes[0].note_text,
            "学习备注"
        );
        delete_user_note(&database, note.id).unwrap();
        assert!(get(&database, item_id).unwrap().notes.is_empty());

        let stats = dashboard_stats(&database).unwrap();
        assert_eq!(stats.knowledge_count, 1);
        assert_eq!(stats.annotation_count, 1);
        assert_eq!(stats.recent_view_count, 1);

        let _ = fs::remove_dir_all(data_dir);
    }

    fn seed_item(database: &Database, item_type: &str, name: &str, category: Option<&str>) -> i64 {
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, category, summary, content, source_note, tags, data_status,
                      completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'imported', 'partial', 1, 0, '{}', datetime('now'), datetime('now'))",
                    params![
                        item_type,
                        name,
                        category,
                        format!("{name} 摘要"),
                        format!("{name} 正文"),
                        "测试来源",
                        "测试"
                    ],
                )?;
                Ok(connection.last_insert_rowid())
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
