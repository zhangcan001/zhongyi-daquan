use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::data_pipeline::{ExecuteImportPlanResult, ImportPlan, ImportPlanAction};
use crate::repositories::search_repository;
use crate::services::{
    ai_import_assist_service, import_engine_service, import_project_service, normalize_service,
    search_index_service,
};
use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde_json::{Map, Value};

pub fn preview_import_plan(database: &Database, package_path: &str) -> AppResult<ImportPlan> {
    let parsed = import_project_service::parse_path_import_package(package_path)?;
    let descriptor = parsed.descriptor;
    let primary_file = descriptor
        .primary_files
        .first()
        .cloned()
        .unwrap_or_else(|| "smart_import_package".to_string());
    let intent = infer_import_intent(
        &descriptor.import_intent,
        descriptor.import_profile.as_deref(),
        descriptor.detected_type.as_str(),
        descriptor.package_name.as_deref(),
    );
    let duplicate_policy = descriptor
        .duplicate_policy
        .clone()
        .unwrap_or_else(|| default_duplicate_policy(&intent).to_string());
    let engine = import_engine_service::prepare_import_rows(
        &primary_file,
        &parsed.import_type,
        "mixed",
        &parsed.rows,
        None,
    );
    if !engine.direct_import_ready
        && matches!(
            engine.detection.detected_type.as_str(),
            "generic_csv" | "generic_json" | "unknown"
        )
    {
        return Ok(generic_mapping_plan(
            package_path,
            descriptor.package_name,
            engine.detection.record_count,
        ));
    }
    let drafts = normalize_service::normalize_rows_batch(database, engine.mapped_rows)?;
    let ai = ai_import_assist_service::request_assist("duplicate_resolution")?;
    let mut warnings = descriptor.warnings.clone();
    warnings.extend(engine.warnings);
    warnings.push(ai.message.clone());
    let actions = drafts
        .iter()
        .enumerate()
        .map(|(index, draft)| {
            plan_action_for_draft(
                database,
                &intent,
                &duplicate_policy,
                index as i64 + 1,
                draft,
            )
        })
        .collect::<AppResult<Vec<_>>>()?;
    Ok(build_plan(
        package_path,
        descriptor.package_name,
        intent,
        duplicate_policy,
        warnings,
        actions,
        Some(ai.message),
    ))
}

pub fn execute_import_plan(
    database: &Database,
    plan: ImportPlan,
) -> AppResult<ExecuteImportPlanResult> {
    let mut created_count = 0;
    let mut merged_count = 0;
    let mut attached_annotation_count = 0;
    let mut skipped_count = 0;
    let mut needs_review_count = 0;
    let mut rejected_count = 0;
    let mut warnings = Vec::new();

    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        for action in &plan.actions {
            let draft = action.draft_json.as_object().cloned().unwrap_or_default();
            match action.action_type.as_str() {
                "create_new" => {
                    insert_knowledge_item_tx(&transaction, &draft)?;
                    created_count += 1;
                }
                "merge_empty_fields" => {
                    if let Some(item_id) = action.existing_item_id {
                        merge_empty_fields_tx(&transaction, item_id, &draft)?;
                        merged_count += 1;
                    }
                }
                "attach_annotation" => {
                    if let Some(item_id) = action.existing_item_id {
                        insert_annotation_tx(&transaction, item_id, &draft)?;
                        attached_annotation_count += 1;
                    }
                }
                "skip_duplicate" => skipped_count += 1,
                "needs_review" => needs_review_count += 1,
                "reject_invalid" => rejected_count += 1,
                other => warnings.push(format!("未知导入计划动作，已跳过: {other}")),
            }
        }
        transaction.commit()?;
        Ok(())
    })?;

    search_index_service::rebuild_search_index(database)?;
    Ok(ExecuteImportPlanResult {
        plan_id: plan.plan_id,
        created_count,
        merged_count,
        attached_annotation_count,
        skipped_count,
        needs_review_count,
        rejected_count,
        search_index_rebuilt: true,
        warnings,
    })
}

fn generic_mapping_plan(
    package_path: &str,
    package_name: Option<String>,
    total_records: i64,
) -> ImportPlan {
    build_plan(
        package_path,
        package_name,
        "primary_seed".to_string(),
        "ask_on_conflict".to_string(),
        vec!["generic_csv / unknown 数据仍进入字段映射确认流程。".to_string()],
        vec![ImportPlanAction {
            row_index: 0,
            action_type: "needs_review".to_string(),
            item_type: None,
            name: None,
            existing_item_id: None,
            confidence: 0.0,
            reason: "需要先完成字段映射。".to_string(),
            draft_json: Value::Null,
        }],
        None,
    )
    .with_total_records(total_records)
}

trait WithTotalRecords {
    fn with_total_records(self, total_records: i64) -> Self;
}

impl WithTotalRecords for ImportPlan {
    fn with_total_records(mut self, total_records: i64) -> Self {
        self.total_records = total_records;
        self
    }
}

fn build_plan(
    package_path: &str,
    package_name: Option<String>,
    import_intent: String,
    duplicate_policy: String,
    warnings: Vec<String>,
    actions: Vec<ImportPlanAction>,
    ai_message: Option<String>,
) -> ImportPlan {
    let count = |kind: &str| {
        actions
            .iter()
            .filter(|action| action.action_type == kind)
            .count() as i64
    };
    ImportPlan {
        plan_id: format!("smart-import-{}", Utc::now().timestamp_millis()),
        package_path: package_path.to_string(),
        package_name,
        import_intent,
        duplicate_policy,
        total_records: actions.len() as i64,
        create_count: count("create_new"),
        update_count: count("merge_empty_fields"),
        attach_annotation_count: count("attach_annotation"),
        skip_duplicate_count: count("skip_duplicate"),
        needs_review_count: count("needs_review"),
        reject_invalid_count: count("reject_invalid"),
        warnings,
        actions,
        ai_message,
    }
}

fn plan_action_for_draft(
    database: &Database,
    intent: &str,
    duplicate_policy: &str,
    row_index: i64,
    draft: &Map<String, Value>,
) -> AppResult<ImportPlanAction> {
    let item_type = text(draft, "type").unwrap_or_else(|| "unknown".to_string());
    let name = text(draft, "name");
    if name.is_none() {
        return Ok(action(
            row_index,
            "reject_invalid",
            None,
            None,
            None,
            0.0,
            "缺少 name，不能自动导入。",
            draft,
        ));
    }
    let existing = find_existing_item(
        database,
        &item_type,
        name.as_deref().unwrap(),
        text(draft, "source_note").as_deref(),
    )?;
    let Some(existing) = existing else {
        return Ok(action(
            row_index,
            "create_new",
            Some(item_type),
            name,
            None,
            0.92,
            "未发现同名同类条目。",
            draft,
        ));
    };
    let similar = content_similar(
        existing.content.as_deref(),
        text(draft, "content").as_deref(),
    );
    match intent {
        "annotation_enrichment" => {
            if similar {
                Ok(action(
                    row_index,
                    "skip_duplicate",
                    Some(item_type),
                    name,
                    Some(existing.id),
                    0.95,
                    "同名条目内容高度相似，跳过重复资料。",
                    draft,
                ))
            } else if item_type == "herb" || item_type == "中药" {
                Ok(action(
                    row_index,
                    "attach_annotation",
                    Some(item_type),
                    name,
                    Some(existing.id),
                    0.9,
                    "同名中药已存在，作为注解资料附加到主条目。",
                    draft,
                ))
            } else {
                Ok(action(
                    row_index,
                    "create_new",
                    Some(item_type),
                    name,
                    None,
                    0.7,
                    "未匹配到中药注解规则，创建新条目。",
                    draft,
                ))
            }
        }
        "classic_text" => {
            if similar
                || source_note_matches(
                    existing.source_note.as_deref(),
                    text(draft, "source_note").as_deref(),
                )
            {
                Ok(action(
                    row_index,
                    "skip_duplicate",
                    Some(item_type),
                    name,
                    Some(existing.id),
                    0.95,
                    "同 source_note 或内容已存在，跳过重复条文。",
                    draft,
                ))
            } else {
                Ok(action(
                    row_index,
                    "create_new",
                    Some(item_type),
                    name,
                    None,
                    0.9,
                    "经典条文未重复，创建新条目。",
                    draft,
                ))
            }
        }
        "search_terms" => Ok(action(
            row_index,
            "skip_duplicate",
            Some(item_type),
            name,
            Some(existing.id),
            0.8,
            "搜索词导入由 search_terms 去重逻辑处理。",
            draft,
        )),
        _ => {
            if similar {
                Ok(action(
                    row_index,
                    "skip_duplicate",
                    Some(item_type),
                    name,
                    Some(existing.id),
                    0.95,
                    "同名同类条目内容高度相似，跳过重复。",
                    draft,
                ))
            } else if has_empty_field_to_merge(&existing, draft)
                && duplicate_policy != "ask_on_conflict"
            {
                Ok(action(
                    row_index,
                    "merge_empty_fields",
                    Some(item_type),
                    name,
                    Some(existing.id),
                    0.82,
                    "现有条目存在空字段，计划只补空字段，不覆盖已有内容。",
                    draft,
                ))
            } else {
                Ok(action(
                    row_index,
                    "needs_review",
                    Some(item_type),
                    name,
                    Some(existing.id),
                    0.55,
                    "发现同名同类条目但内容存在差异，需要人工确认。",
                    draft,
                ))
            }
        }
    }
}

fn action(
    row_index: i64,
    action_type: &str,
    item_type: Option<String>,
    name: Option<String>,
    existing_item_id: Option<i64>,
    confidence: f64,
    reason: &str,
    draft: &Map<String, Value>,
) -> ImportPlanAction {
    ImportPlanAction {
        row_index,
        action_type: action_type.to_string(),
        item_type,
        name,
        existing_item_id,
        confidence,
        reason: reason.to_string(),
        draft_json: Value::Object(draft.clone()),
    }
}

#[derive(Debug)]
struct ExistingItem {
    id: i64,
    summary: Option<String>,
    content: Option<String>,
    source_note: Option<String>,
    tags: Option<String>,
}

fn find_existing_item(
    database: &Database,
    item_type: &str,
    name: &str,
    source_note: Option<&str>,
) -> AppResult<Option<ExistingItem>> {
    database.with_connection(|connection| {
        let by_source = if let Some(source_note) = source_note {
            connection
                .query_row(
                    "SELECT id, type, name, summary, content, source_note, tags
                 FROM knowledge_items
                 WHERE type = ?1 AND name = ?2 AND COALESCE(source_note, '') = ?3
                 LIMIT 1",
                    params![item_type, name, source_note],
                    map_existing_item,
                )
                .optional()?
        } else {
            None
        };
        if by_source.is_some() {
            return Ok(by_source);
        }
        connection
            .query_row(
                "SELECT id, type, name, summary, content, source_note, tags
             FROM knowledge_items
             WHERE type = ?1 AND name = ?2
             LIMIT 1",
                params![item_type, name],
                map_existing_item,
            )
            .optional()
            .map_err(Into::into)
    })
}

fn map_existing_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExistingItem> {
    Ok(ExistingItem {
        id: row.get(0)?,
        summary: row.get(3)?,
        content: row.get(4)?,
        source_note: row.get(5)?,
        tags: row.get(6)?,
    })
}

fn insert_knowledge_item_tx(
    transaction: &rusqlite::Transaction<'_>,
    draft: &Map<String, Value>,
) -> AppResult<i64> {
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "INSERT INTO knowledge_items
         (type, code, name, alias, pinyin, category, summary, content, source_note, tags,
          data_status, completeness_status, content_version, is_favorite, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 'validated', 'partial', 1, 0, ?11, ?12)",
        params![
            text(draft, "type").unwrap_or_else(|| "unknown".to_string()),
            text(draft, "code"),
            text(draft, "name").unwrap_or_default(),
            text(draft, "alias"),
            text(draft, "pinyin"),
            text(draft, "category"),
            text(draft, "summary"),
            text(draft, "content"),
            text(draft, "source_note"),
            text(draft, "tags"),
            now,
            now
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

fn merge_empty_fields_tx(
    transaction: &rusqlite::Transaction<'_>,
    item_id: i64,
    draft: &Map<String, Value>,
) -> AppResult<()> {
    transaction.execute(
        "UPDATE knowledge_items
         SET summary = CASE WHEN COALESCE(summary, '') = '' THEN ?2 ELSE summary END,
             content = CASE WHEN COALESCE(content, '') = '' THEN ?3 ELSE content END,
             source_note = CASE WHEN COALESCE(source_note, '') = '' THEN ?4 ELSE source_note END,
             tags = CASE WHEN COALESCE(tags, '') = '' THEN ?5 ELSE tags END,
             updated_at = ?6
         WHERE id = ?1",
        params![
            item_id,
            text(draft, "summary"),
            text(draft, "content"),
            text(draft, "source_note"),
            text(draft, "tags"),
            Utc::now().to_rfc3339()
        ],
    )?;
    Ok(())
}

fn insert_annotation_tx(
    transaction: &rusqlite::Transaction<'_>,
    item_id: i64,
    draft: &Map<String, Value>,
) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    transaction.execute(
        "INSERT INTO knowledge_annotations
         (knowledge_item_id, annotation_type, source_title, source_note, content, detail_json, tags_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            item_id,
            "source_annotation",
            text(draft, "name"),
            text(draft, "source_note"),
            text(draft, "content"),
            serde_json::to_string(draft)?,
            text(draft, "tags").map(|tags| serde_json::to_string(&tags)).transpose()?,
            now,
            now
        ],
    )?;
    Ok(())
}

fn has_empty_field_to_merge(existing: &ExistingItem, draft: &Map<String, Value>) -> bool {
    (existing.summary.as_deref().unwrap_or_default().is_empty() && text(draft, "summary").is_some())
        || (existing.content.as_deref().unwrap_or_default().is_empty()
            && text(draft, "content").is_some())
        || (existing
            .source_note
            .as_deref()
            .unwrap_or_default()
            .is_empty()
            && text(draft, "source_note").is_some())
        || (existing.tags.as_deref().unwrap_or_default().is_empty()
            && text(draft, "tags").is_some())
}

fn content_similar(left: Option<&str>, right: Option<&str>) -> bool {
    let left = search_repository::normalize_for_search(left.unwrap_or_default());
    let right = search_repository::normalize_for_search(right.unwrap_or_default());
    if left.is_empty() || right.is_empty() {
        return false;
    }
    left == right || left.contains(&right) || right.contains(&left)
}

fn source_note_matches(left: Option<&str>, right: Option<&str>) -> bool {
    !left.unwrap_or_default().trim().is_empty() && left == right
}

fn infer_import_intent(
    explicit: &Option<String>,
    profile: Option<&str>,
    detected_type: &str,
    package_name: Option<&str>,
) -> String {
    if let Some(explicit) = explicit {
        return explicit.clone();
    }
    if matches!(profile, Some("classics_curated_v1")) {
        return "classic_text".to_string();
    }
    if matches!(profile, Some("pdf_herb_notes_private_v1"))
        || package_name.unwrap_or_default().contains("ni_notes")
    {
        return "annotation_enrichment".to_string();
    }
    if detected_type == "search_terms_v1" {
        return "search_terms".to_string();
    }
    "primary_seed".to_string()
}

fn default_duplicate_policy(intent: &str) -> &'static str {
    match intent {
        "annotation_enrichment" => "attach_annotation",
        "classic_text" | "search_terms" => "skip_existing",
        _ => "auto",
    }
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

#[cfg(test)]
fn annotation_count(database: &Database) -> AppResult<i64> {
    database.with_connection(|connection| {
        connection
            .query_row("SELECT COUNT(1) FROM knowledge_annotations", [], |row| {
                row.get(0)
            })
            .map_err(Into::into)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;
    use crate::models::search::SearchRequest;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_database(test_name: &str) -> (std::path::PathBuf, Database) {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let data_dir =
            std::env::temp_dir().join(format!("zhongyi-smart-import-{test_name}-{unique}"));
        let database = Database::initialize(&data_dir).expect("initialize database");
        (data_dir, database)
    }

    fn write_package(root: &std::path::Path, intent: &str, rows: &str) {
        fs::create_dir_all(root.join("json")).unwrap();
        fs::write(root.join("json/knowledge_items_import.json"), rows).unwrap();
        fs::write(
            root.join("import_manifest.json"),
            format!(
                r#"{{
            "package_name": "smart_test_package",
            "import_profile": "pdf_herb_notes_private_v1",
            "import_intent": "{intent}",
            "duplicate_policy": "auto",
            "ai_assist": true,
            "files": [{{
              "path": "json/knowledge_items_import.json",
              "type": "knowledge_items_v1",
              "target": "knowledge_items",
              "primary": true,
              "auto_stage": true
            }}],
            "import_order": ["knowledge_items"]
        }}"#
            ),
        )
        .unwrap();
    }

    fn seed_herb(database: &Database, name: &str, content: &str, source_note: &str) -> i64 {
        database.with_connection(|connection| {
            connection.execute(
                "INSERT INTO knowledge_items
                 (type, code, name, content, source_note, data_status, completeness_status, created_at, updated_at)
                 VALUES ('herb', ?1, ?2, ?3, ?4, 'validated', 'partial', datetime('now'), datetime('now'))",
                params![format!("SEED-{name}"), name, content, source_note],
            )?;
            Ok(connection.last_insert_rowid())
        }).unwrap()
    }

    #[test]
    fn import_intent_is_inferred_from_legacy_profile() {
        assert_eq!(
            infer_import_intent(
                &None,
                Some("classics_curated_v1"),
                "knowledge_items_v1",
                None
            ),
            "classic_text"
        );
        assert_eq!(
            infer_import_intent(
                &None,
                Some("pdf_herb_notes_private_v1"),
                "knowledge_items_v1",
                None
            ),
            "annotation_enrichment"
        );
        assert_eq!(
            infer_import_intent(&None, None, "knowledge_items_v1", None),
            "primary_seed"
        );
    }

    #[test]
    fn annotation_enrichment_attaches_to_existing_herb() {
        let (data_dir, database) = temp_database("annotation");
        seed_herb(&database, "甘草", "甘草，味甘。", "seed");
        let package = data_dir.join("package");
        write_package(
            &package,
            "annotation_enrichment",
            r#"[{
            "type": "herb",
            "name": "甘草",
            "content": "倪注：甘草可调和诸药。",
            "source_note": "倪注"
        }]"#,
        );
        let plan = preview_import_plan(&database, package.to_str().unwrap()).unwrap();
        assert_eq!(plan.import_intent, "annotation_enrichment");
        assert_eq!(plan.attach_annotation_count, 1);
        let result = execute_import_plan(&database, plan).unwrap();
        assert_eq!(result.attached_annotation_count, 1);
        assert_eq!(annotation_count(&database).unwrap(), 1);
        search_index_service::rebuild_search_index(&database).unwrap();
        let response = search_index_service::search(
            &database,
            SearchRequest {
                query: "倪注".to_string(),
                item_type: None,
                page: Some(1),
                page_size: Some(10),
            },
        )
        .unwrap();
        assert!(!response.results.is_empty());
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn primary_seed_duplicate_does_not_overwrite_conflicts() {
        let (data_dir, database) = temp_database("primary-duplicate");
        seed_herb(&database, "人参", "已有内容", "seed");
        let package = data_dir.join("package");
        write_package(
            &package,
            "primary_seed",
            r#"[{
            "type": "herb",
            "name": "人参",
            "content": "冲突内容",
            "source_note": "new"
        }]"#,
        );
        let plan = preview_import_plan(&database, package.to_str().unwrap()).unwrap();
        assert_eq!(plan.needs_review_count, 1);
        let result = execute_import_plan(&database, plan).unwrap();
        assert_eq!(result.needs_review_count, 1);
        let content: String = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT content FROM knowledge_items WHERE name = '人参'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(Into::into)
            })
            .unwrap();
        assert_eq!(content, "已有内容");
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn classic_text_duplicate_source_is_skipped() {
        let (data_dir, database) = temp_database("classic-skip");
        seed_herb(&database, "神农本草经 甘草", "旧条文", "神农本草经");
        let package = data_dir.join("package");
        write_package(
            &package,
            "classic_text",
            r#"[{
            "type": "herb",
            "name": "神农本草经 甘草",
            "content": "新条文",
            "source_note": "神农本草经"
        }]"#,
        );
        let plan = preview_import_plan(&database, package.to_str().unwrap()).unwrap();
        assert_eq!(plan.skip_duplicate_count, 1);
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn ai_disabled_message_does_not_block_plan() {
        let (data_dir, database) = temp_database("ai-message");
        let package = data_dir.join("package");
        write_package(
            &package,
            "primary_seed",
            r#"[{"type":"herb","name":"黄耆","content":"黄耆测试"}]"#,
        );
        let plan = preview_import_plan(&database, package.to_str().unwrap()).unwrap();
        assert!(plan
            .ai_message
            .unwrap_or_default()
            .contains("AI 导入辅助当前未启用"));
        assert_eq!(plan.create_count, 1);
        let _ = fs::remove_dir_all(data_dir);
    }
}
