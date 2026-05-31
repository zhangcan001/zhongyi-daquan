use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::knowledge::KnowledgeVersion;
use crate::repositories::{knowledge_repository, version_repository};
use crate::services::search_index_service;
use serde_json::Value;

#[allow(dead_code)]
/// 创建版本快照（在 update 时自动调用）
pub fn create_version_snapshot(
    database: &Database,
    item_id: i64,
    snapshot_json: &str,
    change_summary: Option<&str>,
) -> AppResult<()> {
    database.with_connection(|connection| {
        // 获取当前最大版本号
        let max_version: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(version_no), 0) FROM knowledge_versions WHERE item_id = ?1",
                [item_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let next_version = max_version + 1;

        version_repository::insert_version_tx(
            connection,
            item_id,
            next_version,
            snapshot_json,
            change_summary,
        )
    })
}

/// 列出某个知识条目的所有版本
pub fn list_versions(database: &Database, item_id: i64) -> AppResult<Vec<KnowledgeVersion>> {
    version_repository::list_versions(database, item_id)
}

/// 获取特定版本的快照
pub fn get_version(database: &Database, version_id: i64) -> AppResult<KnowledgeVersion> {
    database.with_connection(|connection| {
        connection.query_row(
            "SELECT id, item_id, version_no, snapshot_json, change_summary, changed_at
             FROM knowledge_versions
             WHERE id = ?1",
            [version_id],
            |row| {
                Ok(KnowledgeVersion {
                    id: row.get(0)?,
                    item_id: row.get(1)?,
                    version_no: row.get(2)?,
                    snapshot_json: row.get(3)?,
                    change_summary: row.get(4)?,
                    changed_at: row.get(5)?,
                })
            },
        )
        .map_err(Into::into)
    })
}

/// 对比两个版本的差异
pub fn compare_versions(
    database: &Database,
    version_id_a: i64,
    version_id_b: i64,
) -> AppResult<VersionComparison> {
    let version_a = get_version(database, version_id_a)?;
    let version_b = get_version(database, version_id_b)?;

    let snapshot_a: Value = serde_json::from_str(&version_a.snapshot_json)?;
    let snapshot_b: Value = serde_json::from_str(&version_b.snapshot_json)?;

    let mut differences = Vec::new();

    // 简单的字段级对比
    if let (Some(obj_a), Some(obj_b)) = (snapshot_a.as_object(), snapshot_b.as_object()) {
        for (key, value_a) in obj_a {
            let value_b = obj_b.get(key);
            if value_b != Some(value_a) {
                differences.push(FieldDifference {
                    field_name: key.clone(),
                    old_value: value_b.map(|v| v.to_string()),
                    new_value: Some(value_a.to_string()),
                });
            }
        }

        // 检查 B 中有但 A 中没有的字段
        for (key, value_b) in obj_b {
            if !obj_a.contains_key(key) {
                differences.push(FieldDifference {
                    field_name: key.clone(),
                    old_value: Some(value_b.to_string()),
                    new_value: None,
                });
            }
        }
    }

    Ok(VersionComparison {
        version_a,
        version_b,
        differences,
    })
}

/// 回滚到历史版本
pub fn rollback_to_version(database: &Database, version_id: i64) -> AppResult<i64> {
    let version = get_version(database, version_id)?;
    let snapshot: Value = serde_json::from_str(&version.snapshot_json)?;

    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;

        // 从快照恢复数据
        knowledge_repository::update_from_snapshot(&transaction, version.item_id, &snapshot)?;

        // 创建新版本记录（标记为回滚）
        let max_version: i64 = transaction
            .query_row(
                "SELECT COALESCE(MAX(version_no), 0) FROM knowledge_versions WHERE item_id = ?1",
                [version.item_id],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let next_version = max_version + 1;
        let change_summary = format!("回滚到版本 {}", version.version_no);

        version_repository::insert_version_tx(
            &transaction,
            version.item_id,
            next_version,
            &version.snapshot_json,
            Some(&change_summary),
        )?;

        transaction.commit()?;

        Ok(version.item_id)
    })?;

    // 回滚后重建该条目的搜索索引
    search_index_service::rebuild_item_index(database, version.item_id)?;

    Ok(version.item_id)
}

#[derive(Debug, serde::Serialize)]
pub struct VersionComparison {
    pub version_a: KnowledgeVersion,
    pub version_b: KnowledgeVersion,
    pub differences: Vec<FieldDifference>,
}

#[derive(Debug, serde::Serialize)]
pub struct FieldDifference {
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}
