use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::data_pipeline::{
    CleanStepRequest, CleanStepResult, ConfirmImportResult, CreateImportRequest,
    ImportBatchSummary, ImportParsedPreview, StagingIssue, StagingPage, StagingRowView,
};
use crate::repositories::{import_repository, validation_repository};
use crate::services::{
    field_mapping_service, import_engine_service, normalize_service, search_index_service,
    validation_service,
};
use calamine::{open_workbook_from_rs, Data, Reader, Xlsx};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::io::{Cursor, Read};
use zip::ZipArchive;

pub fn preview_json(content: &str) -> AppResult<ImportParsedPreview> {
    let rows = parse_json_rows(content)?;
    Ok(preview_from_rows(
        "manual-import.json",
        "json",
        rows,
        "mixed",
    ))
}

pub fn preview_csv(content: &str) -> AppResult<ImportParsedPreview> {
    let rows = parse_csv_rows(content)?;
    Ok(preview_from_rows("manual-import.csv", "csv", rows, "mixed"))
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
    Ok(preview_from_rows(
        "manual-import.xlsx",
        "excel",
        rows,
        "mixed",
    ))
}

pub fn import_excel(
    database: &Database,
    request: CreateImportRequest,
) -> AppResult<ImportBatchSummary> {
    import_rows_from_bytes(database, "excel", request)
}

pub fn preview_zip(file_name: &str, content: &[u8]) -> AppResult<ImportParsedPreview> {
    let (rows, import_type, warnings) = parse_zip_import_rows(content)?;
    let mut preview = preview_from_rows(file_name, &import_type, rows, "mixed");
    preview.warnings.extend(warnings);
    Ok(preview)
}

pub fn import_zip(
    database: &Database,
    request: CreateImportRequest,
    bytes: &[u8],
) -> AppResult<ImportBatchSummary> {
    let (rows, import_type, _warnings) = parse_zip_import_rows(bytes)?;
    import_preparsed_rows(database, &import_type, request, rows)
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
        let effective_type = effective_target_type(&batch.target_type, &object);
        let issues = validation_service::validate_row(database, &effective_type, &object)?;
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
            let effective_type = effective_target_type(&batch.target_type, &object);
            let issues = validation_service::validate_row(database, &effective_type, &object)?;
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
            let effective_type = effective_target_type(&batch.target_type, &object);
            let item_id = insert_knowledge_item(&transaction, &effective_type, &object)?;
            insert_detail(&transaction, item_id, &effective_type, &object)?;
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

    let effective_type = effective_target_type(&batch.target_type, &normalized);
    let issues = validation_service::validate_row(database, &effective_type, &normalized)?;
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
    import_preparsed_rows(database, import_type, request, rows)
}

fn import_preparsed_rows(
    database: &Database,
    import_type: &str,
    request: CreateImportRequest,
    rows: Vec<Map<String, Value>>,
) -> AppResult<ImportBatchSummary> {
    let mapping = request
        .mapping
        .or(field_mapping_service::mapping_from_template(
            database,
            request.template_id,
        )?);
    let engine = import_engine_service::prepare_import_rows(
        &request.file_name,
        import_type,
        &request.target_type,
        &rows,
        mapping.as_ref(),
    );
    let _ = (&engine.mapping_suggestions, &engine.warnings);
    let target_type = if engine.direct_import_ready {
        "mixed"
    } else {
        &request.target_type
    };
    let batch = import_repository::create_batch(
        database,
        &request.file_name,
        &engine.detection.detected_type,
        target_type,
        rows.len() as i64,
    )?;
    let batch_id = batch.id.unwrap_or_default();

    let mapped_rows = engine.mapped_rows;

    let normalized_rows = normalize_service::normalize_rows_batch(database, mapped_rows.clone())?;

    let mut import_rows_data = Vec::with_capacity(rows.len());
    let mut all_issues = Vec::new();

    for (index, ((raw, mapped), normalized)) in rows
        .iter()
        .zip(mapped_rows.iter())
        .zip(normalized_rows.iter())
        .enumerate()
    {
        let effective_type = effective_target_type(target_type, normalized);
        let issues = validation_service::validate_row(database, &effective_type, normalized)?;
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
    import_preparsed_rows(database, import_type, request, rows)
}

fn parse_zip_import_rows(
    content: &[u8],
) -> AppResult<(Vec<Map<String, Value>>, String, Vec<String>)> {
    let cursor = Cursor::new(content);
    let mut archive = ZipArchive::new(cursor)
        .map_err(|error| AppError::InvalidInput(format!("无法读取 ZIP 数据包: {}", error)))?;
    let import_manifest = read_zip_text(&mut archive, "import_manifest.json")?;
    let package_manifest = if import_manifest.is_none() {
        read_zip_text(&mut archive, "manifest.json")?
    } else {
        None
    };
    let mut warnings = Vec::new();
    let mut candidates = Vec::new();

    if let Some(manifest_text) = import_manifest {
        let manifest_value: Value = serde_json::from_str(&manifest_text)?;
        let files = manifest_value
            .get("files")
            .and_then(Value::as_array)
            .ok_or_else(|| AppError::InvalidInput("manifest 缺少 files 数组".to_string()))?;
        if let Some(package_name) = manifest_value.get("package_name").and_then(Value::as_str) {
            warnings.push(format!("数据包: {}", package_name));
        }
        warnings.push(format!("manifest 文件数: {}", files.len()));
        for file in files {
            let path = file
                .get("path")
                .and_then(Value::as_str)
                .ok_or_else(|| AppError::InvalidInput("manifest 文件项缺少 path".to_string()))?;
            let import_type = file
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("generic_json");
            let target = file
                .get("target")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let primary = file
                .get("primary")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let can_direct_import = import_type == "knowledge_items_v1"
                || import_type == "classic_passages_v1"
                || target == "knowledge_items"
                || primary;
            warnings.push(format!(
                "文件: {} / 类型: {} / 目标: {} / 可直接导入: {}",
                path,
                import_type,
                if target.is_empty() {
                    "未声明"
                } else {
                    target
                },
                can_direct_import
            ));
            if can_direct_import {
                let text = read_zip_text(&mut archive, path)?.ok_or_else(|| {
                    AppError::InvalidInput(format!("manifest 指向的文件不存在: {}", path))
                })?;
                let rows = if path.to_ascii_lowercase().ends_with(".csv") {
                    parse_csv_rows(&text)?
                } else {
                    parse_json_rows(&text)?
                };
                candidates.extend(rows);
            } else {
                warnings.push(format!(
                    "已识别 manifest 文件 {}，v0.1 暂不直接导入目标 {}",
                    path, target
                ));
            }
        }
        if candidates.is_empty() {
            return Err(AppError::InvalidInput(
                "manifest 中没有可导入到 knowledge_items 的文件".to_string(),
            ));
        }
        return Ok((candidates, "zip_manifest".to_string(), warnings));
    }

    if package_manifest.is_some() {
        warnings.push("检测到 package manifest，但不是 Import Engine V2 的 import_manifest.json，已按内置经典包规则自动查找可导入文件。".to_string());
    }

    for path in [
        "json/knowledge_items_import_curated.json",
        "json/knowledge_items_import_full_clean.json",
        "json/classic_passages_curated.json",
        "json/classic_passages_full_clean.json",
        "csv/knowledge_items_import_curated.csv",
        "csv/knowledge_items_import_full_clean.csv",
        "csv/classic_passages_curated.csv",
        "csv/classic_passages_full_clean.csv",
    ] {
        if let Some(text) = read_zip_text(&mut archive, path)? {
            let rows = if path.ends_with(".csv") {
                parse_csv_rows(&text)?
            } else {
                parse_json_rows(&text)?
            };
            return Ok((rows, "zip_auto".to_string(), warnings));
        }
    }

    Err(AppError::InvalidInput(
        "ZIP 中未找到 import_manifest.json 或支持的经典数据文件".to_string(),
    ))
}

fn read_zip_text<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> AppResult<Option<String>> {
    let normalized = path.replace('\\', "/");
    for index in 0..archive.len() {
        let mut file = archive
            .by_index(index)
            .map_err(|error| AppError::InvalidInput(format!("无法读取 ZIP 文件项: {}", error)))?;
        if file.name().replace('\\', "/").ends_with(&normalized) {
            let mut text = String::new();
            file.read_to_string(&mut text)?;
            return Ok(Some(text));
        }
    }
    Ok(None)
}

fn preview_from_rows(
    file_name: &str,
    import_type: &str,
    rows: Vec<Map<String, Value>>,
    target_type: &str,
) -> ImportParsedPreview {
    let headers = rows
        .iter()
        .flat_map(|row| row.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let detection = import_engine_service::detect_import_type(file_name, import_type, &rows);
    let mapping_suggestions = import_engine_service::score_mapping(&rows, target_type);
    let direct_import_ready =
        import_engine_service::is_direct_import_type(&detection.detected_type);
    let rows = rows.into_iter().take(100).map(Value::Object).collect();
    ImportParsedPreview {
        headers,
        rows,
        detection,
        mapping_suggestions,
        direct_import_ready,
        warnings: Vec::new(),
    }
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
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'partial', 1, 0, ?12, ?13)",
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
            text(object, "data_status").unwrap_or_else(|| "validated".to_string()),
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
    object.get(field).and_then(value_to_text)
}

fn value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        Value::Array(values) => {
            let joined = values
                .iter()
                .filter_map(value_to_text)
                .collect::<Vec<_>>()
                .join(",");
            (!joined.is_empty()).then_some(joined)
        }
        _ => None,
    }
}

fn effective_target_type(target_type: &str, object: &Map<String, Value>) -> String {
    match target_type {
        "mixed" | "auto" | "自动识别" | "混合类型" => {
            text(object, "type").unwrap_or_else(|| "mixed".to_string())
        }
        _ => target_type.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{confirm_import, import_csv, import_json, import_zip};
    use crate::db::connection::Database;
    use crate::models::data_pipeline::CreateImportRequest;
    use crate::models::search::SearchRequest;
    use crate::services::search_index_service;
    use std::fs;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_database(test_name: &str) -> (std::path::PathBuf, Database) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("zhongyi-import-{test_name}-{unique}"));
        let database = Database::initialize(&data_dir).expect("initialize database");
        (data_dir, database)
    }

    #[test]
    fn mixed_classics_json_import_uses_row_type_and_detail_fields() {
        let (data_dir, database) = temp_database("classics");
        let content = r#"
        [
          {
            "type": "syndrome",
            "code": "HDNJ_SW-00001",
            "name": "上古天真论篇第一",
            "category": "原典/黄帝内经·素问",
            "summary": "养生原文摘要",
            "content": "昔在黄帝，生而神灵。",
            "source_note": "黄帝内经·素问",
            "tags": ["原典", "黄帝内经·素问", "机器精校"],
            "detail": {
              "symptoms": "原典篇章内容",
              "notes": "机器结构精校"
            }
          },
          {
            "type": "herb",
            "code": "SNBCJ-00084",
            "name": "甘草",
            "category": "原典/神农本草经",
            "summary": "甘草条文摘要",
            "content": "甘草，味甘平。",
            "source_note": "神农本草经",
            "tags": ["原典", "神农本草经"],
            "detail": {
              "nature_flavor": "味甘平",
              "effects": "主五脏六腑寒热邪气",
              "notes": "机器结构精校"
            }
          }
        ]
        "#;

        let summary = import_json(
            &database,
            CreateImportRequest {
                file_name: "knowledge_items_import_curated.json".to_string(),
                target_type: "mixed".to_string(),
                content: content.to_string(),
                mapping: None,
                template_id: None,
            },
        )
        .expect("import classics json");

        assert_eq!(summary.total_rows, 2);
        assert_eq!(summary.importable_rows, 2);
        assert_eq!(summary.error_rows, 0);

        database
            .with_connection(|connection| {
                let staged_tags: String = connection.query_row(
                    "SELECT json_extract(normalized_json, '$.tags')
                     FROM data_import_rows WHERE row_index = 1",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(staged_tags, "原典,黄帝内经·素问,机器精校");

                let staged_symptoms: String = connection.query_row(
                    "SELECT json_extract(normalized_json, '$.symptoms')
                     FROM data_import_rows WHERE row_index = 1",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(staged_symptoms, "原典篇章内容");
                Ok(())
            })
            .expect("inspect staging rows");

        let result = confirm_import(&database, summary.batch.id.unwrap()).expect("confirm import");
        assert_eq!(result.imported_count, 2);
        assert_eq!(result.skipped_count, 0);

        database
            .with_connection(|connection| {
                let syndrome_count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM knowledge_items WHERE type = 'syndrome'",
                    [],
                    |row| row.get(0),
                )?;
                let herb_count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM knowledge_items WHERE type = 'herb'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(syndrome_count, 1);
                assert_eq!(herb_count, 1);

                let symptoms: String = connection.query_row(
                    "SELECT sd.symptoms
                     FROM syndrome_details sd
                     JOIN knowledge_items ki ON ki.id = sd.item_id
                     WHERE ki.code = 'HDNJ_SW-00001'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(symptoms, "原典篇章内容");

                let effects: String = connection.query_row(
                    "SELECT hd.effects
                     FROM herb_details hd
                     JOIN knowledge_items ki ON ki.id = hd.item_id
                     WHERE ki.code = 'SNBCJ-00084'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(effects, "主五脏六腑寒热邪气");
                Ok(())
            })
            .expect("inspect imported rows");

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn classic_passages_import_preserves_content_source_and_searches() {
        let (data_dir, database) = temp_database("classic-passages");
        let content = r#"
        [
          {
            "work_title": "黄帝内经·素问",
            "page_title": "卷第一",
            "section_title": "上古天真论",
            "original_text": "上古天真论曰：上古之人，其知道者，法于阴阳。",
            "source_note": "黄帝内经·素问 / 上古天真论",
            "source_url": "local://classics/huangdi-neijing"
          },
          {
            "work_title": "伤寒论",
            "page_title": "辨太阳病脉证并治",
            "section_title": "太阳病",
            "original_text": "太阳之为病，脉浮，头项强痛而恶寒。",
            "source_note": "伤寒论 / 太阳病"
          }
        ]
        "#;

        let summary = import_json(
            &database,
            CreateImportRequest {
                file_name: "classic_passages_curated.json".to_string(),
                target_type: "mixed".to_string(),
                content: content.to_string(),
                mapping: None,
                template_id: None,
            },
        )
        .expect("import classic passages");
        assert_eq!(summary.importable_rows, 2);
        confirm_import(&database, summary.batch.id.unwrap()).expect("confirm classic passages");

        database
            .with_connection(|connection| {
                let content: String = connection.query_row(
                    "SELECT content FROM knowledge_items WHERE name LIKE '%上古天真论%'",
                    [],
                    |row| row.get(0),
                )?;
                assert!(content.contains("法于阴阳"));

                let source_note: String = connection.query_row(
                    "SELECT source_note FROM knowledge_items WHERE name LIKE '%上古天真论%'",
                    [],
                    |row| row.get(0),
                )?;
                assert!(source_note.contains("黄帝内经·素问"));
                assert!(source_note.contains("local://classics/huangdi-neijing"));
                Ok(())
            })
            .expect("inspect classic import");

        for query in ["上古天真论", "太阳病"] {
            let response = search_index_service::search(
                &database,
                SearchRequest {
                    query: query.to_string(),
                    item_type: None,
                    page: Some(1),
                    page_size: Some(10),
                },
            )
            .expect("search after import");
            assert!(!response.results.is_empty(), "expected hit for {query}");
        }

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn generic_csv_uses_scored_mapping() {
        let (data_dir, database) = temp_database("generic-csv");
        let content = "名称,编号,原文,标签\r\n桂枝汤,GZT-001,桂枝汤经典原文内容用于测试导入正文识别,方剂、原典\r\n";
        let summary = import_csv(
            &database,
            CreateImportRequest {
                file_name: "generic.csv".to_string(),
                target_type: "formula".to_string(),
                content: content.to_string(),
                mapping: None,
                template_id: None,
            },
        )
        .expect("import generic csv");
        assert_eq!(summary.error_rows, 0);

        database
            .with_connection(|connection| {
                let mapped_name: String = connection.query_row(
                    "SELECT json_extract(normalized_json, '$.name') FROM data_import_rows LIMIT 1",
                    [],
                    |row| row.get(0),
                )?;
                let mapped_content: String = connection.query_row(
                    "SELECT json_extract(normalized_json, '$.content') FROM data_import_rows LIMIT 1",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(mapped_name, "桂枝汤");
                assert!(mapped_content.contains("经典原文"));
                Ok(())
            })
            .expect("inspect csv mapping");

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn zip_import_manifest_loads_primary_knowledge_file() {
        let (data_dir, database) = temp_database("zip-manifest");
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("import_manifest.json", options).unwrap();
            writer
                .write_all(
                    br#"{
                "package_name": "zhongyi_classics_curated_v0_3",
                "schema_version": "1.0",
                "import_profile": "classics_curated_v1",
                "files": [
                  {
                    "path": "json/knowledge_items_import_curated.json",
                    "type": "knowledge_items_v1",
                    "target": "knowledge_items",
                    "primary": true
                  }
                ],
                "import_order": ["knowledge_items"]
            }"#,
                )
                .unwrap();
            writer
                .start_file("json/knowledge_items_import_curated.json", options)
                .unwrap();
            writer
                .write_all(
                    r#"[
              {
                "type": "formula",
                "code": "GZT-001",
                "name": "桂枝汤",
                "content": "桂枝汤经典原文，用于 ZIP manifest 导入测试。",
                "source_note": "伤寒论",
                "tags": ["方剂", "伤寒论"],
                "data_status": "validated",
                "detail": {"composition": "桂枝,芍药,甘草,生姜,大枣"}
              }
            ]"#
                    .as_bytes(),
                )
                .unwrap();
            writer.finish().unwrap();
        }
        let bytes = buffer.into_inner();

        let summary = import_zip(
            &database,
            CreateImportRequest {
                file_name: "zhongyi_classics_curated_v0_3_manifest.zip".to_string(),
                target_type: "mixed".to_string(),
                content: String::new(),
                mapping: None,
                template_id: None,
            },
            &bytes,
        )
        .expect("import zip manifest");
        assert_eq!(summary.total_rows, 1);
        confirm_import(&database, summary.batch.id.unwrap()).expect("confirm zip import");

        let response = search_index_service::search(
            &database,
            SearchRequest {
                query: "桂枝汤".to_string(),
                item_type: None,
                page: Some(1),
                page_size: Some(10),
            },
        )
        .expect("search zip import");
        assert!(!response.results.is_empty());

        let _ = fs::remove_dir_all(data_dir);
    }
}
