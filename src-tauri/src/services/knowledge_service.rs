use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::knowledge::{
    KnowledgeDetailResponse, KnowledgeInput, KnowledgeListRequest, KnowledgeListResponse,
};
use crate::repositories::{detail_repository, knowledge_repository, version_repository};
use crate::services::search_index_service;
use serde_json::json;

const KNOWLEDGE_TYPES: &[&str] = &[
    "herb", "formula", "meridian", "acupoint", "syndrome", "disease",
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
        transaction.commit()?;
        Ok(())
    })?;
    let _ = search_index_service::index_knowledge_item(database, item_id);
    get(database, item_id)
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
