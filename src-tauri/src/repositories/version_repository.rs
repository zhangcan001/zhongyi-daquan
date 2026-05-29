use crate::errors::AppResult;
use crate::models::knowledge::KnowledgeVersion;
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

pub fn list_versions(connection: &Connection, item_id: i64) -> AppResult<Vec<KnowledgeVersion>> {
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
}
