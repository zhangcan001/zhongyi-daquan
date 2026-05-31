use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::data_pipeline::StagingIssue;
use serde_json::{Map, Value};
use std::collections::HashSet;

const KNOWLEDGE_TYPES: &[&str] = &[
    "中药", "方剂", "经络", "穴位", "证型", "病症", "herb", "formula", "meridian", "acupoint",
    "syndrome", "disease",
];

pub struct ValidationContext {
    known_meridians: HashSet<String>,
}

impl ValidationContext {
    pub fn load(database: &Database) -> AppResult<Self> {
        let known_meridians = database.with_connection(|connection| {
            let mut stmt = connection.prepare(
                "SELECT name FROM knowledge_items WHERE type IN ('经络','meridian')"
            )?;
            let names = stmt
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<HashSet<_>, _>>()?;
            Ok(names)
        })?;

        Ok(ValidationContext { known_meridians })
    }

    fn meridian_exists(&self, name: &str) -> bool {
        self.known_meridians.contains(name)
    }
}
#[allow(dead_code)]

pub fn validate_rows_batch(
    database: &Database,
    target_type: &str,
    rows: &[Map<String, Value>],
) -> AppResult<Vec<Vec<StagingIssue>>> {
    let context = ValidationContext::load(database)?;

    Ok(rows
        .iter()
        .map(|row| validate_row_with_context(&context, target_type, row))
        .collect())
}

pub fn validate_row(
    database: &Database,
    target_type: &str,
    row: &Map<String, Value>,
) -> AppResult<Vec<StagingIssue>> {
    let context = ValidationContext::load(database)?;
    Ok(validate_row_with_context(&context, target_type, row))
}

fn validate_row_with_context(
    context: &ValidationContext,
    target_type: &str,
    row: &Map<String, Value>,
) -> Vec<StagingIssue> {
    let mut issues = Vec::new();

    required(
        &mut issues,
        row,
        "type",
        "type 不能为空",
        "设置为导入目标类型",
    );
    required(&mut issues, row, "name", "name 不能为空", "补充知识名称");

    if let Some(value) = text(row, "type") {
        if !KNOWLEDGE_TYPES.contains(&value.as_str()) {
            issues.push(error(
                "invalid_enum",
                Some("type"),
                "type 不在允许的知识类型中",
                Some("请使用中药、方剂、经络、穴位、证型、病症之一"),
            ));
        }
    }

    if let Some(code) = text(row, "code") {
        if !valid_code(&code) {
            issues.push(error(
                "invalid_code_format",
                Some("code"),
                "code 格式不正确",
                Some("建议使用大写字母、数字、短横线或下划线，例如 ST36"),
            ));
        }
    }

    if matches!(target_type, "穴位" | "acupoint") {
        if let Some(meridians) = text(row, "meridians") {
            if !context.meridian_exists(&meridians) {
                issues.push(warning(
                    "reference_not_found",
                    Some("meridians"),
                    "经络引用暂未在正式知识库中找到",
                    Some("确认导入经络后再绑定，或先创建对应经络"),
                ));
            }
        }
    }

    // TODO: 线程 E 接入知识指纹后，在这里调用重复检测接口并写入 duplicate_candidates。
    issues
}

pub fn status_from_issues(
    issues: &[StagingIssue],
) -> (&'static str, Option<String>, Option<String>) {
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == "error")
        .map(|issue| issue.message.clone())
        .collect::<Vec<_>>();
    let warnings = issues
        .iter()
        .filter(|issue| issue.severity == "warning")
        .map(|issue| issue.message.clone())
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        (
            "error",
            Some(errors.join("; ")),
            (!warnings.is_empty()).then(|| warnings.join("; ")),
        )
    } else if !warnings.is_empty() {
        ("warning", None, Some(warnings.join("; ")))
    } else {
        ("valid", None, None)
    }
}

fn required(
    issues: &mut Vec<StagingIssue>,
    row: &Map<String, Value>,
    field: &'static str,
    message: &str,
    suggestion: &str,
) {
    if text(row, field).is_none() {
        issues.push(error("required", Some(field), message, Some(suggestion)));
    }
}

fn text(row: &Map<String, Value>, field: &str) -> Option<String> {
    match row.get(field) {
        Some(Value::String(text)) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Some(Value::Number(number)) => Some(number.to_string()),
        _ => None,
    }
}

fn valid_code(code: &str) -> bool {
    let mut chars = code.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    first.is_ascii_alphabetic()
        && code
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
}

fn error(code: &str, field: Option<&str>, message: &str, suggestion: Option<&str>) -> StagingIssue {
    issue("error", code, field, message, suggestion)
}

fn warning(
    code: &str,
    field: Option<&str>,
    message: &str,
    suggestion: Option<&str>,
) -> StagingIssue {
    issue("warning", code, field, message, suggestion)
}

fn issue(
    severity: &str,
    code: &str,
    field: Option<&str>,
    message: &str,
    suggestion: Option<&str>,
) -> StagingIssue {
    StagingIssue {
        severity: severity.to_string(),
        issue_code: code.to_string(),
        field_name: field.map(ToString::to_string),
        message: message.to_string(),
        suggestion: suggestion.map(ToString::to_string),
    }
}
