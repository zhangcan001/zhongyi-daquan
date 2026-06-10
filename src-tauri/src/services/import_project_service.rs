use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::data_pipeline::{
    CleanStepRequest, CleanStepResult, ConfirmImportResult, CreateImportRequest,
    ImportBatchSummary, ImportDiffReport, ImportPackageDescriptor, ImportPackageFile,
    ImportParsedPreview, ImportQualityReport, RollbackImportResult, StagingIssue, StagingPage,
    StagingRowView,
};
use crate::repositories::{import_repository, search_repository, validation_repository};
use crate::services::{
    field_mapping_service, import_engine_service, normalize_service, search_index_service,
    validation_service,
};
use calamine::{open_workbook_from_rs, Data, Reader, Xlsx};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
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
    let parsed = parse_zip_import_package(file_name, content)?;
    let mut preview = preview_from_rows(
        &primary_file_name(&parsed.descriptor, file_name),
        &parsed.import_type,
        parsed.rows,
        "mixed",
    );
    apply_package_descriptor_to_preview(&mut preview, &parsed.descriptor);
    Ok(preview)
}

pub fn import_zip(
    database: &Database,
    request: CreateImportRequest,
    bytes: &[u8],
) -> AppResult<ImportBatchSummary> {
    let parsed = parse_zip_import_package(&request.file_name, bytes)?;
    let rows = parsed.rows;
    let import_type = parsed.import_type;
    import_preparsed_rows(database, &import_type, request, rows)
}

pub fn preview_package_folder(folder_path: &str) -> AppResult<ImportPackageDescriptor> {
    let parsed = parse_folder_import_package(folder_path)?;
    Ok(parsed.descriptor)
}

pub fn import_package_folder(
    database: &Database,
    folder_path: &str,
) -> AppResult<ImportBatchSummary> {
    let parsed = parse_folder_import_package(folder_path)?;
    let folder_name = Path::new(folder_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("package-folder")
        .to_string();
    let request = CreateImportRequest {
        file_name: folder_name,
        target_type: "mixed".to_string(),
        content: String::new(),
        mapping: None,
        template_id: None,
    };
    import_preparsed_rows(database, &parsed.import_type, request, parsed.rows)
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
    if is_maintenance_import_type(&batch.import_type) {
        return confirm_maintenance_import(database, batch_id, &batch.import_type, &rows);
    }
    let mut imported_count = 0;
    let mut skipped_count = 0;
    let mut imported_item_ids = Vec::new();
    let mut imported_rows = Vec::new();

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
            upsert_import_fingerprint(&transaction, item_id)?;
            transaction.execute(
                "UPDATE data_import_rows SET status = 'imported' WHERE id = ?1",
                [row_id],
            )?;
            imported_count += 1;
            imported_item_ids.push(item_id);
            imported_rows.push(row.clone());
        }
        transaction.execute(
            "UPDATE data_import_batches
             SET status = 'imported', confirmed_item_ids_json = ?2
             WHERE id = ?1",
            params![batch_id, serde_json::to_string(&imported_item_ids)?],
        )?;
        transaction.commit()?;
        Ok(())
    })?;

    import_repository::update_batch_counts(database, batch_id, "imported")?;
    search_index_service::rebuild_search_index(database)?;
    let search_terms_imported = if should_import_package_terms(&batch.import_type) {
        import_search_terms_for_items(database, &imported_rows, &imported_item_ids)?
    } else {
        0
    };
    set_search_terms_imported_count(database, batch_id, search_terms_imported)?;
    let report = import_quality_report(database, batch_id)?;
    import_repository::set_quality_report(database, batch_id, &serde_json::to_string(&report)?)?;
    Ok(ConfirmImportResult {
        batch_id,
        imported_count,
        skipped_count,
        summary: import_repository::summary(database, batch_id)?,
    })
}

pub fn import_quality_report(database: &Database, batch_id: i64) -> AppResult<ImportQualityReport> {
    let summary = import_repository::summary(database, batch_id)?;
    let rows = import_repository::list_all_rows(database, batch_id)?;
    let mut field_counts: BTreeMap<String, i64> = BTreeMap::new();
    let mut empty_counts: BTreeMap<String, i64> = BTreeMap::new();
    let mut fingerprints = HashSet::new();
    let mut duplicate_fingerprint_count = 0_i64;

    for row in &rows {
        let value = parse_optional_json(row.normalized_json.clone())?;
        let object = value.as_object().cloned().unwrap_or_default();
        let fingerprint = row_fingerprint(&object);
        if !fingerprint.is_empty() && !fingerprints.insert(fingerprint) {
            duplicate_fingerprint_count += 1;
        }
        for field in [
            "type",
            "code",
            "name",
            "category",
            "summary",
            "content",
            "source_note",
            "tags",
        ] {
            if object.contains_key(field) {
                *field_counts.entry(field.to_string()).or_default() += 1;
                if text(&object, field).is_none() {
                    *empty_counts.entry(field.to_string()).or_default() += 1;
                }
            }
        }
    }

    let total = rows.len().max(1) as f64;
    let field_coverage = field_counts
        .into_iter()
        .map(|(field, count)| (field, count as f64 / total))
        .collect::<BTreeMap<_, _>>();
    let search_terms_imported_count = database.with_connection(|connection| {
        let count: i64 = connection.query_row(
            "SELECT COALESCE(search_terms_imported_count, 0) FROM data_import_batches WHERE id = ?1",
            params![batch_id],
            |row| row.get(0),
        )?;
        Ok(count)
    })?;
    let confirmed_item_ids = import_repository::confirmed_item_ids(database, batch_id)?;
    let duplicate_warning_rows = database.with_connection(|connection| {
        let count: i64 = connection.query_row(
            "SELECT COUNT(DISTINCT row_id)
             FROM data_validation_issues
             WHERE batch_id = ?1 AND issue_code = 'possible_existing_duplicate'",
            params![batch_id],
            |row| row.get(0),
        )?;
        Ok(count)
    })?;
    let mut affected_types = BTreeMap::new();
    for row in &rows {
        let value = parse_optional_json(row.normalized_json.clone())?;
        let object = value.as_object().cloned().unwrap_or_default();
        let kind = text(&object, "type").unwrap_or_else(|| "unknown".to_string());
        *affected_types.entry(kind).or_default() += 1;
    }
    let checked = search_keywords(database, &["桂枝汤", "太阳病", "上古天真论", "神农本草经"])?;
    let mut suggestions = Vec::new();
    if summary.error_rows > 0 {
        suggestions.push("存在错误行，请先在暂存区修正后再确认入库。".to_string());
    }
    if duplicate_fingerprint_count > 0 {
        suggestions.push(format!(
            "发现 {duplicate_fingerprint_count} 条批内疑似重复，请运行去重检查。"
        ));
    }
    if field_coverage
        .get("source_note")
        .copied()
        .unwrap_or_default()
        < 0.8
    {
        suggestions.push("source_note 覆盖率偏低，建议补充出处信息。".to_string());
    }
    if duplicate_warning_rows > 0 {
        suggestions.push(format!(
            "有 {duplicate_warning_rows} 行与正式库疑似重复，建议导入前先确认是否合并。"
        ));
    }

    Ok(ImportQualityReport {
        batch_id,
        detected_type: summary.batch.import_type,
        total_rows: summary.total_rows,
        importable_rows: summary.importable_rows,
        warning_rows: summary.warning_rows,
        error_rows: summary.error_rows,
        field_coverage,
        empty_field_counts: empty_counts,
        duplicate_fingerprint_count,
        search_terms_imported_count,
        import_diff: ImportDiffReport {
            inserted_items: confirmed_item_ids.len() as i64,
            skipped_rows: summary.total_rows - confirmed_item_ids.len() as i64,
            duplicate_warning_rows,
            imported_search_terms: search_terms_imported_count,
            affected_types,
        },
        searchable_keywords_checked: checked,
        suggestions,
    })
}

pub fn rollback_import_batch(
    database: &Database,
    batch_id: i64,
) -> AppResult<RollbackImportResult> {
    let item_ids = import_repository::confirmed_item_ids(database, batch_id)?;
    if item_ids.is_empty() {
        return Err(AppError::InvalidInput(
            "该批次没有可回滚的 confirmed_item_ids".to_string(),
        ));
    }
    let deleted_search_terms = database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        let mut deleted_terms = 0_i64;
        for item_id in &item_ids {
            deleted_terms += transaction.execute(
                "DELETE FROM search_terms WHERE item_id = ?1",
                params![item_id],
            )? as i64;
            transaction.execute(
                "DELETE FROM knowledge_fingerprints WHERE item_id = ?1",
                params![item_id],
            )?;
            transaction.execute(
                "DELETE FROM knowledge_items WHERE id = ?1",
                params![item_id],
            )?;
        }
        transaction.commit()?;
        Ok(deleted_terms)
    })?;
    import_repository::mark_rolled_back(database, batch_id)?;
    search_index_service::rebuild_search_index(database)?;
    Ok(RollbackImportResult {
        batch_id,
        deleted_items: item_ids.len() as i64,
        deleted_search_terms,
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
    let target_type = if is_maintenance_import_type(&engine.detection.detected_type) {
        engine.detection.detected_type.as_str()
    } else if engine.direct_import_ready {
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
        let issues = if is_maintenance_import_type(&engine.detection.detected_type) {
            validate_maintenance_row(&engine.detection.detected_type, normalized)
        } else {
            let effective_type = effective_target_type(target_type, normalized);
            let mut issues =
                validation_service::validate_row(database, &effective_type, normalized)?;
            issues.extend(existing_duplicate_warnings(database, normalized)?);
            issues
        };
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

fn confirm_maintenance_import(
    database: &Database,
    batch_id: i64,
    import_type: &str,
    rows: &[crate::models::data_pipeline::DataImportRow],
) -> AppResult<ConfirmImportResult> {
    let mut imported_count = 0_i64;
    let mut skipped_count = 0_i64;
    let mut affected_item_ids = Vec::new();

    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        for row in rows {
            let row_id = row.id.unwrap_or_default();
            if row.status == "error" {
                skipped_count += 1;
                continue;
            }
            let normalized = parse_optional_json(row.normalized_json.clone())?;
            let object = normalized.as_object().cloned().unwrap_or_default();
            let result = match import_type {
                "standard_terms_v1" => {
                    insert_standard_term_row(&transaction, &object).map(|_| None)
                }
                "search_terms_v1" => insert_search_term_row(&transaction, &object),
                "relation_suggestions_v1" => insert_relation_suggestion_row(&transaction, &object),
                _ => Err(AppError::InvalidInput(format!(
                    "不支持的维护导入类型: {import_type}"
                ))),
            };

            match result {
                Ok(item_id) => {
                    if let Some(item_id) = item_id {
                        affected_item_ids.push(item_id);
                    }
                    transaction.execute(
                        "UPDATE data_import_rows
                         SET status = 'imported', error_message = NULL
                         WHERE id = ?1",
                        params![row_id],
                    )?;
                    imported_count += 1;
                }
                Err(AppError::InvalidInput(message)) | Err(AppError::Data(message)) => {
                    transaction.execute(
                        "UPDATE data_import_rows
                         SET status = 'error', error_message = ?2
                         WHERE id = ?1",
                        params![row_id, message],
                    )?;
                    skipped_count += 1;
                }
                Err(error) => return Err(error),
            }
        }
        affected_item_ids.sort_unstable();
        affected_item_ids.dedup();
        transaction.execute(
            "UPDATE data_import_batches
             SET status = 'imported', confirmed_item_ids_json = ?2
             WHERE id = ?1",
            params![batch_id, serde_json::to_string(&affected_item_ids)?],
        )?;
        transaction.commit()?;
        Ok(())
    })?;

    import_repository::update_batch_counts(database, batch_id, "imported")?;
    if import_type == "search_terms_v1" {
        set_search_terms_imported_count(database, batch_id, imported_count)?;
    }
    let report = import_quality_report(database, batch_id)?;
    import_repository::set_quality_report(database, batch_id, &serde_json::to_string(&report)?)?;
    Ok(ConfirmImportResult {
        batch_id,
        imported_count,
        skipped_count,
        summary: import_repository::summary(database, batch_id)?,
    })
}

fn insert_standard_term_row(
    transaction: &rusqlite::Transaction<'_>,
    object: &Map<String, Value>,
) -> AppResult<()> {
    let term_type = required_text(object, "term_type")?;
    let standard_name = required_text(object, "standard_name")?;
    transaction.execute(
        "UPDATE standard_terms
         SET aliases = COALESCE(?3, aliases),
             code = COALESCE(?4, code),
             notes = COALESCE(?5, notes)
         WHERE term_type = ?1 AND standard_name = ?2",
        params![
            term_type,
            standard_name,
            text(object, "aliases"),
            text(object, "code"),
            text(object, "notes")
        ],
    )?;
    if transaction.changes() == 0 {
        transaction.execute(
            "INSERT INTO standard_terms(term_type, standard_name, aliases, code, notes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                term_type,
                standard_name,
                text(object, "aliases"),
                text(object, "code"),
                text(object, "notes")
            ],
        )?;
    }
    Ok(())
}

fn insert_search_term_row(
    transaction: &rusqlite::Transaction<'_>,
    object: &Map<String, Value>,
) -> AppResult<Option<i64>> {
    let item_id = resolve_knowledge_item_id(
        transaction,
        text(object, "item_id"),
        text(object, "item_code"),
        text(object, "item_name"),
        text(object, "item_type"),
        "搜索词",
    )?;
    let term = required_text(object, "term")?;
    let normalized = search_repository::normalize_for_search(&term);
    if normalized.is_empty() {
        return Err(AppError::InvalidInput("搜索词 term 不能为空".to_string()));
    }
    let term_type = text(object, "term_type").unwrap_or_else(|| "imported".to_string());
    let weight = number_i64(object, "weight").unwrap_or(80).clamp(1, 500);
    transaction.execute(
        "INSERT INTO search_terms(item_id, term, term_type, weight)
         SELECT ?1, ?2, ?3, ?4
         WHERE NOT EXISTS (
           SELECT 1 FROM search_terms
           WHERE item_id = ?1 AND term = ?2 AND term_type = ?3
         )",
        params![item_id, normalized, term_type, weight],
    )?;
    Ok(Some(item_id))
}

fn insert_relation_suggestion_row(
    transaction: &rusqlite::Transaction<'_>,
    object: &Map<String, Value>,
) -> AppResult<Option<i64>> {
    let source_item_id = resolve_knowledge_item_id(
        transaction,
        text(object, "source_item_id"),
        text(object, "source_code"),
        text(object, "source_name"),
        text(object, "source_type"),
        "关系来源",
    )?;
    let target_item_id = resolve_knowledge_item_id(
        transaction,
        text(object, "target_item_id"),
        text(object, "target_code"),
        text(object, "target_name"),
        text(object, "target_type"),
        "关系目标",
    )?;
    if source_item_id == target_item_id {
        return Err(AppError::InvalidInput(
            "关系来源和目标不能是同一条目".to_string(),
        ));
    }
    let relation_type = text(object, "relation_type").unwrap_or_else(|| "related_to".to_string());
    let confidence = number_f64(object, "confidence")
        .unwrap_or(0.8)
        .clamp(0.0, 1.0);
    let reason = text(object, "reason").unwrap_or_else(|| "维护导入".to_string());
    transaction.execute(
        "INSERT INTO relation_suggestions
         (source_item_id, target_item_id, relation_type, confidence, reason, status, created_at)
         SELECT ?1, ?2, ?3, ?4, ?5, 'pending', datetime('now')
         WHERE NOT EXISTS (
           SELECT 1 FROM relation_suggestions
           WHERE source_item_id = ?1 AND target_item_id = ?2
             AND relation_type = ?3 AND status = 'pending'
         )
         AND NOT EXISTS (
           SELECT 1 FROM knowledge_relations
           WHERE source_item_id = ?1 AND target_item_id = ?2 AND relation_type = ?3
         )",
        params![
            source_item_id,
            target_item_id,
            relation_type,
            confidence,
            reason
        ],
    )?;
    Ok(Some(source_item_id))
}

fn resolve_knowledge_item_id(
    transaction: &rusqlite::Transaction<'_>,
    raw_id: Option<String>,
    code: Option<String>,
    name: Option<String>,
    item_type: Option<String>,
    label: &str,
) -> AppResult<i64> {
    if let Some(raw_id) = raw_id {
        let item_id = raw_id.parse::<i64>().map_err(|_| {
            AppError::InvalidInput(format!("{label} item_id 不是有效数字: {raw_id}"))
        })?;
        let exists: Option<i64> = transaction
            .query_row(
                "SELECT id FROM knowledge_items WHERE id = ?1",
                params![item_id],
                |row| row.get(0),
            )
            .optional()?;
        return exists
            .ok_or_else(|| AppError::InvalidInput(format!("{label} item_id 不存在: {item_id}")));
    }

    let mut clauses = Vec::new();
    let mut args = Vec::new();
    if let Some(code) = code.filter(|value| !value.trim().is_empty()) {
        clauses.push("code = ?".to_string());
        args.push(code);
    }
    if let Some(name) = name.filter(|value| !value.trim().is_empty()) {
        clauses.push("name = ?".to_string());
        args.push(name);
    }
    if clauses.is_empty() {
        return Err(AppError::InvalidInput(format!(
            "{label} 缺少 item_id、item_code 或 item_name"
        )));
    }
    let type_clause = if item_type.is_some() {
        " AND type = ?"
    } else {
        ""
    };
    let sql = format!(
        "SELECT id FROM knowledge_items WHERE ({}){} ORDER BY id DESC LIMIT 1",
        clauses.join(" OR "),
        type_clause
    );
    let mut statement = transaction.prepare(&sql)?;
    let item_id = match (args.len(), item_type) {
        (1, Some(kind)) => statement
            .query_row(params![args[0], kind], |row| row.get(0))
            .optional()?,
        (2, Some(kind)) => statement
            .query_row(params![args[0], args[1], kind], |row| row.get(0))
            .optional()?,
        (1, None) => statement
            .query_row(params![args[0]], |row| row.get(0))
            .optional()?,
        (2, None) => statement
            .query_row(params![args[0], args[1]], |row| row.get(0))
            .optional()?,
        _ => None,
    };
    item_id.ok_or_else(|| AppError::InvalidInput(format!("{label} 未匹配到知识条目")))
}

fn parse_json_rows(content: &str) -> AppResult<Vec<Map<String, Value>>> {
    let value: Value = serde_json::from_str(strip_json_bom(content))?;
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

pub(crate) struct ParsedImportPackage {
    pub descriptor: ImportPackageDescriptor,
    pub rows: Vec<Map<String, Value>>,
    pub import_type: String,
}

trait ImportPackageReader {
    fn package_root(&self) -> String;
    fn read_text(&mut self, path: &str) -> AppResult<Option<String>>;
    fn exists(&self, path: &str) -> bool;
    fn contains_raw_source_files(&self) -> bool;
}

struct ZipImportPackageReader<R: Read + std::io::Seek> {
    archive: ZipArchive<R>,
    names: Vec<String>,
    package_root: String,
}

impl<R: Read + std::io::Seek> ZipImportPackageReader<R> {
    fn new(reader: R, package_root: String) -> AppResult<Self> {
        let mut archive = ZipArchive::new(reader)
            .map_err(|error| AppError::InvalidInput(format!("无法读取 ZIP 数据包: {}", error)))?;
        let mut names = Vec::new();
        for index in 0..archive.len() {
            let file = archive.by_index(index).map_err(|error| {
                AppError::InvalidInput(format!("无法读取 ZIP 文件项: {}", error))
            })?;
            names.push(normalize_package_path(file.name()));
        }
        Ok(Self {
            archive,
            names,
            package_root,
        })
    }
}

impl<R: Read + std::io::Seek> ImportPackageReader for ZipImportPackageReader<R> {
    fn package_root(&self) -> String {
        self.package_root.clone()
    }

    fn read_text(&mut self, path: &str) -> AppResult<Option<String>> {
        let normalized = normalize_package_path(path);
        for index in 0..self.archive.len() {
            let mut file = self.archive.by_index(index).map_err(|error| {
                AppError::InvalidInput(format!("无法读取 ZIP 文件项: {}", error))
            })?;
            if normalize_package_path(file.name()).ends_with(&normalized) {
                let mut text = String::new();
                file.read_to_string(&mut text)?;
                return Ok(Some(text));
            }
        }
        Ok(None)
    }

    fn exists(&self, path: &str) -> bool {
        let normalized = normalize_package_path(path);
        self.names.iter().any(|name| name.ends_with(&normalized))
    }

    fn contains_raw_source_files(&self) -> bool {
        self.names.iter().any(|name| is_raw_source_path(name))
    }
}

struct FolderImportPackageReader {
    root: PathBuf,
    files: Vec<String>,
}

impl FolderImportPackageReader {
    fn new(folder_path: &str) -> AppResult<Self> {
        let root = PathBuf::from(folder_path);
        if !root.exists() {
            return Err(AppError::InvalidInput(format!(
                "数据包文件夹不存在: {}",
                folder_path
            )));
        }
        if !root.is_dir() {
            return Err(AppError::InvalidInput(format!(
                "数据包路径不是文件夹: {}",
                folder_path
            )));
        }
        let mut files = Vec::new();
        collect_folder_files(&root, &root, &mut files)?;
        Ok(Self { root, files })
    }

    fn path_for(&self, path: &str) -> PathBuf {
        self.root.join(normalize_package_path(path))
    }
}

impl ImportPackageReader for FolderImportPackageReader {
    fn package_root(&self) -> String {
        self.root.to_string_lossy().to_string()
    }

    fn read_text(&mut self, path: &str) -> AppResult<Option<String>> {
        let path = self.path_for(path);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(std::fs::read_to_string(path)?))
    }

    fn exists(&self, path: &str) -> bool {
        self.path_for(path).is_file()
    }

    fn contains_raw_source_files(&self) -> bool {
        self.files.iter().any(|name| is_raw_source_path(name))
    }
}

fn parse_zip_import_package(file_name: &str, content: &[u8]) -> AppResult<ParsedImportPackage> {
    let cursor = Cursor::new(content);
    let mut reader = ZipImportPackageReader::new(cursor, file_name.to_string())?;
    read_import_package(&mut reader)
}

pub(crate) fn parse_folder_import_package(folder_path: &str) -> AppResult<ParsedImportPackage> {
    let mut reader = FolderImportPackageReader::new(folder_path)?;
    read_import_package(&mut reader)
}

pub(crate) fn parse_path_import_package(package_path: &str) -> AppResult<ParsedImportPackage> {
    let path = Path::new(package_path);
    if path.is_dir() {
        parse_folder_import_package(package_path)
    } else if path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("zip"))
        .unwrap_or(false)
    {
        let bytes = std::fs::read(path)?;
        parse_zip_import_package(
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(package_path),
            &bytes,
        )
    } else {
        Err(AppError::InvalidInput(
            "Smart Import Center 当前只支持标准 ZIP 数据包或已解压文件夹。".to_string(),
        ))
    }
}

fn read_import_package<R: ImportPackageReader>(reader: &mut R) -> AppResult<ParsedImportPackage> {
    let mut warnings = Vec::new();
    let package_root = reader.package_root();
    let import_manifest = reader.read_text("import_manifest.json")?;
    let example_import_manifest = if import_manifest.is_none() {
        reader.read_text("import_manifest.example.json")?
    } else {
        None
    };
    let package_manifest = if import_manifest.is_none() {
        reader.read_text("manifest.json")?
    } else {
        None
    };

    if let Some(manifest_text) = import_manifest {
        return read_manifest_import_package(reader, package_root, manifest_text);
    }
    if let Some(manifest_text) = example_import_manifest {
        return read_manifest_import_package_with_path(
            reader,
            package_root,
            manifest_text,
            "import_manifest.example.json",
        );
    }

    if package_manifest.is_some() {
        warnings.push("检测到 package manifest，但不是 Import Engine V2 的 import_manifest.json，已按内置经典包规则自动查找可导入文件。".to_string());
    }

    read_auto_import_package(reader, package_root, warnings)
}

fn read_manifest_import_package<R: ImportPackageReader>(
    reader: &mut R,
    package_root: String,
    manifest_text: String,
) -> AppResult<ParsedImportPackage> {
    read_manifest_import_package_with_path(
        reader,
        package_root,
        manifest_text,
        "import_manifest.json",
    )
}

fn read_manifest_import_package_with_path<R: ImportPackageReader>(
    reader: &mut R,
    package_root: String,
    manifest_text: String,
    manifest_path: &str,
) -> AppResult<ParsedImportPackage> {
    let manifest_value: Value = serde_json::from_str(strip_json_bom(&manifest_text))?;
    let files = manifest_value
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidInput("manifest 缺少 files 数组".to_string()))?;
    let package_name = manifest_value
        .get("package_name")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let import_profile = manifest_value
        .get("import_profile")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let import_intent = manifest_value
        .get("import_intent")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let duplicate_policy = manifest_value
        .get("duplicate_policy")
        .and_then(Value::as_str)
        .map(ToString::to_string);
    let ai_assist = manifest_value.get("ai_assist").and_then(Value::as_bool);
    let import_order = manifest_value
        .get("import_order")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let mut file_entries = files
        .iter()
        .map(manifest_file_from_value)
        .collect::<AppResult<Vec<_>>>()?;
    file_entries.sort_by_key(|file| manifest_order_index(file, &import_order));

    let mut package_files = Vec::new();
    let mut primary_files = Vec::new();
    let mut auto_stage_files = Vec::new();
    let mut candidates = Vec::new();
    let mut primary_path = None;
    let mut primary_import_type = "zip_manifest".to_string();
    let mut package_search_terms = Vec::new();
    let mut warnings = Vec::new();
    let has_duplicate_direct_targets = has_duplicate_direct_import_targets(&file_entries);

    if let Some(name) = package_name.as_deref() {
        warnings.push(format!("数据包: {}", name));
    }
    warnings.push(format!("manifest 文件数: {}", file_entries.len()));
    if has_duplicate_direct_targets {
        warnings.push("检测到多个可导入文件指向同一目标表，系统仅自动暂存 primary 主文件，其余文件作为辅助文件保留，避免重复导入。".to_string());
    }

    for file in &file_entries {
        let exists = reader.exists(&file.path);
        if !exists {
            return Err(AppError::InvalidInput(format!(
                "manifest 指向的文件不存在: {}",
                file.path
            )));
        }
        warnings.push(format!(
            "文件: {} / 类型: {} / 目标: {} / 可直接导入: {}",
            file.path,
            file.import_type,
            file.target.as_deref().unwrap_or("未声明"),
            is_direct_package_import_type(&file.import_type)
        ));

        let mut record_count = None;
        let mut skip_reason = None;
        if file.import_type == "search_terms_v1" && should_auto_stage_manifest_file(file) {
            let rows = read_package_rows(reader, &file.path)?;
            record_count = Some(rows.len() as i64);
            primary_path = Some(file.path.clone());
            primary_import_type = file.import_type.clone();
            candidates.extend(rows);
            primary_files.push(file.path.clone());
            auto_stage_files.push(file.path.clone());
        } else if file.import_type == "search_terms_v1" {
            let rows = read_package_rows(reader, &file.path)?;
            record_count = Some(rows.len() as i64);
            warnings.push(format!("已读取搜索词文件 {}：{} 条", file.path, rows.len()));
            package_search_terms.extend(rows);
            if !file.primary {
                skip_reason = Some(
                    "搜索词辅助文件不会单独暂存；确认主知识文件后会按包内搜索词规则追加。"
                        .to_string(),
                );
            }
        } else if should_auto_stage_manifest_file(file) {
            let rows = read_package_rows(reader, &file.path)?;
            record_count = Some(rows.len() as i64);
            primary_path = Some(file.path.clone());
            primary_import_type = file.import_type.clone();
            candidates.extend(rows);
            primary_files.push(file.path.clone());
            auto_stage_files.push(file.path.clone());
        } else if is_direct_package_import_type(&file.import_type) {
            let rows = read_package_rows(reader, &file.path)?;
            record_count = Some(rows.len() as i64);
            skip_reason = Some(skip_reason_for_manifest_file(
                file,
                has_duplicate_direct_targets,
            ));
            warnings.push(format!(
                "已识别 {}，但它不是 primary 主数据文件；{}",
                file.path,
                skip_reason.as_deref().unwrap_or_default()
            ));
        } else {
            skip_reason = Some("当前版本暂不直接导入该 manifest 文件。".to_string());
            warnings.push(format!(
                "已识别 manifest 文件 {}，当前版本暂不直接导入目标 {}",
                file.path,
                file.target.as_deref().unwrap_or("未声明")
            ));
        }

        package_files.push(ImportPackageFile {
            path: file.path.clone(),
            import_type: file.import_type.clone(),
            target: file.target.clone(),
            primary: file.primary,
            role: file.role.clone(),
            auto_stage: file.auto_stage,
            description: file.description.clone(),
            skip_reason,
            required: file.required,
            exists,
            record_count,
        });
    }

    if package_search_terms.is_empty() {
        package_search_terms = read_standard_package_search_terms(reader, &mut warnings)?;
    }
    if candidates.is_empty() {
        return Err(AppError::InvalidInput(
            "manifest 中没有可导入到 knowledge_items 的 primary 主数据文件".to_string(),
        ));
    }
    attach_package_search_terms(&mut candidates, &package_search_terms);

    let detected = detect_package_rows(
        primary_path.as_deref().unwrap_or("import_manifest.json"),
        &primary_import_type,
        &candidates,
    );
    let direct_import_ready = import_engine_service::is_direct_import_type(&detected);
    let descriptor = ImportPackageDescriptor {
        package_root,
        package_name,
        import_profile,
        import_intent,
        duplicate_policy,
        ai_assist,
        manifest_found: true,
        manifest_path: Some(manifest_path.to_string()),
        primary_files,
        auxiliary_files: package_files
            .iter()
            .filter(|file| !file.primary)
            .cloned()
            .collect(),
        auto_stage_files,
        skipped_manifest_files: package_files
            .iter()
            .filter(|file| file.skip_reason.is_some())
            .cloned()
            .collect(),
        files: package_files,
        detected_type: detected,
        record_count: candidates.len() as i64,
        direct_import_ready,
        warnings,
        errors: Vec::new(),
    };

    Ok(ParsedImportPackage {
        descriptor,
        rows: candidates,
        import_type: primary_import_type,
    })
}

fn read_auto_import_package<R: ImportPackageReader>(
    reader: &mut R,
    package_root: String,
    mut warnings: Vec<String>,
) -> AppResult<ParsedImportPackage> {
    for path in STANDARD_PACKAGE_MAIN_PATHS {
        if reader.exists(path) {
            let mut rows = read_package_rows(reader, path)?;
            let mut search_warnings = Vec::new();
            let package_search_terms =
                read_standard_package_search_terms(reader, &mut search_warnings)?;
            warnings.extend(search_warnings);
            attach_package_search_terms(&mut rows, &package_search_terms);
            let detected_type = detect_package_rows(path, "zip_auto", &rows);
            let direct_import_ready = import_engine_service::is_direct_import_type(&detected_type);
            let descriptor = ImportPackageDescriptor {
                package_root,
                package_name: None,
                import_profile: None,
                import_intent: None,
                duplicate_policy: None,
                ai_assist: None,
                manifest_found: false,
                manifest_path: None,
                files: vec![ImportPackageFile {
                    path: (*path).to_string(),
                    import_type: detected_type.clone(),
                    target: Some("knowledge_items".to_string()),
                    primary: true,
                    role: Some("auto_discovered_primary".to_string()),
                    auto_stage: true,
                    description: None,
                    skip_reason: None,
                    required: true,
                    exists: true,
                    record_count: Some(rows.len() as i64),
                }],
                primary_files: vec![(*path).to_string()],
                auxiliary_files: Vec::new(),
                auto_stage_files: vec![(*path).to_string()],
                skipped_manifest_files: Vec::new(),
                detected_type,
                record_count: rows.len() as i64,
                direct_import_ready,
                warnings,
                errors: Vec::new(),
            };
            return Ok(ParsedImportPackage {
                descriptor,
                rows,
                import_type: "zip_auto".to_string(),
            });
        }
    }

    if reader.contains_raw_source_files() {
        return Err(AppError::InvalidInput(
            "PDF 原始资料不能直接导入，请先使用外部数据处理工具转换为标准 import_manifest 数据包。"
                .to_string(),
        ));
    }

    Err(AppError::InvalidInput(
        "当前文件夹不是标准导入数据包，请先通过外部工具处理为 import_manifest 数据包。".to_string(),
    ))
}

#[derive(Debug, Clone)]
struct ManifestFileEntry {
    path: String,
    import_type: String,
    target: Option<String>,
    primary: bool,
    role: Option<String>,
    auto_stage: bool,
    description: Option<String>,
    required: bool,
}

fn manifest_file_from_value(value: &Value) -> AppResult<ManifestFileEntry> {
    let path = value
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidInput("manifest 文件项缺少 path".to_string()))?;
    let primary = value
        .get("primary")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let auto_stage = value
        .get("auto_stage")
        .or_else(|| value.get("autoStage"))
        .and_then(Value::as_bool)
        .unwrap_or(primary);
    Ok(ManifestFileEntry {
        path: normalize_package_path(path),
        import_type: value
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("generic_json")
            .to_string(),
        target: value
            .get("target")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        primary,
        role: value
            .get("role")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        auto_stage,
        description: value
            .get("description")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        required: value
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(true),
    })
}

fn should_auto_stage_manifest_file(file: &ManifestFileEntry) -> bool {
    file.primary && file.auto_stage && is_direct_package_import_type(&file.import_type)
}

fn skip_reason_for_manifest_file(
    file: &ManifestFileEntry,
    has_duplicate_direct_targets: bool,
) -> String {
    if !file.primary && file.auto_stage {
        return "该文件声明 auto_stage: true，但不是 primary 主数据文件；当前版本先显示为可手动选择，不自动暂存，避免重复导入。".to_string();
    }
    if has_duplicate_direct_targets {
        return "非 primary 主数据文件，默认不自动暂存，避免重复导入。".to_string();
    }
    "该文件已识别为可导入数据，但不是 primary 主数据文件；系统默认不自动暂存。".to_string()
}

fn has_duplicate_direct_import_targets(files: &[ManifestFileEntry]) -> bool {
    let mut target_counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for file in files
        .iter()
        .filter(|file| is_direct_package_import_type(&file.import_type))
    {
        let key = format!(
            "{}|{}",
            file.import_type,
            file.target.as_deref().unwrap_or("未声明")
        );
        let entry = target_counts.entry(key).or_default();
        entry.0 += 1;
        if file.primary {
            entry.1 += 1;
        }
    }
    target_counts
        .values()
        .any(|(total_count, primary_count)| *total_count > 1 && *primary_count == 1)
}

fn manifest_order_index(file: &ManifestFileEntry, import_order: &[String]) -> usize {
    file.target
        .as_ref()
        .and_then(|target| import_order.iter().position(|item| item == target))
        .unwrap_or(import_order.len())
}

fn read_standard_package_search_terms<R: ImportPackageReader>(
    reader: &mut R,
    warnings: &mut Vec<String>,
) -> AppResult<Vec<Map<String, Value>>> {
    for path in STANDARD_PACKAGE_SEARCH_TERM_PATHS {
        if reader.exists(path) {
            let rows = read_package_rows(reader, path)?;
            warnings.push(format!("已自动读取搜索词文件 {}：{} 条", path, rows.len()));
            return Ok(rows);
        }
    }
    Ok(Vec::new())
}

fn read_package_rows<R: ImportPackageReader>(
    reader: &mut R,
    path: &str,
) -> AppResult<Vec<Map<String, Value>>> {
    let text = reader
        .read_text(path)?
        .ok_or_else(|| AppError::InvalidInput(format!("manifest 指向的文件不存在: {}", path)))?;
    if path.to_ascii_lowercase().ends_with(".csv") {
        parse_csv_rows(&text)
    } else {
        parse_json_rows(&text)
    }
}

fn detect_package_rows(path: &str, import_type: &str, rows: &[Map<String, Value>]) -> String {
    import_engine_service::detect_import_type(path, import_type, rows).detected_type
}

fn is_direct_package_import_type(import_type: &str) -> bool {
    matches!(
        import_type,
        "knowledge_items_v1"
            | "classic_passages_v1"
            | "annotation_items_v1"
            | "search_terms_v1"
            | "standard_terms_v1"
            | "relation_suggestions_v1"
    )
}

fn primary_file_name(descriptor: &ImportPackageDescriptor, fallback: &str) -> String {
    descriptor
        .primary_files
        .first()
        .cloned()
        .unwrap_or_else(|| fallback.to_string())
}

fn apply_package_descriptor_to_preview(
    preview: &mut ImportParsedPreview,
    descriptor: &ImportPackageDescriptor,
) {
    preview.direct_import_ready = descriptor.direct_import_ready;
    if descriptor.manifest_found {
        if let Some(import_profile) = descriptor.import_profile.as_deref() {
            preview.detection.detected_type = import_profile.to_string();
        }
        preview.detection.confidence = 0.99;
        preview.detection.reason =
            "检测到 import_manifest.json，按 manifest 驱动的标准数据包导入".to_string();
    }
    preview.warnings.extend(descriptor.warnings.clone());
}

const STANDARD_PACKAGE_MAIN_PATHS: &[&str] = &[
    "json/knowledge_items_import.json",
    "json/knowledge_items_import_curated.json",
    "json/knowledge_items_import_full_clean.json",
    "json/classic_passages_curated.json",
    "json/classic_passages_full_clean.json",
    "csv/knowledge_items_import.csv",
    "csv/knowledge_items_import_curated.csv",
    "csv/knowledge_items_import_full_clean.csv",
    "csv/classic_passages_curated.csv",
    "csv/classic_passages_full_clean.csv",
];

const STANDARD_PACKAGE_SEARCH_TERM_PATHS: &[&str] = &[
    "json/search_terms_curated.json",
    "json/search_terms.json",
    "csv/search_terms_curated.csv",
    "csv/search_terms.csv",
];

fn normalize_package_path(path: &str) -> String {
    path.replace('\\', "/").trim_start_matches('/').to_string()
}

fn is_raw_source_path(path: &str) -> bool {
    matches!(
        Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("pdf" | "doc" | "docx" | "png" | "jpg" | "jpeg" | "webp" | "tif" | "tiff")
    )
}

fn collect_folder_files(root: &Path, current: &Path, files: &mut Vec<String>) -> AppResult<()> {
    for entry in std::fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_folder_files(root, &path, files)?;
        } else if path.is_file() {
            if let Ok(relative) = path.strip_prefix(root) {
                files.push(normalize_package_path(&relative.to_string_lossy()));
            }
        }
    }
    Ok(())
}

fn strip_json_bom(content: &str) -> &str {
    content.trim_start_matches('\u{feff}')
}

fn attach_package_search_terms(
    rows: &mut [Map<String, Value>],
    search_terms: &[Map<String, Value>],
) {
    if search_terms.is_empty() {
        return;
    }
    let mut by_code: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    let mut by_name: BTreeMap<String, Vec<Value>> = BTreeMap::new();
    for term in search_terms {
        let value = Value::Object(term.clone());
        if let Some(code) = text(term, "item_code").or_else(|| text(term, "code")) {
            by_code.entry(code).or_default().push(value.clone());
        }
        if let Some(name) = text(term, "item_name").or_else(|| text(term, "name")) {
            by_name.entry(name).or_default().push(value);
        }
    }

    for row in rows {
        let mut attached = Vec::new();
        if let Some(code) = text(row, "code") {
            if let Some(values) = by_code.get(&code) {
                attached.extend(values.clone());
            }
        }
        if let Some(name) = text(row, "name") {
            if let Some(values) = by_name.get(&name) {
                attached.extend(values.clone());
            }
        }
        if !attached.is_empty() {
            row.insert(
                "_package_search_terms".to_string(),
                Value::Array(dedup_search_term_values(attached)),
            );
        }
    }
}

fn dedup_search_term_values(values: Vec<Value>) -> Vec<Value> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for value in values {
        let key = value.to_string();
        if seen.insert(key) {
            deduped.push(value);
        }
    }
    deduped
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
    let item_type = text(object, "type").unwrap_or_else(|| target_type.to_string());
    let name = text(object, "name").unwrap_or_default();
    let content = text(object, "content")
        .map(|value| sanitize_import_content(&value, &name))
        .filter(|value| !value.trim().is_empty());
    transaction.execute(
        "INSERT INTO knowledge_items
         (type, code, name, alias, pinyin, category, summary, content, source_note, tags,
          data_status, completeness_status, content_version, is_favorite, detail, source_package, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'partial', 1, 0, ?12, ?13, ?14, ?15)",
        params![
            item_type,
            text(object, "code"),
            name,
            text(object, "alias"),
            text(object, "pinyin"),
            text(object, "category"),
            text(object, "summary"),
            content,
            text(object, "source_note"),
            text(object, "tags"),
            text(object, "data_status").unwrap_or_else(|| "imported".to_string()),
            detail_json(object),
            text(object, "_source_package"),
            now,
            now
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

fn upsert_import_fingerprint(
    transaction: &rusqlite::Transaction<'_>,
    item_id: i64,
) -> AppResult<()> {
    transaction.execute(
        "INSERT INTO knowledge_fingerprints
         (item_id, type, code_norm, name_norm, pinyin_norm, alias_norm, fingerprint)
         SELECT id, type,
                upper(replace(replace(COALESCE(code, ''), ' ', ''), '-', '')),
                lower(replace(COALESCE(name, ''), ' ', '')),
                lower(replace(COALESCE(pinyin, ''), ' ', '')),
                lower(COALESCE(alias, '')),
                lower(type || '|' || COALESCE(code, '') || '|' || name || '|' || COALESCE(source_note, ''))
         FROM knowledge_items
         WHERE id = ?1
         ON CONFLICT(item_id) DO UPDATE SET
           type = excluded.type,
           code_norm = excluded.code_norm,
           name_norm = excluded.name_norm,
           pinyin_norm = excluded.pinyin_norm,
           alias_norm = excluded.alias_norm,
           fingerprint = excluded.fingerprint",
        params![item_id],
    )?;
    Ok(())
}

fn import_search_terms_for_items(
    database: &Database,
    rows: &[crate::models::data_pipeline::DataImportRow],
    item_ids: &[i64],
) -> AppResult<i64> {
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        let mut imported = 0_i64;
        for (row, item_id) in rows.iter().zip(item_ids.iter()) {
            let value = parse_optional_json(row.normalized_json.clone())?;
            let object = value.as_object().cloned().unwrap_or_default();
            let mut seen = HashSet::new();
            let mut terms = Vec::new();
            for package_term in package_search_terms(&object) {
                terms.push(package_term);
            }
            for field in ["name", "code", "category", "tags"] {
                if let Some(value) = text(&object, field) {
                    terms.extend(
                        split_search_terms(&value)
                            .into_iter()
                            .map(|term| (term, "imported_package".to_string(), 80)),
                    );
                }
            }
            for (term, term_type, weight) in terms {
                let normalized = search_repository::normalize_for_search(&term);
                let dedup_key = format!("{normalized}|{term_type}");
                if normalized.is_empty() || !seen.insert(dedup_key) {
                    continue;
                }
                transaction.execute(
                    "INSERT INTO search_terms (item_id, term, term_type, weight)
                     SELECT ?1, ?2, ?3, ?4
                     WHERE NOT EXISTS (
                       SELECT 1 FROM search_terms
                       WHERE item_id = ?1 AND term = ?2 AND term_type = ?3
                     )",
                    params![item_id, normalized, term_type, weight],
                )?;
                if transaction.changes() > 0 {
                    imported += 1;
                }
            }
        }
        transaction.commit()?;
        Ok(imported)
    })
}

fn set_search_terms_imported_count(
    database: &Database,
    batch_id: i64,
    imported_count: i64,
) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute(
            "UPDATE data_import_batches SET search_terms_imported_count = ?2 WHERE id = ?1",
            params![batch_id, imported_count],
        )?;
        Ok(())
    })
}

fn should_import_package_terms(import_type: &str) -> bool {
    matches!(
        import_type,
        "zip_manifest" | "classics_curated_v1" | "knowledge_items_v1" | "classic_passages_v1"
    )
}

fn split_search_terms(value: &str) -> Vec<String> {
    value
        .split([',', '，', ';', '；', '|', '/', '、'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn package_search_terms(object: &Map<String, Value>) -> Vec<(String, String, i64)> {
    object
        .get("_package_search_terms")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_object)
        .filter_map(|term| {
            let value = text(term, "term")?;
            let term_type =
                text(term, "term_type").unwrap_or_else(|| "imported_package".to_string());
            let weight = term
                .get("weight")
                .and_then(Value::as_i64)
                .or_else(|| text(term, "weight").and_then(|value| value.parse::<i64>().ok()))
                .unwrap_or(80)
                .clamp(1, 200);
            Some((value, format!("package_{term_type}"), weight))
        })
        .collect()
}

fn existing_duplicate_warnings(
    database: &Database,
    row: &Map<String, Value>,
) -> AppResult<Vec<StagingIssue>> {
    let item_type = text(row, "type").unwrap_or_default();
    let code = text(row, "code");
    let name = text(row, "name");
    if item_type.is_empty() || (code.is_none() && name.is_none()) {
        return Ok(Vec::new());
    }

    let duplicate = database.with_connection(|connection| {
        let mut duplicate = None;
        if let Some(code) = code.as_deref() {
            duplicate = connection
                .query_row(
                    "SELECT id, name FROM knowledge_items
                     WHERE type = ?1 AND upper(replace(replace(COALESCE(code, ''), ' ', ''), '-', '')) = ?2
                     LIMIT 1",
                    params![item_type, normalize_code_for_duplicate(code)],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, String>(1)?,
                            "code".to_string(),
                        ))
                    },
                )
                .optional()?;
        }
        if duplicate.is_none() {
            if let Some(name) = name.as_deref() {
                duplicate = connection
                    .query_row(
                        "SELECT id, name FROM knowledge_items
                         WHERE type = ?1 AND lower(replace(COALESCE(name, ''), ' ', '')) = ?2
                         LIMIT 1",
                        params![item_type, normalize_name_for_duplicate(name)],
                        |row| {
                            Ok((
                                row.get::<_, i64>(0)?,
                                row.get::<_, String>(1)?,
                                "name".to_string(),
                            ))
                        },
                    )
                    .optional()?;
            }
        }
        Ok(duplicate)
    })?;

    Ok(duplicate
        .map(|(id, name, field_name)| {
            vec![StagingIssue {
                severity: "warning".to_string(),
                issue_code: "possible_existing_duplicate".to_string(),
                field_name: Some(field_name),
                message: format!("正式库中存在疑似重复条目 #{id}: {name}"),
                suggestion: Some("确认是否需要跳过、合并或保留为新版本后再入库".to_string()),
            }]
        })
        .unwrap_or_default())
}

fn normalize_code_for_duplicate(code: &str) -> String {
    code.replace([' ', '-'], "").to_ascii_uppercase()
}

fn normalize_name_for_duplicate(name: &str) -> String {
    name.replace(' ', "").to_lowercase()
}

fn row_fingerprint(object: &Map<String, Value>) -> String {
    [
        text(object, "type").unwrap_or_default(),
        text(object, "code").unwrap_or_default(),
        text(object, "name").unwrap_or_default(),
        text(object, "source_note").unwrap_or_default(),
    ]
    .join("|")
    .to_lowercase()
}

fn search_keywords(database: &Database, keywords: &[&str]) -> AppResult<BTreeMap<String, bool>> {
    let mut result = BTreeMap::new();
    for keyword in keywords {
        let response = search_index_service::search(
            database,
            crate::models::search::SearchRequest {
                query: (*keyword).to_string(),
                item_type: None,
                page: Some(1),
                page_size: Some(1),
            },
        );
        result.insert(
            (*keyword).to_string(),
            response
                .map(|response| !response.results.is_empty())
                .unwrap_or(false),
        );
    }
    Ok(result)
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
                 (item_id, nature_flavor, four_qi, five_flavors, meridians, channel_tropism, toxicity, origin, effects, indications, dosage, contraindications, compatibility, processing, classic_applications, notes, property_notes)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![item_id, text(object, "nature_flavor"), text(object, "four_qi").or_else(|| text(object, "fourQi")), text(object, "five_flavors").or_else(|| text(object, "fiveFlavors")), text(object, "meridians"), text(object, "channel_tropism").or_else(|| text(object, "channelTropism")), text(object, "toxicity"), text(object, "origin"), text(object, "effects"), text(object, "indications"), text(object, "dosage"), text(object, "contraindications"), text(object, "compatibility"), text(object, "processing"), text(object, "classic_applications").or_else(|| text(object, "classicApplications")), text(object, "notes"), text(object, "property_notes").or_else(|| text(object, "propertyNotes"))],
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

fn is_maintenance_import_type(import_type: &str) -> bool {
    matches!(
        import_type,
        "search_terms_v1" | "standard_terms_v1" | "relation_suggestions_v1"
    )
}

fn validate_maintenance_row(import_type: &str, row: &Map<String, Value>) -> Vec<StagingIssue> {
    let mut issues = Vec::new();
    match import_type {
        "standard_terms_v1" => {
            required_issue(&mut issues, row, "term_type", "term_type 不能为空");
            required_issue(&mut issues, row, "standard_name", "standard_name 不能为空");
        }
        "search_terms_v1" => {
            required_issue(&mut issues, row, "term", "term 不能为空");
            if text(row, "item_id").is_none()
                && text(row, "item_code").is_none()
                && text(row, "item_name").is_none()
            {
                issues.push(staging_error(
                    "required",
                    Some("item_id"),
                    "搜索词必须提供 item_id、item_code 或 item_name",
                ));
            }
        }
        "relation_suggestions_v1" => {
            required_issue(&mut issues, row, "relation_type", "relation_type 不能为空");
            if text(row, "source_item_id").is_none()
                && text(row, "source_code").is_none()
                && text(row, "source_name").is_none()
            {
                issues.push(staging_error(
                    "required",
                    Some("source_item_id"),
                    "关系来源必须提供 source_item_id、source_code 或 source_name",
                ));
            }
            if text(row, "target_item_id").is_none()
                && text(row, "target_code").is_none()
                && text(row, "target_name").is_none()
            {
                issues.push(staging_error(
                    "required",
                    Some("target_item_id"),
                    "关系目标必须提供 target_item_id、target_code 或 target_name",
                ));
            }
        }
        _ => {}
    }
    issues
}

fn required_issue(
    issues: &mut Vec<StagingIssue>,
    row: &Map<String, Value>,
    field: &'static str,
    message: &str,
) {
    if text(row, field).is_none() {
        issues.push(staging_error("required", Some(field), message));
    }
}

fn staging_error(code: &str, field: Option<&str>, message: &str) -> StagingIssue {
    StagingIssue {
        severity: "error".to_string(),
        issue_code: code.to_string(),
        field_name: field.map(ToString::to_string),
        message: message.to_string(),
        suggestion: None,
    }
}

fn required_text(object: &Map<String, Value>, field: &str) -> AppResult<String> {
    text(object, field).ok_or_else(|| AppError::InvalidInput(format!("{field} 不能为空")))
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

fn number_i64(object: &Map<String, Value>, field: &str) -> Option<i64> {
    match object.get(field) {
        Some(Value::Number(number)) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|v| v as i64)),
        Some(Value::String(text)) => text.trim().parse::<i64>().ok(),
        _ => None,
    }
}

fn number_f64(object: &Map<String, Value>, field: &str) -> Option<f64> {
    match object.get(field) {
        Some(Value::Number(number)) => number.as_f64(),
        Some(Value::String(text)) => text.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn sanitize_import_content(content: &str, item_name: &str) -> String {
    let current = content
        .split("【原PDF对应页完整文本校对备份】")
        .next()
        .unwrap_or(content);
    let source_lines = current.lines().map(str::trim).collect::<Vec<_>>();
    let item_key = compact_text(item_name);
    let mut output: Vec<String> = Vec::new();
    let mut seen_short = HashSet::new();

    for (index, line) in source_lines.iter().enumerate() {
        if line.is_empty() {
            if output.last().is_some_and(|line| !line.is_empty()) {
                output.push(String::new());
            }
            continue;
        }
        if is_import_layout_noise(line) {
            continue;
        }
        let normalized_line = line.split_whitespace().collect::<Vec<_>>().join(" ");
        let line_key = compact_text(&normalized_line);
        let next_key = source_lines
            .iter()
            .skip(index + 1)
            .find(|candidate| !candidate.trim().is_empty())
            .map(|candidate| compact_text(candidate))
            .unwrap_or_default();
        let previous_key = output
            .last()
            .map(|line| compact_text(line))
            .unwrap_or_default();

        if line_key == previous_key {
            continue;
        }
        if !item_key.is_empty()
            && index < 12
            && line_key == item_key
            && output.iter().any(|part| {
                compact_text(part) == item_key || compact_text(part).starts_with(&item_key)
            })
        {
            continue;
        }
        if line_key.chars().count() <= 6
            && next_key.starts_with(&line_key)
            && next_key.len() > line_key.len()
        {
            continue;
        }
        if line_key.chars().count() <= 6 && seen_short.contains(&line_key) && next_key != line_key {
            continue;
        }

        output.push(normalized_line);
        if line_key.chars().count() <= 6 {
            seen_short.insert(line_key);
        }
    }

    let mut text = output.join("\n");
    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    text.trim().to_string()
}

fn is_import_layout_noise(line: &str) -> bool {
    let text = compact_text(line);
    if text.is_empty() {
        return false;
    }
    matches!(
        text.as_str(),
        "倪海厦注" | "倪海厦注《金匮》" | "倪海厦注金匮" | "倪注金匮" | "校排" | "呚"
    ) || text.contains("勤求古訓博采眾方")
        || text.contains("勤求古训博采众方")
        || (text.contains("群龙无首") && text.contains("校排"))
        || (text.starts_with("【PDF页码") && text.ends_with('】'))
        || (text.ends_with("校排")
            && text.chars().all(|ch| {
                ch.is_ascii_digit() || matches!(ch, '.' | '。' | '．') || ch == '校' || ch == '排'
            }))
}

fn compact_text(value: &str) -> String {
    value.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn detail_json(object: &Map<String, Value>) -> String {
    object
        .get("detail")
        .map(normalize_detail_json)
        .unwrap_or_else(|| {
            let mut detail = Map::new();
            for (key, value) in object {
                if !matches!(
                    key.as_str(),
                    "type"
                        | "code"
                        | "name"
                        | "alias"
                        | "pinyin"
                        | "category"
                        | "summary"
                        | "content"
                        | "source_note"
                        | "tags"
                        | "data_status"
                ) && !key.starts_with('_')
                {
                    detail.insert(key.clone(), value.clone());
                }
            }
            Value::Object(detail).to_string()
        })
}

fn normalize_detail_json(value: &Value) -> String {
    match value {
        Value::Object(_) => value.to_string(),
        Value::String(text) => serde_json::from_str::<Value>(text)
            .map(|value| value.to_string())
            .unwrap_or_else(|_| {
                serde_json::json!({ "raw_detail": text, "parse_error": true }).to_string()
            }),
        Value::Null => "{}".to_string(),
        other => serde_json::json!({ "raw_detail": other }).to_string(),
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
    use super::{
        confirm_import, import_csv, import_json, import_package_folder, import_quality_report,
        import_zip, preview_package_folder, preview_zip, rollback_import_batch,
        sanitize_import_content,
    };
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
    fn sanitize_import_content_removes_pdf_layout_noise() {
        let content = "乌头汤方 治脚气疼痛,不可屈伸。\n乌头汤方\n麻黄\n麻黄三两\n倪海厦注\n倪海厦注《金匮》\n勤求古訓 博采眾方\n乌头\n呚\n五枚\n【原PDF对应页完整文本校对备份】\n【PDF页码106】\n倪海厦注\n校排";
        let cleaned = sanitize_import_content(content, "乌头汤方");
        assert!(cleaned.contains("乌头汤方 治脚气疼痛,不可屈伸。"));
        assert!(cleaned.contains("麻黄三两"));
        assert!(cleaned.contains("五枚"));
        assert!(!cleaned.contains("倪海厦注"));
        assert!(!cleaned.contains("勤求古訓"));
        assert!(!cleaned.contains("原PDF对应页"));
        assert!(!cleaned.contains("\n麻黄\n麻黄"));
        assert!(!cleaned.contains("呚"));
    }

    fn write_folder_package(root: &std::path::Path, manifest: Option<&str>, knowledge_path: &str) {
        fs::create_dir_all(root.join("json")).expect("create json dir");
        if let Some(manifest) = manifest {
            fs::write(root.join("import_manifest.json"), manifest).expect("write manifest");
        }
        fs::write(
            root.join(knowledge_path),
            r#"[
              {
                "type": "formula",
                "code": "FOLDER-GZT-001",
                "name": "文件夹桂枝汤",
                "content": "桂枝汤原文用于文件夹数据包导入测试。",
                "source_note": "伤寒论",
                "tags": ["方剂", "文件夹导入"]
              }
            ]"#,
        )
        .expect("write knowledge file");
    }

    fn folder_manifest(path: &str) -> String {
        format!(
            r#"{{
              "package_name": "folder_classics_package",
              "schema_version": "1.0",
              "import_profile": "classics_curated_v1",
              "files": [
                {{
                  "path": "{path}",
                  "type": "knowledge_items_v1",
                  "target": "knowledge_items",
                  "primary": true
                }}
              ],
              "import_order": ["knowledge_items"]
            }}"#
        )
    }

    fn folder_manifest_with_auxiliary(auto_stage: bool) -> String {
        format!(
            r#"{{
              "package_name": "folder_classics_package",
              "schema_version": "1.0",
              "import_profile": "classics_curated_v1",
              "files": [
                {{
                  "path": "json/knowledge_items_import.json",
                  "type": "knowledge_items_v1",
                  "target": "knowledge_items",
                  "primary": true,
                  "role": "main_knowledge_items",
                  "auto_stage": true
                }},
                {{
                  "path": "json/herb_items_import.json",
                  "type": "knowledge_items_v1",
                  "target": "knowledge_items",
                  "primary": false,
                  "required": false,
                  "role": "auxiliary_export",
                  "auto_stage": {auto_stage},
                  "description": "中药条目辅助导出文件，通常已包含在主知识文件中，默认不自动导入，避免重复。"
                }}
              ],
              "import_order": ["knowledge_items"]
            }}"#
        )
    }

    fn write_auxiliary_herb_file(root: &std::path::Path) {
        fs::write(
            root.join("json/herb_items_import.json"),
            r#"[
              {
                "type": "herb",
                "code": "FOLDER-HERB-001",
                "name": "文件夹甘草",
                "content": "甘草辅助导出文件，用于验证非主文件不自动暂存。",
                "source_note": "神农本草经",
                "tags": ["中药", "辅助导出"]
              }
            ]"#,
        )
        .expect("write auxiliary herb file");
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
    fn maintenance_imports_write_standard_terms_search_terms_and_relation_suggestions() {
        let (data_dir, database) = temp_database("maintenance-imports");
        let knowledge = r#"
        [
          {
            "type": "herb",
            "code": "HERB-HQ",
            "name": "黄芪",
            "content": "黄芪，味甘微温，补气固表。",
            "source_note": "神农本草经",
            "tags": ["中药", "补气"]
          },
          {
            "type": "formula",
            "code": "FORM-BZYQT",
            "name": "补中益气汤",
            "content": "补中益气汤，用于中气不足。",
            "source_note": "方剂资料",
            "tags": ["方剂", "补气"]
          }
        ]
        "#;
        let summary = import_json(
            &database,
            CreateImportRequest {
                file_name: "knowledge_items_import.json".to_string(),
                target_type: "mixed".to_string(),
                content: knowledge.to_string(),
                mapping: None,
                template_id: None,
            },
        )
        .expect("stage knowledge");
        confirm_import(&database, summary.batch.id.unwrap()).expect("confirm knowledge");

        let standard_terms = r#"
        [
          {
            "term_type": "herb_name",
            "standard_name": "黄芪",
            "aliases": ["黄耆", "绵黄芪"],
            "code": "HERB-HQ",
            "notes": "维护导入"
          }
        ]
        "#;
        let standard_summary = import_json(
            &database,
            CreateImportRequest {
                file_name: "standard_terms_import.json".to_string(),
                target_type: "mixed".to_string(),
                content: standard_terms.to_string(),
                mapping: None,
                template_id: None,
            },
        )
        .expect("stage standard terms");
        assert_eq!(standard_summary.batch.import_type, "standard_terms_v1");
        let standard_result =
            confirm_import(&database, standard_summary.batch.id.unwrap()).expect("confirm terms");
        assert_eq!(standard_result.imported_count, 1);

        let search_terms = r#"
        [
          {
            "item_code": "FORM-BZYQT",
            "term": "中气下陷",
            "term_type": "keyword",
            "weight": 150
          }
        ]
        "#;
        let search_summary = import_json(
            &database,
            CreateImportRequest {
                file_name: "search_terms_import.json".to_string(),
                target_type: "mixed".to_string(),
                content: search_terms.to_string(),
                mapping: None,
                template_id: None,
            },
        )
        .expect("stage search terms");
        assert_eq!(search_summary.batch.import_type, "search_terms_v1");
        let search_result =
            confirm_import(&database, search_summary.batch.id.unwrap()).expect("confirm search");
        assert_eq!(search_result.imported_count, 1);

        let relation_suggestions = r#"
        [
          {
            "source_code": "FORM-BZYQT",
            "target_code": "HERB-HQ",
            "relation_type": "contains",
            "confidence": 0.92,
            "reason": "方中包含黄芪"
          }
        ]
        "#;
        let relation_summary = import_json(
            &database,
            CreateImportRequest {
                file_name: "relation_suggestions_import.json".to_string(),
                target_type: "mixed".to_string(),
                content: relation_suggestions.to_string(),
                mapping: None,
                template_id: None,
            },
        )
        .expect("stage relation suggestions");
        assert_eq!(
            relation_summary.batch.import_type,
            "relation_suggestions_v1"
        );
        let relation_result = confirm_import(&database, relation_summary.batch.id.unwrap())
            .expect("confirm relation suggestions");
        assert_eq!(relation_result.imported_count, 1);

        database
            .with_connection(|connection| {
                let standard_count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM standard_terms
                     WHERE term_type = 'herb_name' AND standard_name = '黄芪'
                       AND aliases LIKE '%黄耆%'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(standard_count, 1);

                let term_count: i64 = connection.query_row(
                    "SELECT COUNT(1)
                     FROM search_terms st
                     JOIN knowledge_items ki ON ki.id = st.item_id
                     WHERE ki.code = 'FORM-BZYQT' AND st.term = '中气下陷'
                       AND st.term_type = 'keyword' AND st.weight = 150",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(term_count, 1);

                let suggestion_count: i64 = connection.query_row(
                    "SELECT COUNT(1)
                     FROM relation_suggestions rs
                     JOIN knowledge_items source ON source.id = rs.source_item_id
                     JOIN knowledge_items target ON target.id = rs.target_item_id
                     WHERE source.code = 'FORM-BZYQT' AND target.code = 'HERB-HQ'
                       AND rs.relation_type = 'contains' AND rs.status = 'pending'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(suggestion_count, 1);
                Ok(())
            })
            .expect("inspect maintenance imports");

        let response = search_index_service::search(
            &database,
            SearchRequest {
                query: "中气下陷".to_string(),
                item_type: Some("formula".to_string()),
                page: Some(1),
                page_size: Some(10),
            },
        )
        .expect("search imported term");
        assert_eq!(
            response.results.first().map(|hit| hit.item_id).is_some(),
            true
        );

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn zip_manifest_can_stage_primary_maintenance_file() {
        let (data_dir, database) = temp_database("maintenance-zip");
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default();
            writer.start_file("import_manifest.json", options).unwrap();
            writer
                .write_all(
                    r#"{
                "package_name": "maintenance_terms",
                "files": [
                  {
                    "path": "json/standard_terms_import.json",
                    "type": "standard_terms_v1",
                    "target": "standard_terms",
                    "primary": true,
                    "auto_stage": true
                  }
                ],
                "import_order": ["standard_terms"]
            }"#
                    .as_bytes(),
                )
                .unwrap();
            writer
                .start_file("json/standard_terms_import.json", options)
                .unwrap();
            writer
                .write_all(
                    r#"[{
                "term_type": "meridian",
                "standard_name": "足太阴脾经",
                "aliases": ["脾经", "SP"],
                "code": "SP",
                "notes": "ZIP 维护包导入"
            }]"#
                    .as_bytes(),
                )
                .unwrap();
            writer.finish().unwrap();
        }
        let bytes = buffer.into_inner();

        let preview = preview_zip("maintenance_terms.zip", &bytes).expect("preview zip");
        assert_eq!(preview.detection.detected_type, "standard_terms_v1");
        assert_eq!(preview.rows.len(), 1);

        let summary = import_zip(
            &database,
            CreateImportRequest {
                file_name: "maintenance_terms.zip".to_string(),
                target_type: "mixed".to_string(),
                content: String::new(),
                mapping: None,
                template_id: None,
            },
            &bytes,
        )
        .expect("import maintenance zip");
        assert_eq!(summary.batch.import_type, "standard_terms_v1");
        confirm_import(&database, summary.batch.id.unwrap()).expect("confirm maintenance zip");

        database
            .with_connection(|connection| {
                let count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM standard_terms
                     WHERE term_type = 'meridian' AND standard_name = '足太阴脾经'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(count, 1);
                Ok(())
            })
            .expect("inspect imported term");

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
                    r#"{
                "package_name": "zhongyi_classics_curated_v0_3",
                "schema_version": "1.0",
                "import_profile": "classics_curated_v1",
                "files": [
                  {
                    "path": "json/knowledge_items_import_curated.json",
                    "type": "knowledge_items_v1",
                    "target": "knowledge_items",
                    "primary": true,
                    "role": "main_knowledge_items",
                    "auto_stage": true
                  },
                  {
                    "path": "json/herb_items_import.json",
                    "type": "knowledge_items_v1",
                    "target": "knowledge_items",
                    "primary": false,
                    "required": false,
                    "role": "auxiliary_export",
                    "auto_stage": false,
                    "description": "中药条目辅助导出文件，通常已包含在主知识文件中，默认不自动导入，避免重复。"
                  }
                ],
                "import_order": ["knowledge_items"]
            }"#
                    .as_bytes(),
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
            writer
                .start_file("json/herb_items_import.json", options)
                .unwrap();
            writer
                .write_all(
                    r#"[
              {
                "type": "herb",
                "code": "GC-001",
                "name": "甘草",
                "content": "甘草辅助导出文件，不应自动暂存。",
                "source_note": "神农本草经",
                "tags": ["中药", "辅助导出"]
              }
            ]"#
                    .as_bytes(),
                )
                .unwrap();
            writer
                .start_file("json/search_terms_curated.json", options)
                .unwrap();
            writer
                .write_all(
                    r#"[
              {
                "item_code": "GZT-001",
                "term": "营卫不和",
                "term_type": "keyword",
                "weight": 120
              }
            ]"#
                    .as_bytes(),
                )
                .unwrap();
            writer.finish().unwrap();
        }
        let bytes = buffer.into_inner();

        let preview =
            preview_zip("zhongyi_classics_curated_v0_3_manifest.zip", &bytes).expect("preview zip");
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("检测到多个可导入文件指向同一目标表")));
        assert!(preview
            .warnings
            .iter()
            .any(|warning| warning.contains("json/herb_items_import.json")));

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
        let batch_id = summary.batch.id.unwrap();
        confirm_import(&database, batch_id).expect("confirm zip import");

        let report = import_quality_report(&database, batch_id).expect("quality report");
        assert_eq!(report.detected_type, "knowledge_items_v1");
        assert_eq!(report.total_rows, 1);
        assert_eq!(report.duplicate_fingerprint_count, 0);
        assert!(report.search_terms_imported_count >= 3);
        assert!(
            report
                .field_coverage
                .get("content")
                .copied()
                .unwrap_or_default()
                >= 1.0
        );

        database
            .with_connection(|connection| {
                let confirmed_item_ids_json: String = connection.query_row(
                    "SELECT confirmed_item_ids_json FROM data_import_batches WHERE id = ?1",
                    [batch_id],
                    |row| row.get(0),
                )?;
                let confirmed_item_ids: Vec<i64> =
                    serde_json::from_str(&confirmed_item_ids_json).expect("confirmed ids json");
                assert_eq!(confirmed_item_ids.len(), 1);

                let imported_terms: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM search_terms WHERE term_type = 'imported_package'",
                    [],
                    |row| row.get(0),
                )?;
                assert!(imported_terms >= 3);

                let package_keyword: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM search_terms WHERE term = '营卫不和' AND term_type = 'package_keyword'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(package_keyword, 1);
                Ok(())
            })
            .expect("inspect import quality fields");

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

    #[test]
    fn import_staging_warns_when_existing_item_has_same_type_and_code() {
        let (data_dir, database) = temp_database("duplicate-warning");
        let first = r#"
        [
          {
            "type": "formula",
            "code": "DUP-GZT-001",
            "name": "重复预警桂枝汤",
            "content": "第一次导入。",
            "source_note": "测试"
          }
        ]
        "#;
        let first_summary = import_json(
            &database,
            CreateImportRequest {
                file_name: "knowledge_items_import_curated.json".to_string(),
                target_type: "mixed".to_string(),
                content: first.to_string(),
                mapping: None,
                template_id: None,
            },
        )
        .expect("stage first item");
        confirm_import(&database, first_summary.batch.id.unwrap()).expect("confirm first item");

        let second_summary = import_json(
            &database,
            CreateImportRequest {
                file_name: "knowledge_items_import_curated.json".to_string(),
                target_type: "mixed".to_string(),
                content: first.to_string(),
                mapping: None,
                template_id: None,
            },
        )
        .expect("stage duplicate item");

        assert_eq!(second_summary.warning_rows, 1);
        database
            .with_connection(|connection| {
                let issue_count: i64 = connection.query_row(
                    "SELECT COUNT(1)
                     FROM data_validation_issues
                     WHERE batch_id = ?1 AND issue_code = 'possible_existing_duplicate'",
                    [second_summary.batch.id.unwrap()],
                    |row| row.get(0),
                )?;
                assert_eq!(issue_count, 1);
                Ok(())
            })
            .expect("inspect duplicate warning");

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn package_folder_preview_reads_root_import_manifest() {
        let (data_dir, _database) = temp_database("folder-manifest-preview");
        let package_dir = data_dir.join("folder_package");
        write_folder_package(
            &package_dir,
            Some(&folder_manifest("json/knowledge_items_import.json")),
            "json/knowledge_items_import.json",
        );

        let descriptor =
            preview_package_folder(package_dir.to_str().unwrap()).expect("preview package folder");
        assert!(descriptor.manifest_found);
        assert_eq!(
            descriptor.package_name.as_deref(),
            Some("folder_classics_package")
        );
        assert_eq!(
            descriptor.import_profile.as_deref(),
            Some("classics_curated_v1")
        );
        assert_eq!(
            descriptor.primary_files,
            vec!["json/knowledge_items_import.json".to_string()]
        );
        assert_eq!(descriptor.record_count, 1);
        assert!(descriptor.direct_import_ready);

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn package_folder_manifest_allows_utf8_bom() {
        let (data_dir, _database) = temp_database("folder-manifest-bom");
        let package_dir = data_dir.join("folder_package");
        let manifest = format!(
            "\u{feff}{}",
            folder_manifest("json/knowledge_items_import.json")
        );
        write_folder_package(
            &package_dir,
            Some(&manifest),
            "json/knowledge_items_import.json",
        );

        let descriptor = preview_package_folder(package_dir.to_str().unwrap())
            .expect("preview package folder with bom");
        assert!(descriptor.manifest_found);
        assert_eq!(descriptor.record_count, 1);

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn package_folder_manifest_missing_file_returns_clear_error() {
        let (data_dir, _database) = temp_database("folder-manifest-missing");
        let package_dir = data_dir.join("folder_package");
        fs::create_dir_all(&package_dir).expect("create package dir");
        fs::write(
            package_dir.join("import_manifest.json"),
            folder_manifest("json/missing_knowledge_items_import.json"),
        )
        .expect("write manifest");

        let error = preview_package_folder(package_dir.to_str().unwrap())
            .expect_err("missing manifest file should fail")
            .to_string();
        assert!(error.contains("manifest 指向的文件不存在"));
        assert!(error.contains("json/missing_knowledge_items_import.json"));

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn package_folder_without_manifest_detects_standard_knowledge_file() {
        let (data_dir, _database) = temp_database("folder-auto-knowledge");
        let package_dir = data_dir.join("folder_package");
        write_folder_package(&package_dir, None, "json/knowledge_items_import.json");

        let descriptor = preview_package_folder(package_dir.to_str().unwrap())
            .expect("preview package folder without manifest");
        assert!(!descriptor.manifest_found);
        assert_eq!(descriptor.detected_type, "knowledge_items_v1");
        assert_eq!(
            descriptor.primary_files,
            vec!["json/knowledge_items_import.json".to_string()]
        );
        assert_eq!(descriptor.record_count, 1);
        assert!(descriptor.direct_import_ready);

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn manifest_auxiliary_files_are_read_but_not_auto_staged() {
        let (data_dir, _database) = temp_database("folder-auxiliary-manifest");
        let package_dir = data_dir.join("folder_package");
        write_folder_package(
            &package_dir,
            Some(&folder_manifest_with_auxiliary(false)),
            "json/knowledge_items_import.json",
        );
        write_auxiliary_herb_file(&package_dir);

        let descriptor = preview_package_folder(package_dir.to_str().unwrap())
            .expect("preview package folder with auxiliary");
        assert_eq!(
            descriptor.primary_files,
            vec!["json/knowledge_items_import.json".to_string()]
        );
        assert_eq!(
            descriptor.auto_stage_files,
            vec!["json/knowledge_items_import.json".to_string()]
        );
        assert_eq!(descriptor.record_count, 1);
        assert_eq!(descriptor.auxiliary_files.len(), 1);
        let auxiliary = &descriptor.auxiliary_files[0];
        assert_eq!(auxiliary.path, "json/herb_items_import.json");
        assert_eq!(auxiliary.role.as_deref(), Some("auxiliary_export"));
        assert!(!auxiliary.auto_stage);
        assert_eq!(auxiliary.record_count, Some(1));
        assert!(auxiliary
            .description
            .as_deref()
            .unwrap_or_default()
            .contains("中药条目辅助导出文件"));
        assert!(auxiliary
            .skip_reason
            .as_deref()
            .unwrap_or_default()
            .contains("非 primary 主数据文件"));
        assert_eq!(descriptor.skipped_manifest_files.len(), 1);
        assert!(descriptor
            .warnings
            .iter()
            .any(|warning| warning.contains("检测到多个可导入文件指向同一目标表")));

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn manifest_auxiliary_auto_stage_true_is_manual_only_for_now() {
        let (data_dir, _database) = temp_database("folder-auxiliary-manual");
        let package_dir = data_dir.join("folder_package");
        write_folder_package(
            &package_dir,
            Some(&folder_manifest_with_auxiliary(true)),
            "json/knowledge_items_import.json",
        );
        write_auxiliary_herb_file(&package_dir);

        let descriptor = preview_package_folder(package_dir.to_str().unwrap())
            .expect("preview package folder with manual auxiliary");
        assert_eq!(
            descriptor.auto_stage_files,
            vec!["json/knowledge_items_import.json".to_string()]
        );
        let auxiliary = descriptor
            .auxiliary_files
            .iter()
            .find(|file| file.path == "json/herb_items_import.json")
            .expect("auxiliary file");
        assert!(auxiliary.auto_stage);
        assert!(auxiliary
            .skip_reason
            .as_deref()
            .unwrap_or_default()
            .contains("可手动选择"));

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn package_folder_with_only_pdf_refuses_direct_import() {
        let (data_dir, _database) = temp_database("folder-pdf-only");
        let package_dir = data_dir.join("folder_package");
        fs::create_dir_all(&package_dir).expect("create package dir");
        fs::write(package_dir.join("3人纪-神农本草经.pdf"), b"%PDF-1.7")
            .expect("write pdf placeholder");

        let error = preview_package_folder(package_dir.to_str().unwrap())
            .expect_err("pdf folder should fail")
            .to_string();
        assert!(error.contains("PDF 原始资料不能直接导入"));
        assert!(error.contains("标准 import_manifest 数据包"));

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn package_folder_import_rebuilds_search_index() {
        let (data_dir, database) = temp_database("folder-import-search");
        let package_dir = data_dir.join("folder_package");
        write_folder_package(
            &package_dir,
            Some(&folder_manifest("json/knowledge_items_import.json")),
            "json/knowledge_items_import.json",
        );

        let summary = import_package_folder(&database, package_dir.to_str().unwrap())
            .expect("import folder package");
        assert_eq!(summary.total_rows, 1);
        let batch_id = summary.batch.id.unwrap();
        confirm_import(&database, batch_id).expect("confirm folder import");

        let response = search_index_service::search(
            &database,
            SearchRequest {
                query: "文件夹桂枝汤".to_string(),
                item_type: None,
                page: Some(1),
                page_size: Some(10),
            },
        )
        .expect("search folder import");
        assert!(!response.results.is_empty());

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn rollback_import_batch_removes_items_and_rebuilds_search() {
        let (data_dir, database) = temp_database("rollback");
        let content = r#"
        [
          {
            "type": "formula",
            "code": "ROLLBACK-GZT-001",
            "name": "回滚测试桂枝汤",
            "category": "原典/伤寒论",
            "content": "桂枝汤原文用于回滚测试。",
            "source_note": "伤寒论",
            "tags": ["方剂", "回滚测试"]
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
        .expect("import rollback fixture");
        let batch_id = summary.batch.id.unwrap();
        confirm_import(&database, batch_id).expect("confirm rollback fixture");

        let before = search_index_service::search(
            &database,
            SearchRequest {
                query: "回滚测试桂枝汤".to_string(),
                item_type: None,
                page: Some(1),
                page_size: Some(10),
            },
        )
        .expect("search before rollback");
        assert_eq!(before.total, 1);

        let rollback = rollback_import_batch(&database, batch_id).expect("rollback import");
        assert_eq!(rollback.deleted_items, 1);

        let after = search_index_service::search(
            &database,
            SearchRequest {
                query: "回滚测试桂枝汤".to_string(),
                item_type: None,
                page: Some(1),
                page_size: Some(10),
            },
        )
        .expect("search after rollback");
        assert_eq!(after.total, 0);

        database
            .with_connection(|connection| {
                let batch_status: String = connection.query_row(
                    "SELECT status FROM data_import_batches WHERE id = ?1",
                    [batch_id],
                    |row| row.get(0),
                )?;
                assert_eq!(batch_status, "rolled_back");

                let item_count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM knowledge_items WHERE code = 'ROLLBACK-GZT-001'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(item_count, 0);
                Ok(())
            })
            .expect("inspect rollback state");

        let _ = fs::remove_dir_all(data_dir);
    }
}
