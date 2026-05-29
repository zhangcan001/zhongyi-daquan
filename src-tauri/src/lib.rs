mod commands;
mod db;
mod errors;
mod models;
mod repositories;
mod services;

use db::connection::Database;
use tauri::Manager;

pub struct AppState {
    pub database: Database,
    pub data_dir: std::path::PathBuf,
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .map_err(|err| format!("无法读取应用数据目录: {err}"))?
                .join("中医大全数据");
            let database = Database::initialize(&data_dir)
                .map_err(|err| format!("数据库初始化失败: {err}"))?;

            app.manage(AppState { database, data_dir });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_commands::health_check,
            commands::app_commands::get_app_status,
            commands::ai_commands::ai_placeholder,
            commands::ai_commands::get_ai_provider_settings,
            commands::ai_commands::save_ai_provider_settings,
            commands::ai_commands::test_ai_connection,
            commands::ai_commands::run_ai_task,
            commands::ai_commands::get_ai_task_status,
            commands::ai_commands::cancel_ai_task,
            commands::knowledge_commands::list_knowledge_items,
            commands::knowledge_commands::get_knowledge_detail,
            commands::knowledge_commands::create_knowledge_item,
            commands::knowledge_commands::update_knowledge_item,
            commands::knowledge_commands::delete_knowledge_item,
            commands::knowledge_commands::set_knowledge_favorite,
            commands::entry_commands::save_grid_dirty_rows,
            commands::search_commands::search_knowledge,
            commands::search_commands::list_knowledge_cache,
            commands::search_commands::rebuild_search_index,
            commands::search_commands::generate_search_performance_test_data,
            commands::search_commands::smoke_test_searches,
            commands::performance_commands::list_performance_logs,
            commands::import_commands::preview_json_import,
            commands::import_commands::preview_csv_import,
            commands::import_commands::import_json_to_staging,
            commands::import_commands::import_csv_to_staging,
            commands::import_commands::save_field_mapping_template,
            commands::import_commands::list_field_mapping_templates,
            commands::import_commands::get_import_staging_page,
            commands::import_commands::validate_import_batch,
            commands::import_commands::confirm_import_batch,
            commands::clean_commands::apply_import_clean_step,
            commands::clean_commands::undo_last_import_clean_step,
            commands::relation_commands::run_duplicate_detection,
            commands::relation_commands::list_duplicate_candidates,
            commands::relation_commands::merge_duplicate_candidate,
            commands::relation_commands::generate_relation_suggestions,
            commands::relation_commands::list_relation_suggestions,
            commands::relation_commands::accept_relation_suggestion,
            commands::relation_commands::reject_relation_suggestion,
            commands::job_commands::create_job,
            commands::job_commands::update_job_progress,
            commands::job_commands::mark_job_success,
            commands::job_commands::mark_job_failed,
            commands::job_commands::list_jobs,
            commands::job_commands::get_job,
            commands::backup_commands::create_backup,
            commands::backup_commands::restore_backup,
            commands::maintenance_commands::run_rebuild_search_index_job,
            commands::maintenance_commands::optimize_database,
            commands::maintenance_commands::clean_temp_imports,
            commands::maintenance_commands::clean_old_performance_logs,
            commands::maintenance_commands::export_performance_report,
            commands::performance_commands::list_performance_logs,
            commands::performance_commands::record_performance_log,
            commands::audit_commands::record_audit_log,
            commands::audit_commands::list_audit_logs
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
