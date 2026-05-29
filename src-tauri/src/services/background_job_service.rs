use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::runtime::{
    BackgroundJob, CreateJobRequest, ListJobsRequest, MarkJobFailedRequest, MarkJobSuccessRequest,
    UpdateJobProgressRequest,
};
use crate::repositories::job_repository;

pub fn create_job(database: &Database, request: CreateJobRequest) -> AppResult<BackgroundJob> {
    validate_job_type(&request.job_type)?;
    job_repository::create(database, &request.job_type, request.params_json.as_deref())
}

pub fn create_internal_job(
    database: &Database,
    job_type: &str,
    params_json: Option<&str>,
) -> AppResult<BackgroundJob> {
    validate_job_type(job_type)?;
    job_repository::create(database, job_type, params_json)
}

pub fn update_job_progress(
    database: &Database,
    request: UpdateJobProgressRequest,
) -> AppResult<BackgroundJob> {
    let progress = request.progress.clamp(0.0, 100.0);
    job_repository::update_progress(
        database,
        request.job_id,
        progress,
        request.result_json.as_deref(),
    )
}

pub fn mark_job_success(
    database: &Database,
    request: MarkJobSuccessRequest,
) -> AppResult<BackgroundJob> {
    job_repository::mark_success(database, request.job_id, request.result_json.as_deref())
}

pub fn mark_job_failed(
    database: &Database,
    request: MarkJobFailedRequest,
) -> AppResult<BackgroundJob> {
    job_repository::mark_failed(database, request.job_id, &request.error_message)
}

pub fn list_jobs(database: &Database, request: ListJobsRequest) -> AppResult<Vec<BackgroundJob>> {
    if let Some(job_type) = request.job_type.as_deref() {
        validate_job_type(job_type)?;
    }
    job_repository::list(
        database,
        request.status.as_deref(),
        request.job_type.as_deref(),
        request.limit.unwrap_or(50),
    )
}

pub fn get_job(database: &Database, job_id: i64) -> AppResult<BackgroundJob> {
    job_repository::get(database, job_id)
}

pub fn set_progress(database: &Database, job_id: i64, progress: f64) -> AppResult<BackgroundJob> {
    job_repository::update_progress(database, job_id, progress.clamp(0.0, 100.0), None)
}

pub fn success_with_json(
    database: &Database,
    job_id: i64,
    result_json: &str,
) -> AppResult<BackgroundJob> {
    job_repository::mark_success(database, job_id, Some(result_json))
}

pub fn fail_with_message(
    database: &Database,
    job_id: i64,
    message: &str,
) -> AppResult<BackgroundJob> {
    job_repository::mark_failed(database, job_id, message)
}

fn validate_job_type(job_type: &str) -> AppResult<()> {
    if job_repository::is_allowed_job_type(job_type) {
        Ok(())
    } else {
        Err(AppError::InvalidInput(format!(
            "不支持的任务类型: {job_type}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::Database;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_data_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("zhongyi-job-test-{test_name}-{unique}"))
    }

    #[test]
    fn creates_updates_and_completes_background_job() {
        let data_dir = temp_data_dir("flow");
        let database = Database::initialize(&data_dir).expect("database initializes");

        let job = create_job(
            &database,
            CreateJobRequest {
                job_type: "backup".to_string(),
                params_json: Some("{\"source\":\"test\"}".to_string()),
            },
        )
        .expect("job creates");
        assert_eq!(job.status, "pending");
        assert_eq!(job.progress, 0.0);

        let running = update_job_progress(
            &database,
            UpdateJobProgressRequest {
                job_id: job.id,
                progress: 42.0,
                result_json: None,
            },
        )
        .expect("job updates");
        assert_eq!(running.status, "running");
        assert_eq!(running.progress, 42.0);

        let completed = mark_job_success(
            &database,
            MarkJobSuccessRequest {
                job_id: job.id,
                result_json: Some("{\"ok\":true}".to_string()),
            },
        )
        .expect("job completes");
        assert_eq!(completed.status, "success");
        assert_eq!(completed.progress, 100.0);

        let jobs = list_jobs(
            &database,
            ListJobsRequest {
                status: Some("success".to_string()),
                job_type: Some("backup".to_string()),
                limit: Some(10),
            },
        )
        .expect("jobs list");
        assert_eq!(jobs.len(), 1);

        let _ = fs::remove_dir_all(data_dir);
    }
}
