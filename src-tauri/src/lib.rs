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
            commands::ai_commands::ai_placeholder
        ])
        .run(tauri::generate_context!())
        .expect("failed to run tauri application");
}
