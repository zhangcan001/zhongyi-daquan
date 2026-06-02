use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::runtime::BackgroundJob;
use rusqlite::params;

const ALLOWED_JOB_TYPES: &[&str] = &[
    "import_batch",
    "clean_batch",
    "dedup_batch",
    "relation_suggest_batch",
    "rebuild_search_index",
    "backup",
    "restore",
    "ai_task",
    "clear_database_content",
];

pub fn is_allowed_job_type(job_type: &str) -> bool {
    ALLOWED_JOB_TYPES.contains(&job_type)
}

pub fn create(
    database: &Database,
    job_type: &str,
    params_json: Option<&str>,
) -> AppResult<BackgroundJob> {
    database.with_connection(|connection| {
        connection.execute(
            "INSERT INTO background_jobs (job_type, status, progress, params_json, created_at, updated_at)
             VALUES (?1, 'pending', 0, ?2, datetime('now'), datetime('now'))",
            params![job_type, params_json],
        )?;
        let id = connection.last_insert_rowid();
        get_by_connection(connection, id)
    })
}

pub fn update_progress(
    database: &Database,
    job_id: i64,
    progress: f64,
    result_json: Option<&str>,
) -> AppResult<BackgroundJob> {
    database.with_connection(|connection| {
        connection.execute(
            "UPDATE background_jobs
             SET status = CASE WHEN status = 'pending' THEN 'running' ELSE status END,
                 progress = ?2,
                 result_json = COALESCE(?3, result_json),
                 updated_at = datetime('now')
             WHERE id = ?1",
            params![job_id, progress, result_json],
        )?;
        get_by_connection(connection, job_id)
    })
}

pub fn mark_success(
    database: &Database,
    job_id: i64,
    result_json: Option<&str>,
) -> AppResult<BackgroundJob> {
    database.with_connection(|connection| {
        connection.execute(
            "UPDATE background_jobs
             SET status = 'success', progress = 100, result_json = ?2, error_message = NULL, updated_at = datetime('now')
             WHERE id = ?1",
            params![job_id, result_json],
        )?;
        get_by_connection(connection, job_id)
    })
}

pub fn mark_failed(
    database: &Database,
    job_id: i64,
    error_message: &str,
) -> AppResult<BackgroundJob> {
    database.with_connection(|connection| {
        connection.execute(
            "UPDATE background_jobs
             SET status = 'failed', error_message = ?2, updated_at = datetime('now')
             WHERE id = ?1",
            params![job_id, error_message],
        )?;
        get_by_connection(connection, job_id)
    })
}

pub fn list(
    database: &Database,
    status: Option<&str>,
    job_type: Option<&str>,
    limit: u32,
) -> AppResult<Vec<BackgroundJob>> {
    let limit = limit.clamp(1, 200);
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, job_type, status, progress, params_json, result_json, error_message, created_at, updated_at
             FROM background_jobs
             WHERE (?1 IS NULL OR status = ?1)
               AND (?2 IS NULL OR job_type = ?2)
             ORDER BY id DESC
             LIMIT ?3",
        )?;
        let rows = statement.query_map(params![status, job_type, limit], map_job)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}

pub fn get(database: &Database, job_id: i64) -> AppResult<BackgroundJob> {
    database.with_connection(|connection| get_by_connection(connection, job_id))
}

fn get_by_connection(connection: &rusqlite::Connection, job_id: i64) -> AppResult<BackgroundJob> {
    connection
        .query_row(
            "SELECT id, job_type, status, progress, params_json, result_json, error_message, created_at, updated_at
             FROM background_jobs
             WHERE id = ?1",
            params![job_id],
            map_job,
        )
        .map_err(Into::into)
}

fn map_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<BackgroundJob> {
    Ok(BackgroundJob {
        id: row.get(0)?,
        job_type: row.get(1)?,
        status: row.get(2)?,
        progress: row.get(3)?,
        params_json: row.get(4)?,
        result_json: row.get(5)?,
        error_message: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}
