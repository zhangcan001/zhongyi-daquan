use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::relation::{
    ListDuplicateCandidatesRequest, ListDuplicateCandidatesResponse,
    MergeDuplicateCandidateRequest, MergeDuplicateCandidateResponse, RunDuplicateDetectionRequest,
    RunDuplicateDetectionResponse,
};
use crate::repositories::dedup_repository;
use crate::services::search_index_service;

pub fn run_duplicate_detection(
    database: &Database,
    request: RunDuplicateDetectionRequest,
) -> AppResult<RunDuplicateDetectionResponse> {
    let fingerprints_upserted =
        dedup_repository::rebuild_fingerprints(database, request.item_type.as_deref())?;
    let candidates_created = dedup_repository::detect_duplicates(
        database,
        request.batch_id,
        request.item_type.as_deref(),
    )?;
    Ok(RunDuplicateDetectionResponse {
        fingerprints_upserted,
        candidates_created,
    })
}

pub fn list_duplicate_candidates(
    database: &Database,
    request: ListDuplicateCandidatesRequest,
) -> AppResult<ListDuplicateCandidatesResponse> {
    let page = request.page.unwrap_or(1).max(1);
    let page_size = request.page_size.unwrap_or(50).clamp(1, 200);
    let (total, candidates) =
        dedup_repository::list_candidates(database, request.status.as_deref(), page, page_size)?;
    Ok(ListDuplicateCandidatesResponse {
        total,
        page,
        page_size,
        candidates,
    })
}

pub fn merge_duplicate_candidate(
    database: &Database,
    request: MergeDuplicateCandidateRequest,
) -> AppResult<MergeDuplicateCandidateResponse> {
    validate_strategy(&request.strategy)?;
    let result =
        dedup_repository::merge_candidate(database, request.candidate_id, &request.strategy)?;

    if request.strategy != "keep_existing" && request.strategy != "save_as_new" {
        let _ = search_index_service::index_knowledge_item(database, result.existing_item_id);
    }
    if let Some(created_item_id) = result.created_item_id {
        let _ = search_index_service::index_knowledge_item(database, created_item_id);
    }

    Ok(MergeDuplicateCandidateResponse {
        candidate_id: request.candidate_id,
        existing_item_id: result.existing_item_id,
        created_item_id: result.created_item_id,
        merge_record_id: result.merge_record_id,
        status: result.status,
    })
}

fn validate_strategy(strategy: &str) -> AppResult<()> {
    match strategy {
        "keep_existing" | "overwrite" | "fill_empty" | "merge_tags" | "save_as_new" => Ok(()),
        _ => Err(AppError::InvalidInput(format!("未知合并策略: {strategy}"))),
    }
}
