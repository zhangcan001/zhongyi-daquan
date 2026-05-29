use crate::errors::AppResult;
use crate::models::runtime::{
    BackgroundJob, CreateJobRequest, ListJobsRequest, MarkJobFailedRequest, MarkJobSuccessRequest,
    UpdateJobProgressRequest,
};
use crate::services::background_job_service;
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn create_job(
    state: State<'_, AppState>,
    request: CreateJobRequest,
) -> AppResult<BackgroundJob> {
    background_job_service::create_job(&state.database, request)
}

#[tauri::command]
pub fn update_job_progress(
    state: State<'_, AppState>,
    request: UpdateJobProgressRequest,
) -> AppResult<BackgroundJob> {
    background_job_service::update_job_progress(&state.database, request)
}

#[tauri::command]
pub fn mark_job_success(
    state: State<'_, AppState>,
    request: MarkJobSuccessRequest,
) -> AppResult<BackgroundJob> {
    background_job_service::mark_job_success(&state.database, request)
}

#[tauri::command]
pub fn mark_job_failed(
    state: State<'_, AppState>,
    request: MarkJobFailedRequest,
) -> AppResult<BackgroundJob> {
    background_job_service::mark_job_failed(&state.database, request)
}

#[tauri::command]
pub fn list_jobs(
    state: State<'_, AppState>,
    request: ListJobsRequest,
) -> AppResult<Vec<BackgroundJob>> {
    background_job_service::list_jobs(&state.database, request)
}

#[tauri::command]
pub fn get_job(state: State<'_, AppState>, job_id: i64) -> AppResult<BackgroundJob> {
    background_job_service::get_job(&state.database, job_id)
}
