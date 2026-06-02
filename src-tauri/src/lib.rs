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
        .plugin(tauri_plugin_dialog::init())
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
            commands::knowledge_commands::toggle_favorite,
            commands::knowledge_commands::list_favorites,
            commands::knowledge_commands::record_recent_view,
            commands::knowledge_commands::list_recent_views,
            commands::knowledge_commands::save_user_note,
            commands::knowledge_commands::delete_user_note,
            commands::knowledge_commands::get_dashboard_stats,
            commands::version_commands::list_knowledge_versions,
            commands::version_commands::get_knowledge_version,
            commands::version_commands::compare_knowledge_versions,
            commands::version_commands::rollback_knowledge_version,
            commands::knowledge_commands::batch_delete_knowledge_items,
            commands::knowledge_commands::batch_update_knowledge_status,
            commands::knowledge_commands::batch_add_knowledge_tags,
            commands::entry_commands::save_grid_dirty_rows,
            commands::search_commands::search_knowledge,
            commands::search_commands::search_knowledge_enhanced,
            commands::search_commands::search_knowledge_with_cache,
            commands::search_commands::get_hot_search_terms,
            commands::search_commands::clear_search_cache,
            commands::search_commands::list_knowledge_cache,
            commands::export_commands::export_knowledge_to_json,
            commands::export_commands::export_knowledge_to_csv,
            commands::export_commands::export_knowledge_to_excel,
            commands::error_commands::log_error,
            commands::error_commands::get_recent_errors,
            commands::error_commands::get_error_statistics,
            commands::error_commands::clear_old_error_logs,
            commands::search_commands::rebuild_search_index,
            commands::search_commands::generate_search_performance_test_data,
            commands::search_commands::smoke_test_searches,
            commands::performance_commands::list_performance_logs,
            commands::import_commands::preview_json_import,
            commands::import_commands::preview_csv_import,
            commands::import_commands::preview_excel_import,
            commands::import_commands::preview_zip_import,
            commands::import_commands::import_json_to_staging,
            commands::import_commands::import_csv_to_staging,
            commands::import_commands::import_excel_to_staging,
            commands::import_commands::import_zip_to_staging,
            commands::import_commands::preview_package_folder_import,
            commands::import_commands::import_package_folder,
            commands::import_commands::preview_import_plan,
            commands::import_commands::execute_import_plan,
            commands::import_commands::list_import_runs,
            commands::import_commands::get_import_run_report,
            commands::import_commands::rollback_import_run,
            commands::import_commands::save_field_mapping_template,
            commands::import_commands::list_field_mapping_templates,
            commands::import_commands::get_import_staging_page,
            commands::import_commands::validate_import_batch,
            commands::import_commands::confirm_import_batch,
            commands::import_commands::get_import_quality_report,
            commands::import_commands::rollback_import_batch,
            commands::import_commands::update_staging_row_field,
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
            commands::maintenance_commands::check_data_integrity,
            commands::maintenance_commands::clear_database_content,
            commands::performance_commands::list_performance_logs,
            commands::performance_commands::record_performance_log,
            commands::audit_commands::record_audit_log,
            commands::audit_commands::list_audit_logs
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
