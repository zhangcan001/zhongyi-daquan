use crate::errors::AppResult;
use crate::models::app::{AppStatus, HealthCheck};
use crate::services::app_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn health_check(state: State<'_, AppState>) -> AppResult<HealthCheck> {
    app_service::health_check(&state.database)
}

#[tauri::command]
pub fn get_app_status(state: State<'_, AppState>) -> AppResult<AppStatus> {
    app_service::get_app_status(&state.database, &state.data_dir)
}
