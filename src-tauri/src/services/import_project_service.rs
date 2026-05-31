use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::data_pipeline::{
    CleanStepRequest, CleanStepResult, ConfirmImportResult, CreateImportRequest,
    ImportBatchSummary, ImportParsedPreview, StagingIssue, StagingPage, StagingRowView,
};
use crate::repositories::{import_repository, validation_repository};
use crate::services::{
    field_mapping_service, normalize_service, search_index_service, validation_service,
};
use calamine::{open_workbook_from_rs, Data, Reader, Xlsx};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde_json::{Map, Value};
use std::collections::BTreeSet;

pub fn preview_json(content: &str) -> AppResult<ImportParsedPreview> {
    let rows = parse_json_rows(content)?;
    Ok(preview_from_rows(rows))
}

pub fn preview_csv(content: &str) -> AppResult<ImportParsedPreview> {
    let rows = parse_csv_rows(content)?;
    Ok(preview_from_rows(rows))
}

pub fn import_json(
    database: &Database,
    request: CreateImportRequest,
) -> AppResult<ImportBatchSummary> {
    import_rows(database, "json", request, parse_json_rows)
}

pub fn import_csv(
    database: &Database,
    request: CreateImportRequest,
) -> AppResult<ImportBatchSummary> {
    import_rows(database, "csv", request, parse_csv_rows)
}

pub fn preview_excel(content: &[u8]) -> AppResult<ImportParsedPreview> {
    let rows = parse_excel_rows(content)?;
    Ok(preview_from_rows(rows))
}

pub fn import_excel(
    database: &Database,
    request: CreateImportRequest,
) -> AppResult<ImportBatchSummary> {
    import_rows_from_bytes(database, "excel", request)
}

pub fn staging_page(
    database: &Database,
    batch_id: i64,
    page: i64,
    page_size: i64,
) -> AppResult<StagingPage> {
    let rows = import_repository::list_rows(database, batch_id, page, page_size)?;
    let row_ids = rows.iter().filter_map(|row| row.id).collect::<Vec<_>>();
    let issues = validation_repository::list_issues_for_rows(database, batch_id, &row_ids)?;

    let views = rows
        .into_iter()
        .map(|row| {
            let row_id = row.id.unwrap_or_default();
            let row_issues = issues
                .iter()
                .filter(|issue| issue.row_id == Some(row_id))
                .map(|issue| StagingIssue {
                    severity: issue.severity.clone(),
                    issue_code: issue.issue_code.clone(),
                    field_name: issue.field_name.clone(),
                    message: issue.message.clone(),
                    suggestion: issue.suggestion.clone(),
                })
                .collect();
            Ok(StagingRowView {
                id: row_id,
                row_index: row.row_index,
                raw: parse_optional_json(row.raw_json)?,
                mapped: parse_optional_json(row.mapped_json)?,
                normalized: parse_optional_json(row.normalized_json)?,
                status: row.status,
                error_message: row.error_message,
                warning_message: row.warning_message,
                issues: row_issues,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;

    Ok(StagingPage {
        summary: import_repository::summary(database, batch_id)?,
        rows: views,
        page,
        page_size: page_size.clamp(1, 200),
    })
}

pub fn validate_batch(database: &Database, batch_id: i64) -> AppResult<ImportBatchSummary> {
    let batch = import_repository::get_batch(database, batch_id)?;
    for row in import_repository::list_all_rows(database, batch_id)? {
        let row_id = row.id.unwrap_or_default();
        let normalized = parse_optional_json(row.normalized_json)?;
        let object = normalized.as_object().cloned().unwrap_or_default();
        let issues = validation_service::validate_row(database, &batch.target_type, &object)?;
        let (status, error_message, warning_message) =
            validation_service::status_from_issues(&issues);
        validation_repository::replace_row_issues(database, batch_id, row_id, &issues)?;
        import_repository::update_row_normalized(
            database,
            row_id,
            &serde_json::to_string(&Value::Object(object))?,
            status,
            error_message.as_deref(),
            warning_message.as_deref(),
        )?;
    }
    import_repository::update_batch_counts(database, batch_id, "validated")?;
    import_repository::summary(database, batch_id)
}

pub fn apply_clean_step(
    database: &Database,
    request: CleanStepRequest,
) -> AppResult<CleanStepResult> {
    let batch = import_repository::get_batch(database, request.batch_id)?;
    let rows = import_repository::list_all_rows(database, request.batch_id)?;
    let step_order = next_step_order(database, request.batch_id)?;
    let params_json = serde_json::to_string(&request.params)?;
    let mut changes = Vec::new();

    for row in rows {
        let row_id = row.id.unwrap_or_default();
        let before = parse_optional_json(row.normalized_json)?;
        let mut object = before.as_object().cloned().unwrap_or_default();
        let before_object = object.clone();
        object = match request.step_type.as_str() {
            "normalize_all" | "standardize_all" => {
                normalize_service::normalize_row(database, object)?
            }
            "trim" => clean_trim(object),
            "half_width" => clean_half_width(object),
            "uppercase_code" => clean_uppercase_code(object),
            "split_tags" => clean_split_tags(object),
            "standardize_meridian" => normalize_service::normalize_row(database, object)?,
            "set_category" => clean_set_category(object, request.params.as_ref()),
            _ => {
                return Err(AppError::InvalidInput(format!(
                    "不支持的清洗步骤: {}",
                    request.step_type
                )))
            }
        };

        for (field, old_value, new_value) in diff_objects(&before_object, &object) {
            changes.push((row_id, field, old_value, new_value));
        }

        if before_object != object {
            let issues = validation_service::validate_row(database, &batch.target_type, &object)?;
            let (status, error_message, warning_message) =
                validation_service::status_from_issues(&issues);
            validation_repository::replace_row_issues(database, request.batch_id, row_id, &issues)?;
            import_repository::update_row_normalized(
                database,
                row_id,
                &serde_json::to_string(&Value::Object(object))?,
                status,
                error_message.as_deref(),
                warning_message.as_deref(),
            )?;
        }
    }

    let affected_rows = changes
        .iter()
        .map(|change| change.0)
        .collect::<BTreeSet<_>>()
        .len() as i64;
    let step_id = if changes.is_empty() {
        None
    } else {
        Some(insert_step(
            database,
            request.batch_id,
            step_order,
            &request.step_type,
            &params_json,
            &changes,
        )?)
    };

    import_repository::update_batch_counts(database, request.batch_id, "cleaned")?;
    Ok(CleanStepResult {
        step_id,
        affected_rows,
        summary: import_repository::summary(database, request.batch_id)?,
    })
}

pub fn undo_last_clean_step(database: &Database, batch_id: i64) -> AppResult<CleanStepResult> {
    let step_id = latest_step_id(database, batch_id)?;
    let Some(step_id) = step_id else {
        return Ok(CleanStepResult {
            step_id: None,
            affected_rows: 0,
            summary: import_repository::summary(database, batch_id)?,
        });
    };

    let changes = load_step_changes(database, step_id)?;
    for (row_id, field, old_value) in &changes {
        let row = row_by_id(database, *row_id)?;
        let mut object = parse_optional_json(row.normalized_json)?
            .as_object()
            .cloned()
            .unwrap_or_default();
        object.insert(
            field.clone(),
            serde_json::from_str(old_value).unwrap_or(Value::Null),
        );
        import_repository::update_row_normalized(
            database,
            *row_id,
            &serde_json::to_string(&Value::Object(object))?,
            "staged",
            None,
            None,
        )?;
    }
    delete_step(database, step_id)?;
    let summary = validate_batch(database, batch_id)?;
    Ok(CleanStepResult {
        step_id: Some(step_id),
        affected_rows: changes
            .iter()
            .map(|change| change.0)
            .collect::<BTreeSet<_>>()
            .len() as i64,
        summary,
    })
}

pub fn confirm_import(database: &Database, batch_id: i64) -> AppResult<ConfirmImportResult> {
    let batch = import_repository::get_batch(database, batch_id)?;
    let rows = import_repository::list_all_rows(database, batch_id)?;
    let mut imported_count = 0;
    let mut skipped_count = 0;

    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        for row in &rows {
            let row_id = row.id.unwrap_or_default();
            if row.status == "error" {
                skipped_count += 1;
                continue;
            }
            let normalized = parse_optional_json(row.normalized_json.clone())?;
            let object = normalized.as_object().cloned().unwrap_or_default();
            let item_id = insert_knowledge_item(&transaction, &batch.target_type, &object)?;
            insert_detail(&transaction, item_id, &batch.target_type, &object)?;
            transaction.execute(
                "UPDATE data_import_rows SET status = 'imported' WHERE id = ?1",
                [row_id],
            )?;
            imported_count += 1;
        }
        transaction.execute(
            "UPDATE data_import_batches SET status = 'imported' WHERE id = ?1",
            [batch_id],
        )?;
        transaction.commit()?;
        Ok(())
    })?;

    import_repository::update_batch_counts(database, batch_id, "imported")?;
    search_index_service::rebuild_search_index(database)?;
    Ok(ConfirmImportResult {
        batch_id,
        imported_count,
        skipped_count,
        summary: import_repository::summary(database, batch_id)?,
    })
}

pub fn update_staging_row_field(
    database: &Database,
    batch_id: i64,
    row_id: i64,
    field_name: &str,
    new_value: &str,
) -> AppResult<ImportBatchSummary> {
    let batch = import_repository::get_batch(database, batch_id)?;
    let row = row_by_id(database, row_id)?;

    let mut normalized = parse_optional_json(row.normalized_json)?
        .as_object()
        .cloned()
        .unwrap_or_default();

    normalized.insert(field_name.to_string(), Value::String(new_value.to_string()));

    let issues = validation_service::validate_row(database, &batch.target_type, &normalized)?;
    let (status, error_message, warning_message) = validation_service::status_from_issues(&issues);

    import_repository::update_row_normalized(
        database,
        row_id,
        &serde_json::to_string(&Value::Object(normalized))?,
        status,
        error_message.as_deref(),
        warning_message.as_deref(),
    )?;

    validation_repository::replace_row_issues(database, batch_id, row_id, &issues)?;
    import_repository::update_batch_counts(database, batch_id, "edited")?;

    import_repository::summary(database, batch_id)
}

fn import_rows(
    database: &Database,
    import_type: &str,
    request: CreateImportRequest,
    parser: fn(&str) -> AppResult<Vec<Map<String, Value>>>,
) -> AppResult<ImportBatchSummary> {
    let rows = parser(&request.content)?;
    let mapping = request
        .mapping
        .or(field_mapping_service::mapping_from_template(
            database,
            request.template_id,
        )?);
    let batch = import_repository::create_batch(
        database,
        &request.file_name,
        import_type,
        &request.target_type,
        rows.len() as i64,
    )?;
    let batch_id = batch.id.unwrap_or_default();

    let mapped_rows: Vec<Map<String, Value>> = rows
        .iter()
        .map(|raw| {
            field_mapping_service::apply_mapping(raw, mapping.as_ref(), &request.target_type)
        })
        .collect();

    let normalized_rows = normalize_service::normalize_rows_batch(database, mapped_rows.clone())?;

    let mut import_rows_data = Vec::with_capacity(rows.len());
    let mut all_issues = Vec::new();

    for (index, ((raw, mapped), normalized)) in rows
        .iter()
        .zip(mapped_rows.iter())
        .zip(normalized_rows.iter())
        .enumerate()
    {
        let issues = validation_service::validate_row(database, &request.target_type, normalized)?;
        let (status, error_message, warning_message) =
            validation_service::status_from_issues(&issues);

        import_rows_data.push(import_repository::ImportRowData {
            row_index: index as i64 + 1,
            raw_json: serde_json::to_string(&Value::Object(raw.clone()))?,
            mapped_json: serde_json::to_string(&Value::Object(mapped.clone()))?,
            normalized_json: serde_json::to_string(&Value::Object(normalized.clone()))?,
            status: status.to_string(),
            error_message,
            warning_message,
        });

        all_issues.push(issues);
    }

    let row_ids = import_repository::insert_rows_batch(database, batch_id, import_rows_data)?;

    for (row_id, issues) in row_ids.iter().zip(all_issues.iter()) {
        validation_repository::replace_row_issues(database, batch_id, *row_id, issues)?;
    }

    // TODO: 线程 F 完成后，大文件在这里创建 background_jobs 并切到异步解析。
    import_repository::update_batch_counts(database, batch_id, "staged")?;
    import_repository::summary(database, batch_id)
}

fn parse_json_rows(content: &str) -> AppResult<Vec<Map<String, Value>>> {
    let value: Value = serde_json::from_str(content)?;
    let rows = match value {
        Value::Array(rows) => rows,
        Value::Object(mut object) => match object.remove("rows") {
            Some(Value::Array(rows)) => rows,
            _ => vec![Value::Object(object)],
        },
        _ => {
            return Err(AppError::InvalidInput(
                "JSON 必须是对象数组或包含 rows 数组的对象".to_string(),
            ))
        }
    };
    rows.into_iter()
        .map(|row| match row {
            Value::Object(object) => Ok(object),
            _ => Err(AppError::InvalidInput("JSON 行必须是对象".to_string())),
        })
        .collect()
}

fn parse_csv_rows(content: &str) -> AppResult<Vec<Map<String, Value>>> {
    let records = parse_csv_records(content)?;
    let Some(headers) = records.first() else {
        return Ok(Vec::new());
    };
    let mut rows = Vec::new();
    for record in records.iter().skip(1) {
        if record.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        let mut row = Map::new();
        for (index, header) in headers.iter().enumerate() {
            let value = record.get(index).cloned().unwrap_or_default();
            row.insert(header.trim().to_string(), Value::String(value));
        }
        rows.push(row);
    }
    Ok(rows)
}

fn parse_csv_records(content: &str) -> AppResult<Vec<Vec<String>>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut cell = String::new();
    let mut chars = content.chars().peekable();
    let mut quoted = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                record.push(cell.clone());
                cell.clear();
            }
            '\n' if !quoted => {
                record.push(cell.trim_end_matches('\r').to_string());
                cell.clear();
                records.push(record.clone());
                record.clear();
            }
            _ => cell.push(ch),
        }
    }
    if quoted {
        return Err(AppError::InvalidInput("CSV 引号未闭合".to_string()));
    }
    if !cell.is_empty() || !record.is_empty() {
        record.push(cell.trim_end_matches('\r').to_string());
        records.push(record);
    }
    Ok(records)
}

fn parse_excel_rows(content: &[u8]) -> AppResult<Vec<Map<String, Value>>> {
    use std::io::Cursor;

    let cursor = Cursor::new(content);
    let mut workbook: Xlsx<_> = open_workbook_from_rs(cursor)
        .map_err(|e| AppError::InvalidInput(format!("无法打开 Excel 文件: {}", e)))?;

    let sheet_name = workbook
        .sheet_names()
        .first()
        .ok_or_else(|| AppError::InvalidInput("Excel 文件没有工作表".to_string()))?
        .clone();

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::InvalidInput(format!("无法读取工作表: {}", e)))?;

    let mut rows_data = Vec::new();
    let mut headers = Vec::new();

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 {
            // 第一行作为表头
            for cell in row {
                headers.push(cell_to_string(cell));
            }
            continue;
        }

        // 跳过空行
        if row.iter().all(|cell| matches!(cell, Data::Empty)) {
            continue;
        }

        let mut row_map = Map::new();
        for (col_idx, cell) in row.iter().enumerate() {
            if let Some(header) = headers.get(col_idx) {
                if !header.is_empty() {
                    row_map.insert(header.clone(), Value::String(cell_to_string(cell)));
                }
            }
        }

        if !row_map.is_empty() {
            rows_data.push(row_map);
        }
    }

    Ok(rows_data)
}

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::Int(i) => i.to_string(),
        Data::Float(f) => f.to_string(),
        Data::String(s) => s.clone(),
        Data::Bool(b) => b.to_string(),
        Data::DateTime(dt) => dt.to_string(),
        Data::Error(e) => format!("Error: {:?}", e),
        Data::Empty => String::new(),
        _ => String::new(),
    }
}

fn import_rows_from_bytes(
    database: &Database,
    import_type: &str,
    request: CreateImportRequest,
) -> AppResult<ImportBatchSummary> {
    let rows = parse_excel_rows(request.content.as_bytes())?;
    let mapping = request
        .mapping
        .or(field_mapping_service::mapping_from_template(
            database,
            request.template_id,
        )?);
    let batch = import_repository::create_batch(
        database,
        &request.file_name,
        import_type,
        &request.target_type,
        rows.len() as i64,
    )?;
    let batch_id = batch.id.unwrap_or_default();

    let mapped_rows: Vec<Map<String, Value>> = rows
        .iter()
        .map(|raw| {
            field_mapping_service::apply_mapping(raw, mapping.as_ref(), &request.target_type)
        })
        .collect();

    let normalized_rows = normalize_service::normalize_rows_batch(database, mapped_rows.clone())?;

    let mut import_rows_data = Vec::with_capacity(rows.len());
    let mut all_issues = Vec::new();

    for (index, ((raw, mapped), normalized)) in rows
        .iter()
        .zip(mapped_rows.iter())
        .zip(normalized_rows.iter())
        .enumerate()
    {
        let issues = validation_service::validate_row(database, &request.target_type, normalized)?;
        let (status, error_message, warning_message) =
            validation_service::status_from_issues(&issues);

        import_rows_data.push(import_repository::ImportRowData {
            row_index: index as i64 + 1,
            raw_json: serde_json::to_string(&Value::Object(raw.clone()))?,
            mapped_json: serde_json::to_string(&Value::Object(mapped.clone()))?,
            normalized_json: serde_json::to_string(&Value::Object(normalized.clone()))?,
            status: status.to_string(),
            error_message,
            warning_message,
        });

        all_issues.push(issues);
    }

    let row_ids = import_repository::insert_rows_batch(database, batch_id, import_rows_data)?;

    for (row_id, issues) in row_ids.iter().zip(all_issues.iter()) {
        validation_repository::replace_row_issues(database, batch_id, *row_id, issues)?;
    }

    import_repository::update_batch_counts(database, batch_id, "staged")?;
    import_repository::summary(database, batch_id)
}

fn preview_from_rows(rows: Vec<Map<String, Value>>) -> ImportParsedPreview {
    let headers = rows
        .iter()
        .flat_map(|row| row.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let rows = rows.into_iter().take(100).map(Value::Object).collect();
    ImportParsedPreview { headers, rows }
}

fn parse_optional_json(json: Option<String>) -> AppResult<Value> {
    Ok(match json {
        Some(json) => serde_json::from_str(&json)?,
        None => Value::Null,
    })
}

fn clean_trim(mut object: Map<String, Value>) -> Map<String, Value> {
    for value in object.values_mut() {
        if let Value::String(text) = value {
            *text = text.trim().to_string();
        }
    }
    object
}

fn clean_half_width(mut object: Map<String, Value>) -> Map<String, Value> {
    for value in object.values_mut() {
        if let Value::String(text) = value {
            *text = normalize_service::to_half_width(text);
        }
    }
    object
}

fn clean_uppercase_code(mut object: Map<String, Value>) -> Map<String, Value> {
    if let Some(Value::String(code)) = object.get_mut("code") {
        *code = code.to_ascii_uppercase();
    }
    object
}

fn clean_split_tags(mut object: Map<String, Value>) -> Map<String, Value> {
    if let Some(Value::String(tags)) = object.get_mut("tags") {
        *tags = tags
            .split([',', '，', ';', '；', '、', '|'])
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join(",");
    }
    object
}

fn clean_set_category(
    mut object: Map<String, Value>,
    params: Option<&Value>,
) -> Map<String, Value> {
    if let Some(category) = params
        .and_then(|value| value.get("category"))
        .and_then(Value::as_str)
    {
        object.insert("category".to_string(), Value::String(category.to_string()));
    }
    object
}

fn diff_objects(
    before: &Map<String, Value>,
    after: &Map<String, Value>,
) -> Vec<(String, String, String)> {
    let keys = before
        .keys()
        .chain(after.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    keys.into_iter()
        .filter_map(|key| {
            let old_value = before.get(&key).cloned().unwrap_or(Value::Null);
            let new_value = after.get(&key).cloned().unwrap_or(Value::Null);
            (old_value != new_value).then(|| {
                (
                    key,
                    serde_json::to_string(&old_value).unwrap_or_else(|_| "null".to_string()),
                    serde_json::to_string(&new_value).unwrap_or_else(|_| "null".to_string()),
                )
            })
        })
        .collect()
}

fn next_step_order(database: &Database, batch_id: i64) -> AppResult<i64> {
    database.with_connection(|connection| {
        let order = connection.query_row(
            "SELECT COALESCE(MAX(step_order), 0) + 1 FROM data_transform_steps WHERE batch_id = ?1",
            [batch_id],
            |row| row.get(0),
        )?;
        Ok(order)
    })
}

fn insert_step(
    database: &Database,
    batch_id: i64,
    step_order: i64,
    step_type: &str,
    params_json: &str,
    changes: &[(i64, String, String, String)],
) -> AppResult<i64> {
    let now = Utc::now().to_rfc3339();
    database.with_connection(|connection| {
        connection.execute(
            "INSERT INTO data_transform_steps
             (batch_id, step_order, step_type, params_json, affected_rows, before_summary, after_summary, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                batch_id,
                step_order,
                step_type,
                params_json,
                changes.iter().map(|change| change.0).collect::<BTreeSet<_>>().len() as i64,
                "清洗前字段值已记录在 data_transform_row_changes",
                "清洗后字段值已记录在 data_transform_row_changes",
                now
            ],
        )?;
        let step_id = connection.last_insert_rowid();
        for (row_id, field, old_value, new_value) in changes {
            connection.execute(
                "INSERT INTO data_transform_row_changes(step_id, row_id, field_name, old_value, new_value)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![step_id, row_id, field, old_value, new_value],
            )?;
        }
        Ok(step_id)
    })
}

fn latest_step_id(database: &Database, batch_id: i64) -> AppResult<Option<i64>> {
    database.with_connection(|connection| {
        let step_id = connection
            .query_row(
                "SELECT id FROM data_transform_steps WHERE batch_id = ?1 ORDER BY step_order DESC, id DESC LIMIT 1",
                [batch_id],
                |row| row.get(0),
            )
            .optional()?;
        Ok(step_id)
    })
}

fn load_step_changes(database: &Database, step_id: i64) -> AppResult<Vec<(i64, String, String)>> {
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT row_id, field_name, old_value FROM data_transform_row_changes WHERE step_id = ?1 ORDER BY id DESC",
        )?;
        let rows = statement
            .query_map([step_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    })
}

fn delete_step(database: &Database, step_id: i64) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute("DELETE FROM data_transform_steps WHERE id = ?1", [step_id])?;
        Ok(())
    })
}

fn row_by_id(
    database: &Database,
    row_id: i64,
) -> AppResult<crate::models::data_pipeline::DataImportRow> {
    database.with_connection(|connection| {
        let row = connection.query_row(
            "SELECT id, batch_id, row_index, raw_json, mapped_json, normalized_json, status, error_message, warning_message
             FROM data_import_rows WHERE id = ?1",
            [row_id],
            |row| {
                Ok(crate::models::data_pipeline::DataImportRow {
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
            },
        )?;
        Ok(row)
    })
}

fn insert_knowledge_item(
    transaction: &rusqlite::Transaction<'_>,
    target_type: &str,
    object: &Map<String, Value>,
) -> AppResult<i64> {
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "INSERT INTO knowledge_items
         (type, code, name, alias, pinyin, category, summary, content, source_note, tags,
          data_status, completeness_status, content_version, is_favorite, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'imported', 'partial', 1, 0, ?11, ?12)",
        params![
            text(object, "type").unwrap_or_else(|| target_type.to_string()),
            text(object, "code"),
            text(object, "name").unwrap_or_default(),
            text(object, "alias"),
            text(object, "pinyin"),
            text(object, "category"),
            text(object, "summary"),
            text(object, "content"),
            text(object, "source_note"),
            text(object, "tags"),
            now,
            now
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

fn insert_detail(
    transaction: &rusqlite::Transaction<'_>,
    item_id: i64,
    target_type: &str,
    object: &Map<String, Value>,
) -> AppResult<()> {
    match target_type {
        "中药" | "herb" => {
            transaction.execute(
                "INSERT OR REPLACE INTO herb_details
                 (item_id, nature_flavor, meridians, effects, indications, dosage, contraindications, compatibility, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![item_id, text(object, "nature_flavor"), text(object, "meridians"), text(object, "effects"), text(object, "indications"), text(object, "dosage"), text(object, "contraindications"), text(object, "compatibility"), text(object, "notes")],
            )?;
        }
        "方剂" | "formula" => {
            transaction.execute(
                "INSERT OR REPLACE INTO formula_details
                 (item_id, source_text, composition, usage, effects, indications, explanation, modifications, contraindications, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![item_id, text(object, "source_text"), text(object, "composition"), text(object, "usage"), text(object, "effects"), text(object, "indications"), text(object, "explanation"), text(object, "modifications"), text(object, "contraindications"), text(object, "notes")],
            )?;
        }
        "经络" | "meridian" => {
            transaction.execute(
                "INSERT OR REPLACE INTO meridian_details
                 (item_id, meridian_code, category, yin_yang, hand_foot, organ_relation, paired_meridian, pathway_text, main_indications, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![item_id, text(object, "meridian_code").or_else(|| text(object, "code")), text(object, "category"), text(object, "yin_yang"), text(object, "hand_foot"), text(object, "organ_relation"), text(object, "paired_meridian"), text(object, "pathway_text"), text(object, "main_indications"), text(object, "notes")],
            )?;
        }
        "穴位" | "acupoint" => {
            let meridian_item_id = text(object, "meridians")
                .and_then(|name| lookup_meridian_id(transaction, &name).ok().flatten());
            transaction.execute(
                "INSERT OR REPLACE INTO acupoint_details
                 (item_id, acupoint_code, meridian_item_id, body_region, body_subregion, side_type, standard_location,
                  locating_method, bone_cun, anatomy, functions, indications, needling_summary, moxibustion_summary,
                  massage_summary, contraindications, precautions, risk_level)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
                params![item_id, text(object, "acupoint_code").or_else(|| text(object, "code")), meridian_item_id, text(object, "body_region"), text(object, "body_subregion"), text(object, "side_type"), text(object, "standard_location"), text(object, "locating_method"), text(object, "bone_cun"), text(object, "anatomy"), text(object, "functions"), text(object, "indications"), text(object, "needling_summary"), text(object, "moxibustion_summary"), text(object, "massage_summary"), text(object, "contraindications"), text(object, "precautions"), text(object, "risk_level")],
            )?;
        }
        "证型" | "syndrome" => {
            transaction.execute(
                "INSERT OR REPLACE INTO syndrome_details
                 (item_id, symptoms, tongue, pulse, pathogenesis, treatment_principle, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    item_id,
                    text(object, "symptoms"),
                    text(object, "tongue"),
                    text(object, "pulse"),
                    text(object, "pathogenesis"),
                    text(object, "treatment_principle"),
                    text(object, "notes")
                ],
            )?;
        }
        "病症" | "disease" => {
            transaction.execute(
                "INSERT OR REPLACE INTO disease_details
                 (item_id, symptoms, common_syndromes, care_advice, medical_warning, notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    item_id,
                    text(object, "symptoms"),
                    text(object, "common_syndromes"),
                    text(object, "care_advice"),
                    text(object, "medical_warning"),
                    text(object, "notes")
                ],
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn lookup_meridian_id(
    transaction: &rusqlite::Transaction<'_>,
    name: &str,
) -> AppResult<Option<i64>> {
    let value = transaction
        .query_row(
            "SELECT id FROM knowledge_items WHERE type IN ('经络','meridian') AND name = ?1 LIMIT 1",
            [name],
            |row| row.get(0),
        )
        .optional()?;
    Ok(value)
}

fn text(object: &Map<String, Value>, field: &str) -> Option<String> {
    match object.get(field) {
        Some(Value::String(text)) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    }
}
