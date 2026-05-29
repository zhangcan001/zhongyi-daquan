use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::search::PerformanceLogEntry;
use rusqlite::params;

pub fn record(
    database: &Database,
    action: &str,
    duration_ms: i64,
    row_count: Option<i64>,
    query_type: Option<&str>,
) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute(
            "INSERT INTO performance_logs (action, duration_ms, row_count, query_type, created_at)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![action, duration_ms, row_count, query_type],
        )?;
        Ok(())
    })
}

pub fn list_recent(database: &Database, limit: u32) -> AppResult<Vec<PerformanceLogEntry>> {
    let limit = limit.clamp(1, 200);
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, action, duration_ms, row_count, query_type, created_at
             FROM performance_logs
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit], |row| {
            Ok(PerformanceLogEntry {
                id: row.get(0)?,
                action: row.get(1)?,
                duration_ms: row.get(2)?,
                row_count: row.get(3)?,
                query_type: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}

pub fn delete_older_than_days(database: &Database, keep_days: u32) -> AppResult<i64> {
    let keep_days = keep_days.clamp(1, 3650);
    database.with_connection(|connection| {
        let affected = connection.execute(
            "DELETE FROM performance_logs
             WHERE datetime(created_at) < datetime('now', ?1)",
            params![format!("-{keep_days} days")],
        )?;
        Ok(affected as i64)
    })
}

pub fn list_all_for_report(database: &Database, limit: u32) -> AppResult<Vec<PerformanceLogEntry>> {
    let limit = limit.clamp(1, 10_000);
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, action, duration_ms, row_count, query_type, created_at
             FROM performance_logs
             ORDER BY id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map(params![limit], |row| {
            Ok(PerformanceLogEntry {
                id: row.get(0)?,
                action: row.get(1)?,
                duration_ms: row.get(2)?,
                row_count: row.get(3)?,
                query_type: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}
