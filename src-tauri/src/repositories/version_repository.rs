use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::knowledge::KnowledgeVersion;
use chrono::Utc;
use rusqlite::{params, Connection};

pub fn insert_version_tx(
    connection: &Connection,
    item_id: i64,
    version_no: i64,
    snapshot_json: &str,
    change_summary: Option<&str>,
) -> AppResult<()> {
    connection.execute(
        "INSERT INTO knowledge_versions
         (item_id, version_no, snapshot_json, change_summary, changed_at)
         VALUES (?1, ?2, ?3, ?4, datetime('now'))",
        params![item_id, version_no, snapshot_json, change_summary],
    )?;
    Ok(())
}
#[allow(dead_code)]

pub fn create_snapshot(
    database: &Database,
    item_id: i64,
    snapshot_json: &str,
    change_summary: Option<&str>,
) -> AppResult<i64> {
    database.with_connection(|connection| {
        let next_version: i64 = connection.query_row(
            "SELECT COALESCE(MAX(version_no), 0) + 1 FROM knowledge_versions WHERE item_id = ?1",
            [item_id],
            |row| row.get(0),
        )?;

        let now = Utc::now().to_rfc3339();
        connection.execute(
            "INSERT INTO knowledge_versions
             (item_id, version_no, snapshot_json, change_summary, changed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![item_id, next_version, snapshot_json, change_summary, now],
        )?;

        Ok(connection.last_insert_rowid())
    })
}

pub fn list_versions(database: &Database, item_id: i64) -> AppResult<Vec<KnowledgeVersion>> {
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, item_id, version_no, snapshot_json, change_summary, changed_at
             FROM knowledge_versions
             WHERE item_id = ?1
             ORDER BY version_no DESC, id DESC",
        )?;
        let rows = statement.query_map(params![item_id], |row| {
            Ok(KnowledgeVersion {
                id: row.get(0)?,
                item_id: row.get(1)?,
                version_no: row.get(2)?,
                snapshot_json: row.get(3)?,
                change_summary: row.get(4)?,
                changed_at: row.get(5)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}
#[allow(dead_code)]

pub fn get_version(database: &Database, version_id: i64) -> AppResult<KnowledgeVersion> {
    database.with_connection(|connection| {
        connection
            .query_row(
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
