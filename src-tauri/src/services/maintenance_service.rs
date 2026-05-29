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
