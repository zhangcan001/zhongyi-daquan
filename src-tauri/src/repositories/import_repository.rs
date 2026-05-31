use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::data_pipeline::{DataImportBatch, DataImportRow, ImportBatchSummary};
use chrono::Utc;
use rusqlite::params;

pub fn create_batch(
    database: &Database,
    file_name: &str,
    import_type: &str,
    target_type: &str,
    total_count: i64,
) -> AppResult<DataImportBatch> {
    let now = Utc::now().to_rfc3339();
    database.with_connection(|connection| {
        connection.execute(
            "INSERT INTO data_import_batches
             (file_name, import_type, target_type, status, total_count, parsed_count, valid_count, warning_count, error_count, created_at)
             VALUES (?1, ?2, ?3, 'staged', ?4, ?4, 0, 0, 0, ?5)",
            params![file_name, import_type, target_type, total_count, now],
        )?;
        let id = connection.last_insert_rowid();
        Ok(DataImportBatch {
            id: Some(id),
            file_name: file_name.to_string(),
            import_type: import_type.to_string(),
            target_type: target_type.to_string(),
            status: "staged".to_string(),
            total_count,
            parsed_count: total_count,
            valid_count: 0,
            warning_count: 0,
            error_count: 0,
            created_at: now,
        })
    })
}
#[allow(dead_code)]

pub fn insert_row(
    database: &Database,
    batch_id: i64,
    row_index: i64,
    raw_json: &str,
    mapped_json: &str,
    normalized_json: &str,
    status: &str,
    error_message: Option<&str>,
    warning_message: Option<&str>,
) -> AppResult<i64> {
    database.with_connection(|connection| {
        connection.execute(
            "INSERT INTO data_import_rows
             (batch_id, row_index, raw_json, mapped_json, normalized_json, status, error_message, warning_message)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                batch_id,
                row_index,
                raw_json,
                mapped_json,
                normalized_json,
                status,
                error_message,
                warning_message
            ],
        )?;
        Ok(connection.last_insert_rowid())
    })
}

pub struct ImportRowData {
    pub row_index: i64,
    pub raw_json: String,
    pub mapped_json: String,
    pub normalized_json: String,
    pub status: String,
    pub error_message: Option<String>,
    pub warning_message: Option<String>,
}

pub fn insert_rows_batch(
    database: &Database,
    batch_id: i64,
    rows: Vec<ImportRowData>,
) -> AppResult<Vec<i64>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    database.with_connection(|connection| {
        let mut row_ids = Vec::with_capacity(rows.len());

        for row in rows {
            connection.execute(
                "INSERT INTO data_import_rows
                 (batch_id, row_index, raw_json, mapped_json, normalized_json, status, error_message, warning_message)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    batch_id,
                    row.row_index,
                    row.raw_json,
                    row.mapped_json,
                    row.normalized_json,
                    row.status,
                    row.error_message,
                    row.warning_message
                ],
            )?;
            row_ids.push(connection.last_insert_rowid());
        }

        Ok(row_ids)
    })
}

pub fn get_batch(database: &Database, batch_id: i64) -> AppResult<DataImportBatch> {
    database.with_connection(|connection| {
        let batch = connection.query_row(
            "SELECT id, file_name, import_type, target_type, status, total_count, parsed_count,
                    valid_count, warning_count, error_count, created_at
             FROM data_import_batches WHERE id = ?1",
            [batch_id],
            batch_from_row,
        )?;
        Ok(batch)
    })
}

pub fn list_rows(
    database: &Database,
    batch_id: i64,
    page: i64,
    page_size: i64,
) -> AppResult<Vec<DataImportRow>> {
    let limit = page_size.clamp(1, 200);
    let offset = (page.max(1) - 1) * limit;
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, batch_id, row_index, raw_json, mapped_json, normalized_json, status, error_message, warning_message
             FROM data_import_rows
             WHERE batch_id = ?1
             ORDER BY row_index
             LIMIT ?2 OFFSET ?3",
        )?;
        let rows = statement
            .query_map(params![batch_id, limit, offset], row_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

pub fn list_all_rows(database: &Database, batch_id: i64) -> AppResult<Vec<DataImportRow>> {
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT id, batch_id, row_index, raw_json, mapped_json, normalized_json, status, error_message, warning_message
             FROM data_import_rows WHERE batch_id = ?1 ORDER BY row_index",
        )?;
        let rows = statement
            .query_map([batch_id], row_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

pub fn update_row_normalized(
    database: &Database,
    row_id: i64,
    normalized_json: &str,
    status: &str,
    error_message: Option<&str>,
    warning_message: Option<&str>,
) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute(
            "UPDATE data_import_rows
             SET normalized_json = ?1, status = ?2, error_message = ?3, warning_message = ?4
             WHERE id = ?5",
            params![
                normalized_json,
                status,
                error_message,
                warning_message,
                row_id
            ],
        )?;
        Ok(())
    })
}

pub fn update_batch_counts(database: &Database, batch_id: i64, status: &str) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute(
            "UPDATE data_import_batches
             SET status = ?2,
                 valid_count = (SELECT COUNT(1) FROM data_import_rows WHERE batch_id = ?1 AND status IN ('valid','warning','imported')),
                 warning_count = (SELECT COUNT(1) FROM data_import_rows WHERE batch_id = ?1 AND status = 'warning'),
                 error_count = (SELECT COUNT(1) FROM data_import_rows WHERE batch_id = ?1 AND status = 'error')
             WHERE id = ?1",
            params![batch_id, status],
        )?;
        Ok(())
    })
}

pub fn summary(database: &Database, batch_id: i64) -> AppResult<ImportBatchSummary> {
    let batch = get_batch(database, batch_id)?;
    Ok(ImportBatchSummary {
        total_rows: batch.total_count,
        importable_rows: batch.valid_count,
        warning_rows: batch.warning_count,
        error_rows: batch.error_count,
        batch,
    })
}

fn batch_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DataImportBatch> {
    Ok(DataImportBatch {
        id: Some(row.get(0)?),
        file_name: row.get(1)?,
        import_type: row.get(2)?,
        target_type: row.get(3)?,
        status: row.get(4)?,
        total_count: row.get(5)?,
        parsed_count: row.get(6)?,
        valid_count: row.get(7)?,
        warning_count: row.get(8)?,
        error_count: row.get(9)?,
        created_at: row.get(10)?,
    })
}

fn row_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DataImportRow> {
    Ok(DataImportRow {
        id: Some(row.get(0)?),
        batch_id: row.get(1)?,
        row_index: row.get(2)?,
        raw_json: row.get(3)?,
        mapped_json: row.get(4)?,
        normalized_json: row.get(5)?,
        status: row.get(6)?,
        error_message: row.get(7)?,
        warning_message: row.get(8)?,
    })
}
