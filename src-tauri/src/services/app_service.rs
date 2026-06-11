use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::app::{AppStatus, HealthCheck};
use crate::repositories::{ai_repository, app_repository};
use std::path::Path;

pub fn health_check(database: &Database) -> AppResult<HealthCheck> {
    Ok(HealthCheck {
        ok: true,
        database_ready: app_repository::database_ready(database)?,
    })
}

pub fn get_app_status(database: &Database, data_dir: &Path) -> AppResult<AppStatus> {
    Ok(AppStatus {
        app_name: "中医大全".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        database_ready: app_repository::database_ready(database)?,
        ai_enabled: ai_repository::ai_enabled(database)?,
        data_dir: data_dir.display().to_string(),
        message: "本地知识库已就绪；AI 默认关闭，配置后可使用本地检索与可选联网摘要。".to_string(),
    })
}
