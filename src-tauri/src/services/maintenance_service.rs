use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::runtime::{CleanOldPerformanceLogsRequest, MaintenanceReport};
use crate::repositories::{audit_repository, performance_repository};
use crate::services::{background_job_service, search_index_service};
use chrono::Utc;
use std::fs;
use std::path::Path;

pub fn rebuild_search_index_job(database: &Database) -> AppResult<MaintenanceReport> {
    let job = background_job_service::create_internal_job(database, "rebuild_search_index", None)?;
    let result = (|| {
        background_job_service::set_progress(database, job.id, 10.0)?;
        let response = search_index_service::rebuild_search_index(database)?;
        background_job_service::set_progress(database, job.id, 90.0)?;
        let result_json = serde_json::to_string(&response)?;
        let completed_job =
            background_job_service::success_with_json(database, job.id, &result_json)?;
        Ok(MaintenanceReport {
            job: completed_job,
            action: "rebuild_search_index".to_string(),
            message: format!(
                "搜索索引已重建：{} 条索引，{} 个搜索词。",
                response.indexed_items, response.search_terms
            ),
            affected_rows: Some(response.indexed_items),
            output_path: None,
        })
    })();
    finish_or_fail(database, job.id, result)
}

pub fn optimize_database(database: &Database) -> AppResult<MaintenanceReport> {
    let job = background_job_service::create_internal_job(
        database,
        "clean_batch",
        Some("{\"action\":\"optimize_database\"}"),
    )?;
    let result = (|| {
        background_job_service::set_progress(database, job.id, 20.0)?;
        database.with_connection(|connection| {
            connection.execute_batch("PRAGMA optimize; VACUUM;")?;
            Ok(())
        })?;
        background_job_service::set_progress(database, job.id, 90.0)?;
        audit_repository::record(
            database,
            "optimize_database",
            Some("database"),
            None,
            None,
            None,
        )?;
        let completed_job = background_job_service::success_with_json(
            database,
            job.id,
            "{\"action\":\"optimize_database\",\"ok\":true}",
        )?;
        Ok(MaintenanceReport {
            job: completed_job,
            action: "optimize_database".to_string(),
            message: "数据库已执行 PRAGMA optimize 和 VACUUM。".to_string(),
            affected_rows: None,
            output_path: None,
        })
    })();
    finish_or_fail(database, job.id, result)
}

pub fn clean_temp_imports(database: &Database, data_dir: &Path) -> AppResult<MaintenanceReport> {
    let job = background_job_service::create_internal_job(
        database,
        "clean_batch",
        Some("{\"action\":\"clean_temp_imports\"}"),
    )?;
    let result = (|| {
        background_job_service::set_progress(database, job.id, 15.0)?;
        let mut removed = 0_i64;
        removed += clean_dir_contents(&data_dir.join("temp"))?;
        removed += clean_tmp_files(&data_dir.join("imports"))?;
        background_job_service::set_progress(database, job.id, 80.0)?;
        audit_repository::record(
            database,
            "clean_temp_imports",
            Some("filesystem"),
            None,
            None,
            None,
        )?;
        let completed_job = background_job_service::success_with_json(
            database,
            job.id,
            &serde_json::json!({ "removed": removed }).to_string(),
        )?;
        Ok(MaintenanceReport {
            job: completed_job,
            action: "clean_temp_imports".to_string(),
            message: format!("已清理 {removed} 个临时导入文件或目录。"),
            affected_rows: Some(removed),
            output_path: None,
        })
    })();
    finish_or_fail(database, job.id, result)
}

pub fn clean_old_performance_logs(
    database: &Database,
    request: CleanOldPerformanceLogsRequest,
) -> AppResult<MaintenanceReport> {
    let keep_days = request.keep_days.unwrap_or(30).clamp(1, 3650);
    let job = background_job_service::create_internal_job(
        database,
        "clean_batch",
        Some(
            &serde_json::json!({ "action": "clean_old_performance_logs", "keepDays": keep_days })
                .to_string(),
        ),
    )?;
    let result = (|| {
        background_job_service::set_progress(database, job.id, 30.0)?;
        let removed = performance_repository::delete_older_than_days(database, keep_days)?;
        background_job_service::set_progress(database, job.id, 90.0)?;
        audit_repository::record(
            database,
            "clean_old_performance_logs",
            Some("performance_logs"),
            None,
            None,
            Some(&serde_json::json!({ "removed": removed, "keepDays": keep_days }).to_string()),
        )?;
        let completed_job = background_job_service::success_with_json(
            database,
            job.id,
            &serde_json::json!({ "removed": removed, "keepDays": keep_days }).to_string(),
        )?;
        Ok(MaintenanceReport {
            job: completed_job,
            action: "clean_old_performance_logs".to_string(),
            message: format!("已清理 {removed} 条 {keep_days} 天以前的性能日志。"),
            affected_rows: Some(removed),
            output_path: None,
        })
    })();
    finish_or_fail(database, job.id, result)
}

pub fn export_performance_report(
    database: &Database,
    data_dir: &Path,
) -> AppResult<MaintenanceReport> {
    let job = background_job_service::create_internal_job(
        database,
        "clean_batch",
        Some("{\"action\":\"export_performance_report\"}"),
    )?;
    let result = (|| {
        background_job_service::set_progress(database, job.id, 20.0)?;
        let logs = performance_repository::list_all_for_report(database, 10_000)?;
        let export_dir = data_dir.join("exports");
        fs::create_dir_all(&export_dir)?;
        let output_path = export_dir.join(format!(
            "performance_report_{}.json",
            Utc::now().format("%Y%m%d%H%M%S")
        ));
        let report = serde_json::json!({
            "generatedAt": Utc::now().to_rfc3339(),
            "total": logs.len(),
            "logs": logs
        });
        fs::write(&output_path, serde_json::to_string_pretty(&report)?)?;
        background_job_service::set_progress(database, job.id, 85.0)?;
        audit_repository::record(
            database,
            "export_performance_report",
            Some("performance_logs"),
            None,
            None,
            Some(
                &serde_json::json!({ "outputPath": output_path.display().to_string() }).to_string(),
            ),
        )?;
        let completed_job = background_job_service::success_with_json(
            database,
            job.id,
            &serde_json::json!({ "outputPath": output_path.display().to_string(), "total": logs.len() }).to_string(),
        )?;
        Ok(MaintenanceReport {
            job: completed_job,
            action: "export_performance_report".to_string(),
            message: format!("已导出 {} 条性能日志。", logs.len()),
            affected_rows: Some(logs.len() as i64),
            output_path: Some(output_path.display().to_string()),
        })
    })();
    finish_or_fail(database, job.id, result)
}

fn finish_or_fail(
    database: &Database,
    job_id: i64,
    result: AppResult<MaintenanceReport>,
) -> AppResult<MaintenanceReport> {
    match result {
        Ok(report) => Ok(report),
        Err(err) => {
            let _ = background_job_service::fail_with_message(database, job_id, &err.to_string());
            Err(err)
        }
    }
}

fn clean_dir_contents(path: &Path) -> AppResult<i64> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_dir() {
            fs::remove_dir_all(&entry_path)?;
        } else {
            fs::remove_file(&entry_path)?;
        }
        removed += 1;
    }
    Ok(removed)
}

fn clean_tmp_files(path: &Path) -> AppResult<i64> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        return Ok(0);
    }
    let mut removed = 0;
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        if entry_path.is_file()
            && entry_path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tmp"))
        {
            fs::remove_file(entry_path)?;
            removed += 1;
        }
    }
    Ok(removed)
}

pub fn check_data_integrity(database: &Database) -> AppResult<MaintenanceReport> {
    let job = background_job_service::create_internal_job(database, "data_integrity_check", None)?;

    let result = (|| {
        background_job_service::set_progress(database, job.id, 10.0)?;

        let mut issues = Vec::new();

        // 检查孤立的 detail 记录
        let orphaned_details = database.with_connection(|connection| {
            let mut count = 0;

            for table in &[
                "herb_details",
                "formula_details",
                "meridian_details",
                "acupoint_details",
                "syndrome_details",
                "disease_details",
            ] {
                let sql = format!(
                    "SELECT COUNT(*) FROM {} WHERE item_id NOT IN (SELECT id FROM knowledge_items)",
                    table
                );
                let table_count: i64 = connection.query_row(&sql, [], |row| row.get(0))?;
                count += table_count;

                if table_count > 0 {
                    issues.push(format!("{} 表有 {} 条孤立记录", table, table_count));
                }
            }

            Ok::<i64, crate::errors::AppError>(count)
        })?;

        background_job_service::set_progress(database, job.id, 40.0)?;

        // 检查缺失必填字段的记录
        let missing_required = database.with_connection(|connection| {
            let count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM knowledge_items WHERE name IS NULL OR TRIM(name) = ''",
                [],
                |row| row.get(0),
            )?;

            if count > 0 {
                issues.push(format!("{} 条记录缺少名称", count));
            }

            Ok::<i64, crate::errors::AppError>(count)
        })?;

        background_job_service::set_progress(database, job.id, 60.0)?;

        // 检查无效的外键引用
        let invalid_fk = database.with_connection(|connection| {
            let count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM acupoint_details
                 WHERE meridian_item_id IS NOT NULL
                 AND meridian_item_id NOT IN (SELECT id FROM knowledge_items WHERE type IN ('经络', 'meridian'))",
                [],
                |row| row.get(0),
            )?;

            if count > 0 {
                issues.push(format!("{} 条穴位记录引用了不存在的经络", count));
            }

            Ok::<i64, crate::errors::AppError>(count)
        })?;

        background_job_service::set_progress(database, job.id, 80.0)?;

        // 检查重复的 code
        let duplicate_codes = database.with_connection(|connection| {
            let count: i64 = connection.query_row(
                "SELECT COUNT(*) FROM (
                    SELECT code FROM knowledge_items
                    WHERE code IS NOT NULL AND TRIM(code) != ''
                    GROUP BY type, code
                    HAVING COUNT(*) > 1
                )",
                [],
                |row| row.get(0),
            )?;

            if count > 0 {
                issues.push(format!("{} 个编号存在重复", count));
            }

            Ok::<i64, crate::errors::AppError>(count)
        })?;

        let total_issues = orphaned_details + missing_required + invalid_fk + duplicate_codes;

        let report = serde_json::json!({
            "orphaned_details": orphaned_details,
            "missing_required": missing_required,
            "invalid_foreign_keys": invalid_fk,
            "duplicate_codes": duplicate_codes,
            "total_issues": total_issues,
            "issues": issues,
        });

        let completed_job =
            background_job_service::success_with_json(database, job.id, &report.to_string())?;

        let message = if total_issues == 0 {
            "数据完整性检查通过，未发现问题。".to_string()
        } else {
            format!(
                "发现 {} 个数据完整性问题：{}",
                total_issues,
                issues.join("; ")
            )
        };

        Ok(MaintenanceReport {
            job: completed_job,
            action: "data_integrity_check".to_string(),
            message,
            affected_rows: Some(total_issues),
            output_path: None,
        })
    })();

    finish_or_fail(database, job.id, result)
}

pub fn clear_database_content(database: &Database) -> AppResult<MaintenanceReport> {
    let job = background_job_service::create_internal_job(
        database,
        "clear_database_content",
        Some("{\"action\":\"clear_database_content\"}"),
    )?;
    let result = (|| {
        background_job_service::set_progress(database, job.id, 10.0)?;
        let deleted_rows = database.with_connection(|connection| {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch("PRAGMA foreign_keys = ON;")?;
            let mut deleted = 0_i64;

            for table in [
                "knowledge_fts",
                "knowledge_list_view_cache",
                "relation_count_cache",
                "search_terms",
                "knowledge_fingerprints",
                "duplicate_candidates",
                "merge_records",
                "relation_suggestions",
                "knowledge_relations",
                "knowledge_versions",
                "knowledge_annotations",
                "herb_details",
                "formula_details",
                "meridian_details",
                "acupoint_details",
                "syndrome_details",
                "disease_details",
                "knowledge_items",
                "data_transform_row_changes",
                "data_transform_steps",
                "data_validation_issues",
                "data_import_rows",
                "data_import_batches",
                "import_run_changes",
                "import_reports",
                "import_runs",
                "ai_drafts",
                "ai_tasks",
                "ai_call_logs",
                "performance_logs",
                "error_logs",
            ] {
                deleted += transaction.execute(&format!("DELETE FROM {table}"), [])? as i64;
            }

            transaction.execute(
                "DELETE FROM background_jobs WHERE id != ?1",
                [job.id],
            )?;
            transaction.execute(
                "DELETE FROM audit_logs WHERE action != 'clear_database_content'",
                [],
            )?;
            transaction.execute(
                "DELETE FROM sqlite_sequence
                 WHERE name NOT IN ('schema_migrations', 'validation_rules', 'field_mapping_templates',
                                    'standard_terms', 'ai_provider_settings', 'ai_prompt_templates',
                                    'background_jobs', 'audit_logs')",
                [],
            )?;
            transaction.commit()?;
            Ok::<i64, crate::errors::AppError>(deleted)
        })?;

        background_job_service::set_progress(database, job.id, 80.0)?;
        let index = search_index_service::rebuild_search_index(database)?;
        let report_json = serde_json::json!({
            "action": "clear_database_content",
            "deletedRows": deleted_rows,
            "indexedItemsAfterClear": index.indexed_items,
            "searchTermsAfterClear": index.search_terms
        });
        audit_repository::record(
            database,
            "clear_database_content",
            Some("database"),
            None,
            None,
            Some(&report_json.to_string()),
        )?;
        let completed_job =
            background_job_service::success_with_json(database, job.id, &report_json.to_string())?;
        Ok(MaintenanceReport {
            job: completed_job,
            action: "clear_database_content".to_string(),
            message: format!(
                "数据库内容已清空：删除 {deleted_rows} 条业务数据，搜索索引已重置。"
            ),
            affected_rows: Some(deleted_rows),
            output_path: None,
        })
    })();
    finish_or_fail(database, job.id, result)
}

#[cfg(test)]
mod clear_database_tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn clear_database_content_removes_knowledge_rows() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("zhongyi-clear-db-{unique}"));
        let database = Database::initialize(&data_dir).expect("database initializes");

        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, code, name, content, data_status, completeness_status, created_at, updated_at)
                     VALUES ('herb', 'CLEAR-001', '清空测试', '测试内容', 'imported', 'partial', datetime('now'), datetime('now'))",
                    [],
                )?;
                Ok::<(), crate::errors::AppError>(())
            })
            .expect("seed item");

        let report = clear_database_content(&database).expect("clear succeeds");
        assert_eq!(report.action, "clear_database_content");

        let count: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row("SELECT COUNT(1) FROM knowledge_items", [], |row| row.get(0))
                    .map_err(Into::into)
            })
            .expect("count rows");
        assert_eq!(count, 0);

        let _ = fs::remove_dir_all(data_dir);
    }
}
