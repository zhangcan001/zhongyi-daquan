use crate::errors::AppResult;
use crate::models::data_pipeline::{
    ConfirmImportResult, CreateImportRequest, FieldMappingTemplate, ImportBatchSummary,
    ImportParsedPreview, SaveMappingTemplateRequest, StagingPage,
};
use crate::services::{field_mapping_service, import_project_service};
use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn preview_json_import(content: String) -> AppResult<ImportParsedPreview> {
    import_project_service::preview_json(&content)
}

#[tauri::command]
pub fn preview_csv_import(content: String) -> AppResult<ImportParsedPreview> {
    import_project_service::preview_csv(&content)
}

#[tauri::command]
pub fn import_json_to_staging(
    state: State<'_, AppState>,
    request: CreateImportRequest,
) -> AppResult<ImportBatchSummary> {
    import_project_service::import_json(&state.database, request)
}

#[tauri::command]
pub fn import_csv_to_staging(
    state: State<'_, AppState>,
    request: CreateImportRequest,
) -> AppResult<ImportBatchSummary> {
    import_project_service::import_csv(&state.database, request)
}

#[tauri::command]
pub fn preview_excel_import(content: Vec<u8>) -> AppResult<ImportParsedPreview> {
    import_project_service::preview_excel(&content)
}

#[tauri::command]
pub fn import_excel_to_staging(
    state: State<'_, AppState>,
    request: CreateImportRequest,
) -> AppResult<ImportBatchSummary> {
    import_project_service::import_excel(&state.database, request)
}

#[tauri::command]
pub fn save_field_mapping_template(
    state: State<'_, AppState>,
    request: SaveMappingTemplateRequest,
) -> AppResult<FieldMappingTemplate> {
    field_mapping_service::save_template(&state.database, request)
}

#[tauri::command]
pub fn list_field_mapping_templates(
    state: State<'_, AppState>,
    target_type: Option<String>,
) -> AppResult<Vec<FieldMappingTemplate>> {
    field_mapping_service::list_templates(&state.database, target_type)
}

#[tauri::command]
pub fn get_import_staging_page(
    state: State<'_, AppState>,
    batch_id: i64,
    page: i64,
    page_size: i64,
) -> AppResult<StagingPage> {
    import_project_service::staging_page(&state.database, batch_id, page, page_size)
}

#[tauri::command]
pub fn validate_import_batch(
    state: State<'_, AppState>,
    batch_id: i64,
) -> AppResult<ImportBatchSummary> {
    import_project_service::validate_batch(&state.database, batch_id)
}

#[tauri::command]
pub fn confirm_import_batch(
    state: State<'_, AppState>,
    batch_id: i64,
) -> AppResult<ConfirmImportResult> {
    import_project_service::confirm_import(&state.database, batch_id)
}

#[tauri::command]
pub fn update_staging_row_field(
    state: State<'_, AppState>,
    batch_id: i64,
    row_id: i64,
    field_name: String,
    new_value: String,
) -> AppResult<ImportBatchSummary> {
    import_project_service::update_staging_row_field(
        &state.database,
        batch_id,
        row_id,
        &field_name,
        &new_value,
    )
}
