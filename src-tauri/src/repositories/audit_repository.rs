use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::runtime::AuditLog;
use rusqlite::params;

pub fn record(
    database: &Database,
    action: &str,
    target_type: Option<&str>,
    target_id: Option<i64>,
    before_json: Option<&str>,
    after_json: Option<&str>,
) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute(
            "INSERT INTO audit_logs (action, target_type, target_id, before_json, after_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![action, target_type, target_id, before_json, after_json],
        )?;
        Ok(())
    })
}

pub fn list_recent(database: &Database, limit: u32) -> AppResult<Vec<AuditLog>> {
    let limit = limit.clamp(1, 200);
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, action, target_type, target_id, before_json, after_json, created_at
             FROM audit_logs
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit], |row| {
            Ok(AuditLog {
                id: row.get(0)?,
                action: row.get(1)?,
                target_type: row.get(2)?,
                target_id: row.get(3)?,
                before_json: row.get(4)?,
                after_json: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}
