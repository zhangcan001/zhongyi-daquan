use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::data_pipeline::{FieldMappingTemplate, SaveMappingTemplateRequest};
use chrono::Utc;
use rusqlite::params;

pub fn insert_template(
    database: &Database,
    request: SaveMappingTemplateRequest,
) -> AppResult<FieldMappingTemplate> {
    let now = Utc::now().to_rfc3339();
    let source_headers_json = serde_json::to_string(&request.source_headers)?;
    let mapping_json = serde_json::to_string(&request.mapping)?;

    database.with_connection(|connection| {
        connection.execute(
            "INSERT INTO field_mapping_templates
             (name, target_type, source_headers_json, mapping_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                request.name,
                request.target_type,
                source_headers_json,
                mapping_json,
                now,
                now
            ],
        )?;
        let id = connection.last_insert_rowid();
        Ok(FieldMappingTemplate {
            id: Some(id),
            name: request.name,
            target_type: request.target_type,
            source_headers_json,
            mapping_json,
            created_at: now.clone(),
            updated_at: now,
        })
    })
}

pub fn list_templates(
    database: &Database,
    target_type: Option<String>,
) -> AppResult<Vec<FieldMappingTemplate>> {
    database.with_connection(|connection| {
        let sql = match target_type {
            Some(_) => {
                "SELECT id, name, target_type, source_headers_json, mapping_json, created_at, updated_at
                 FROM field_mapping_templates WHERE target_type = ?1 ORDER BY updated_at DESC"
            }
            None => {
                "SELECT id, name, target_type, source_headers_json, mapping_json, created_at, updated_at
                 FROM field_mapping_templates ORDER BY updated_at DESC"
            }
        };
        let mut statement = connection.prepare(sql)?;
        let rows = if let Some(target_type) = target_type {
            statement.query_map([target_type], template_from_row)?.collect::<Result<Vec<_>, _>>()?
        } else {
            statement.query_map([], template_from_row)?.collect::<Result<Vec<_>, _>>()?
        };
        Ok(rows)
    })
}

pub fn get_template(database: &Database, id: i64) -> AppResult<FieldMappingTemplate> {
    database.with_connection(|connection| {
        let template = connection.query_row(
            "SELECT id, name, target_type, source_headers_json, mapping_json, created_at, updated_at
             FROM field_mapping_templates WHERE id = ?1",
            [id],
            template_from_row,
        )?;
        Ok(template)
    })
}

fn template_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<FieldMappingTemplate> {
    Ok(FieldMappingTemplate {
        id: Some(row.get(0)?),
        name: row.get(1)?,
        target_type: row.get(2)?,
        source_headers_json: row.get(3)?,
        mapping_json: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}
