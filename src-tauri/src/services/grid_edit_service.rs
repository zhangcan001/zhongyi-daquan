use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::knowledge::{GridSaveError, GridSaveRequest, GridSaveResponse};
use crate::services::{knowledge_service, search_index_service};

pub fn save_dirty_rows(
    database: &Database,
    request: GridSaveRequest,
) -> AppResult<GridSaveResponse> {
    let mut item_ids = Vec::new();
    let mut errors = Vec::new();

    for (index, row) in request.rows.into_iter().enumerate() {
        if row.item_type != request.item_type {
            errors.push(GridSaveError {
                row_index: index,
                field_name: "itemType".to_string(),
                message: "行类型与表格类型不一致".to_string(),
            });
            continue;
        }
        if row.name.trim().is_empty() {
            errors.push(GridSaveError {
                row_index: index,
                field_name: "name".to_string(),
                message: "名称不能为空".to_string(),
            });
            continue;
        }
        match knowledge_service::create(database, row) {
            Ok(response) => {
                if let Some(id) = response.item.id {
                    item_ids.push(id);
                }
            }
            Err(error) => errors.push(GridSaveError {
                row_index: index,
                field_name: "row".to_string(),
                message: error.to_string(),
            }),
        }
    }

    if !item_ids.is_empty() {
        for item_id in &item_ids {
            let _ = search_index_service::index_knowledge_item(database, *item_id);
        }
    }

    Ok(GridSaveResponse {
        saved_count: item_ids.len(),
        item_ids,
        errors,
    })
}
