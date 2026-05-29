use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::data_pipeline::{DataValidationIssue, StagingIssue};
use rusqlite::params;

pub fn replace_row_issues(
    database: &Database,
    batch_id: i64,
    row_id: i64,
    issues: &[StagingIssue],
) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute(
            "DELETE FROM data_validation_issues WHERE batch_id = ?1 AND row_id = ?2",
            params![batch_id, row_id],
        )?;
        for issue in issues {
            connection.execute(
                "INSERT INTO data_validation_issues
                 (batch_id, row_id, severity, issue_code, field_name, message, suggestion)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    batch_id,
                    row_id,
                    issue.severity,
                    issue.issue_code,
                    issue.field_name,
                    issue.message,
                    issue.suggestion
                ],
            )?;
        }
        Ok(())
    })
}

pub fn list_issues_for_rows(
    database: &Database,
    batch_id: i64,
    row_ids: &[i64],
) -> AppResult<Vec<DataValidationIssue>> {
    if row_ids.is_empty() {
        return Ok(Vec::new());
    }

    database.with_connection(|connection| {
        let placeholders = row_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT id, batch_id, row_id, severity, issue_code, field_name, message, suggestion
             FROM data_validation_issues
             WHERE batch_id = ? AND row_id IN ({placeholders})
             ORDER BY row_id, id"
        );
        let mut values: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(row_ids.len() + 1);
        values.push(&batch_id);
        for row_id in row_ids {
            values.push(row_id);
        }
        let mut statement = connection.prepare(&sql)?;
        let issues = statement
            .query_map(values.as_slice(), issue_from_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(issues)
    })
}

fn issue_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DataValidationIssue> {
    Ok(DataValidationIssue {
        id: Some(row.get(0)?),
        batch_id: row.get(1)?,
        row_id: row.get(2)?,
        severity: row.get(3)?,
        issue_code: row.get(4)?,
        field_name: row.get(5)?,
        message: row.get(6)?,
        suggestion: row.get(7)?,
    })
}
