use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::runtime::{BackupManifest, BackupReport, RestoreBackupRequest, RestoreReport};
use crate::repositories::{audit_repository, backup_repository};
use crate::services::{background_job_service, search_index_service};
use chrono::Utc;
use rusqlite::params;
use std::fs;
use std::path::{Path, PathBuf};

pub fn create_backup(database: &Database, data_dir: &Path) -> AppResult<BackupReport> {
    create_backup_with_note(database, data_dir, "manual")
}

pub fn restore_backup(
    database: &Database,
    data_dir: &Path,
    request: RestoreBackupRequest,
) -> AppResult<RestoreReport> {
    let params_json = serde_json::json!({ "backupDir": request.backup_dir }).to_string();
    let job = background_job_service::create_internal_job(database, "restore", Some(&params_json))?;
    let backup_dir = PathBuf::from(&request.backup_dir);

    let result: AppResult<RestoreReport> = (|| {
        let mut active_job_id = job.id;
        backup_repository::validate_backup_dir(&backup_dir)?;
        background_job_service::set_progress(database, active_job_id, 10.0)?;

        let safety_backup = create_backup_with_note(database, data_dir, "before_restore")?;
        background_job_service::set_progress(database, active_job_id, 30.0)?;

        let database_source = backup_dir.join("database").join("zhongyi.db");
        database.replace_database_file(&database_source)?;
        let restored_job =
            background_job_service::create_internal_job(database, "restore", Some(&params_json))?;
        active_job_id = restored_job.id;
        background_job_service::set_progress(database, active_job_id, 55.0)?;

        let images_restored = backup_repository::replace_dir_if_exists(
            &backup_dir.join("images"),
            &data_dir.join("images"),
        )?;
        background_job_service::set_progress(database, active_job_id, 70.0)?;

        let config_restored = backup_repository::replace_dir_if_exists(
            &backup_dir.join("config"),
            &data_dir.join("config"),
        )?;
        background_job_service::set_progress(database, active_job_id, 80.0)?;

        // TODO: 当搜索线程提供异步搜索索引任务队列后，改为投递 rebuild_search_index 后台任务。
        let rebuild_note = match search_index_service::rebuild_search_index(database) {
            Ok(response) => format!(
                "已调用 rebuild_search_index，占位重建 {} 条索引、{} 个搜索词。",
                response.indexed_items, response.search_terms
            ),
            Err(err) => format!("已预留 rebuild_search_index 调用点，本次调用失败: {err}"),
        };
        background_job_service::set_progress(database, active_job_id, 90.0)?;

        audit_repository::record(
            database,
            "restore_backup",
            Some("backup"),
            None,
            None,
            Some(&serde_json::json!({ "backupDir": request.backup_dir }).to_string()),
        )?;

        let completed_job = background_job_service::success_with_json(
            database,
            active_job_id,
            &serde_json::json!({
                "restoredFrom": backup_dir.display().to_string(),
                "safetyBackupDir": safety_backup.backup_dir,
                "rebuildSearchIndexNote": rebuild_note
            })
            .to_string(),
        )?;

        Ok(RestoreReport {
            job: completed_job,
            restored_from: backup_dir.display().to_string(),
            safety_backup_dir: safety_backup.backup_dir,
            database_restored: true,
            images_restored,
            config_restored,
            rebuild_search_index_note: rebuild_note,
        })
    })();

    match result {
        Ok(report) => Ok(report),
        Err(err) => {
            let _ = background_job_service::fail_with_message(database, job.id, &err.to_string());
            Err(err)
        }
    }
}

fn create_backup_with_note(
    database: &Database,
    data_dir: &Path,
    note: &str,
) -> AppResult<BackupReport> {
    let now = Utc::now();
    let backup_id = format!(
        "backup-{}-{}-{note}",
        now.format("%Y%m%d%H%M%S"),
        now.timestamp_subsec_millis()
    );
    let params_json = serde_json::json!({ "backupId": backup_id, "note": note }).to_string();
    let job = background_job_service::create_internal_job(database, "backup", Some(&params_json))?;

    let result: AppResult<BackupReport> = (|| {
        let backup_dir = backup_repository::ensure_backup_dir(data_dir, &backup_id)?;
        let database_dir = backup_dir.join("database");
        fs::create_dir_all(&database_dir)?;
        let database_path = database_dir.join("zhongyi.db");

        background_job_service::set_progress(database, job.id, 15.0)?;
        vacuum_database_to(database, &database_path)?;
        background_job_service::set_progress(database, job.id, 45.0)?;

        let images_copied = backup_repository::copy_dir_if_exists(
            &data_dir.join("images"),
            &backup_dir.join("images"),
        )?;
        background_job_service::set_progress(database, job.id, 65.0)?;

        let config_copied = backup_repository::copy_dir_if_exists(
            &data_dir.join("config"),
            &backup_dir.join("config"),
        )?;
        background_job_service::set_progress(database, job.id, 80.0)?;

        let manifest = BackupManifest {
            backup_id: backup_id.clone(),
            created_at: Utc::now().to_rfc3339(),
            app_name: "中医大全".to_string(),
            database_file: "database/zhongyi.db".to_string(),
            includes_images: images_copied,
            includes_config: config_copied,
            notes: vec![
                "SQLite 数据库通过 VACUUM INTO 生成一致性副本。".to_string(),
                "恢复后会调用 rebuild_search_index；后续可替换为真正异步索引任务。".to_string(),
            ],
        };
        let manifest_path = backup_dir.join("backup_manifest.json");
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;

        audit_repository::record(
            database,
            "create_backup",
            Some("backup"),
            None,
            None,
            Some(&serde_json::to_string(&manifest)?),
        )?;

        let completed_job = background_job_service::success_with_json(
            database,
            job.id,
            &serde_json::json!({
                "backupId": backup_id,
                "backupDir": backup_dir.display().to_string(),
                "manifestPath": manifest_path.display().to_string()
            })
            .to_string(),
        )?;

        Ok(BackupReport {
            job: completed_job,
            backup_id,
            backup_dir: backup_dir.display().to_string(),
            manifest_path: manifest_path.display().to_string(),
            database_path: database_path.display().to_string(),
            images_path: images_copied.then(|| backup_dir.join("images").display().to_string()),
            config_path: config_copied.then(|| backup_dir.join("config").display().to_string()),
        })
    })();

    match result {
        Ok(report) => Ok(report),
        Err(err) => {
            let _ = background_job_service::fail_with_message(database, job.id, &err.to_string());
            Err(err)
        }
    }
}

fn vacuum_database_to(database: &Database, destination: &Path) -> AppResult<()> {
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    let destination_text = destination.display().to_string();
    database.with_connection(|connection| {
        connection.execute("VACUUM main INTO ?1", params![destination_text])?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_data_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("zhongyi-backup-test-{test_name}-{unique}"))
    }

    #[test]
    fn backs_up_and_restores_database_files_and_rebuild_hook() {
        let data_dir = temp_data_dir("restore");
        let database = Database::initialize(&data_dir).expect("database initializes");

        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items (type, code, name, data_status, created_at, updated_at)
                     VALUES ('herb', 'HERB-BACKUP', '备份测试', 'validated', datetime('now'), datetime('now'))",
                    [],
                )?;
                Ok(())
            })
            .expect("seed row");

        std::fs::write(
            data_dir.join("config").join("settings.json"),
            "{\"theme\":\"test\"}",
        )
        .expect("config file");
        std::fs::write(
            data_dir.join("images").join("sample.txt"),
            "image-placeholder",
        )
        .expect("image placeholder");

        let backup = create_backup(&database, &data_dir).expect("backup creates");
        assert!(PathBuf::from(&backup.manifest_path).exists());
        assert!(PathBuf::from(&backup.database_path).exists());

        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items (type, code, name, data_status, created_at, updated_at)
                     VALUES ('herb', 'HERB-AFTER', '恢复前新增', 'validated', datetime('now'), datetime('now'))",
                    [],
                )?;
                Ok(())
            })
            .expect("post backup row");

        let restore = restore_backup(
            &database,
            &data_dir,
            RestoreBackupRequest {
                backup_dir: backup.backup_dir.clone(),
            },
        )
        .expect("restore succeeds");
        assert!(restore.database_restored);
        assert!(restore
            .rebuild_search_index_note
            .contains("rebuild_search_index"));

        database
            .with_connection(|connection| {
                let restored_count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM knowledge_items WHERE code = 'HERB-BACKUP'",
                    [],
                    |row| row.get(0),
                )?;
                let after_count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM knowledge_items WHERE code = 'HERB-AFTER'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(restored_count, 1);
                assert_eq!(after_count, 0);
                Ok(())
            })
            .expect("restored database state");

        let _ = std::fs::remove_dir_all(data_dir);
    }
}
