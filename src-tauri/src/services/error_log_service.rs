use crate::db::connection::Database;
use crate::errors::AppResult;
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn log_error(
    database: &Database,
    error_type: &str,
    error_message: &str,
    stack_trace: Option<&str>,
    context: Option<&str>,
) -> AppResult<i64> {
    let now = Utc::now().to_rfc3339();

    // 写入数据库
    let error_id = database.with_connection(|connection| {
        connection.execute(
            "INSERT INTO error_logs (error_type, error_message, stack_trace, context, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![error_type, error_message, stack_trace, context, now],
        )?;
        Ok(connection.last_insert_rowid())
    })?;

    // 同时写入文件日志
    write_to_file_log(error_type, error_message, stack_trace, context)?;

    Ok(error_id)
}

pub fn get_recent_errors(
    database: &Database,
    limit: i64,
) -> AppResult<Vec<ErrorLog>> {
    database.with_connection(|connection| {
        let mut stmt = connection.prepare(
            "SELECT id, error_type, error_message, stack_trace, context, created_at
             FROM error_logs
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(ErrorLog {
                id: row.get(0)?,
                error_type: row.get(1)?,
                error_message: row.get(2)?,
                stack_trace: row.get(3)?,
                context: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}

pub fn get_error_statistics(database: &Database) -> AppResult<ErrorStatistics> {
    database.with_connection(|connection| {
        let total_errors: i64 = connection.query_row(
            "SELECT COUNT(*) FROM error_logs",
            [],
            |row| row.get(0),
        )?;

        let errors_last_24h: i64 = connection.query_row(
            "SELECT COUNT(*) FROM error_logs
             WHERE created_at >= datetime('now', '-1 day')",
            [],
            |row| row.get(0),
        )?;

        let mut stmt = connection.prepare(
            "SELECT error_type, COUNT(*) as count
             FROM error_logs
             WHERE created_at >= datetime('now', '-7 days')
             GROUP BY error_type
             ORDER BY count DESC
             LIMIT 10",
        )?;

        let error_types = stmt
            .query_map([], |row| {
                Ok(ErrorTypeCount {
                    error_type: row.get(0)?,
                    count: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(ErrorStatistics {
            total_errors,
            errors_last_24h,
            top_error_types: error_types,
        })
    })
}

pub fn clear_old_error_logs(database: &Database, days: i64) -> AppResult<i64> {
    database.with_connection(|connection| {
        let deleted = connection.execute(
            "DELETE FROM error_logs WHERE created_at < datetime('now', ?1)",
            [format!("-{} days", days)],
        )?;
        Ok(deleted as i64)
    })
}

fn write_to_file_log(
    error_type: &str,
    error_message: &str,
    stack_trace: Option<&str>,
    context: Option<&str>,
) -> AppResult<()> {
    let log_dir = Path::new("logs");
    if !log_dir.exists() {
        fs::create_dir_all(log_dir)?;
    }

    let log_file = log_dir.join("error.log");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;

    let timestamp = Utc::now().to_rfc3339();
    writeln!(file, "[{}] [{}] {}", timestamp, error_type, error_message)?;

    if let Some(trace) = stack_trace {
        writeln!(file, "Stack Trace: {}", trace)?;
    }

    if let Some(ctx) = context {
        writeln!(file, "Context: {}", ctx)?;
    }

    writeln!(file, "---")?;

    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct ErrorLog {
    pub id: i64,
    pub error_type: String,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub context: Option<String>,
    pub created_at: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ErrorStatistics {
    pub total_errors: i64,
    pub errors_last_24h: i64,
    pub top_error_types: Vec<ErrorTypeCount>,
}

#[derive(Debug, serde::Serialize)]
pub struct ErrorTypeCount {
    pub error_type: String,
    pub count: i64,
}
