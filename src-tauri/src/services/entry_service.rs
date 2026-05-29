use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::knowledge::{GridSaveRequest, GridSaveResponse};
use crate::services::grid_edit_service;

pub fn save_grid_dirty_rows(
    database: &Database,
    request: GridSaveRequest,
) -> AppResult<GridSaveResponse> {
    grid_edit_service::save_dirty_rows(database, request)
}
