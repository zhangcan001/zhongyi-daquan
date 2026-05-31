use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::repositories::version_repository;
use serde_json::{Map, Value};

pub fn create_version_snapshot(
    database: &Database,
    item_id: i64,
    snapshot_json: &str,
    change_summary: Option<&str>,
) -> AppResult<i64> {
    version_repository::create_snapshot(database, item_id, snapshot_json, change_summary)
}

pub fn list_versions(
    database: &Database,
    item_id: i64,
) -> AppResult<Vec<crate::models::knowledge::KnowledgeVersion>> {
    version_repository::list_versions(database, item_id)
}

pub fn get_version(
    database: &Database,
    version_id: i64,
) -> AppResult<crate::models::knowledge::KnowledgeVersion> {
    version_repository::get_version(database, version_id)
}

pub fn compare_versions(
    database: &Database,
    version_id_1: i64,
    version_id_2: i64,
) -> AppResult<VersionComparison> {
    let version1 = version_repository::get_version(database, version_id_1)?;
    let version2 = version_repository::get_version(database, version_id_2)?;

    let snapshot1: Map<String, Value> = serde_json::from_str(&version1.snapshot_json)?;
    let snapshot2: Map<String, Value> = serde_json::from_str(&version2.snapshot_json)?;

    let mut changes = Vec::new();

    // 找出所有字段
    let all_keys: std::collections::HashSet<String> = snapshot1
        .keys()
        .chain(snapshot2.keys())
        .cloned()
        .collect();

    for key in all_keys {
        let val1 = snapshot1.get(&key);
        let val2 = snapshot2.get(&key);

        if val1 != val2 {
            changes.push(FieldChange {
                field_name: key.clone(),
                old_value: val1.map(|v| v.to_string()),
                new_value: val2.map(|v| v.to_string()),
            });
        }
    }

    Ok(VersionComparison {
        version1,
        version2,
        changes,
    })
}

pub fn rollback_to_version(
    database: &Database,
    item_id: i64,
    version_id: i64,
) -> AppResult<()> {
    let version = version_repository::get_version(database, version_id)?;

    if version.item_id != item_id {
        return Err(crate::errors::AppError::InvalidInput(
            "版本不属于该知识条目".to_string(),
        ));
    }

    // 先创建当前状态的快照
    let current_item = crate::repositories::knowledge_repository::get_item(database, item_id)?;
    let current_snapshot = serde_json::to_string(&current_item)?;
    version_repository::create_snapshot(
        database,
        item_id,
        &current_snapshot,
        Some("回滚前的自动快照"),
    )?;

    // 恢复到指定版本
    let snapshot: Map<String, Value> = serde_json::from_str(&version.snapshot_json)?;
    crate::repositories::knowledge_repository::update_from_snapshot(database, item_id, &snapshot)?;

    // 创建回滚后的快照
    let rollback_summary = format!("回滚到版本 #{}", version.version_no);
    let rollback_snapshot = serde_json::to_string(&snapshot)?;
    version_repository::create_snapshot(database, item_id, &rollback_snapshot, Some(&rollback_summary))?;

    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct VersionComparison {
    pub version1: crate::models::knowledge::KnowledgeVersion,
    pub version2: crate::models::knowledge::KnowledgeVersion,
    pub changes: Vec<FieldChange>,
}

#[derive(Debug, serde::Serialize)]
pub struct FieldChange {
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}
