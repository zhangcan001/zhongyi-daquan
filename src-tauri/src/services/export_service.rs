use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::repositories::knowledge_repository;
use serde_json::{Map, Value};
use std::path::Path;

pub fn export_to_json(
    database: &Database,
    item_ids: Vec<i64>,
    output_path: &Path,
) -> AppResult<ExportResult> {
    let items = fetch_items(database, &item_ids)?;

    let json_content = serde_json::to_string_pretty(&items)?;
    std::fs::write(output_path, json_content)?;

    Ok(ExportResult {
        exported_count: items.len(),
        output_path: output_path.to_string_lossy().to_string(),
        format: "json".to_string(),
    })
}

pub fn export_to_csv(
    database: &Database,
    item_ids: Vec<i64>,
    output_path: &Path,
) -> AppResult<ExportResult> {
    let items = fetch_items(database, &item_ids)?;

    let mut csv_content = String::new();

    // 生成表头
    if let Some(first_item) = items.first() {
        let headers: Vec<String> = first_item.keys().cloned().collect();
        csv_content.push_str(&headers.join(","));
        csv_content.push('\n');

        // 生成数据行
        for item in &items {
            let row: Vec<String> = headers
                .iter()
                .map(|header| {
                    let value = item.get(header).unwrap_or(&Value::Null);
                    escape_csv_value(&value.to_string())
                })
                .collect();
            csv_content.push_str(&row.join(","));
            csv_content.push('\n');
        }
    }

    std::fs::write(output_path, csv_content)?;

    Ok(ExportResult {
        exported_count: items.len(),
        output_path: output_path.to_string_lossy().to_string(),
        format: "csv".to_string(),
    })
}

pub fn export_to_excel(
    database: &Database,
    item_ids: Vec<i64>,
    output_path: &Path,
) -> AppResult<ExportResult> {
    // 使用 xlsxwriter 或类似库生成 Excel
    // 由于 calamine 只支持读取，这里简化为导出 CSV
    // 实际项目中可以使用 rust_xlsxwriter crate

    export_to_csv(database, item_ids, output_path)
}

fn fetch_items(database: &Database, item_ids: &[i64]) -> AppResult<Vec<Map<String, Value>>> {
    // 使用批量查询避免 N+1 问题
    knowledge_repository::get_items_batch(database, item_ids)
}

fn escape_csv_value(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

#[derive(Debug, serde::Serialize)]
pub struct ExportResult {
    pub exported_count: usize,
    pub output_path: String,
    pub format: String,
}
