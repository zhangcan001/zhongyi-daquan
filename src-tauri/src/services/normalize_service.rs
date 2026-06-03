use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::repositories::standard_term_repository;
use pinyin::ToPinyin;
use serde_json::{Map, Value};
use std::collections::HashMap;

pub struct NormalizeCache {
    terms: HashMap<(String, String), Option<String>>,
}

impl NormalizeCache {
    pub fn load(database: &Database) -> AppResult<Self> {
        standard_term_repository::ensure_basic_terms(database)?;

        let mut terms = HashMap::new();

        database.with_connection(|connection| {
            let mut stmt = connection
                .prepare("SELECT term_type, standard_name, aliases FROM standard_terms")?;

            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            })?;

            for row in rows {
                let (term_type, standard_name, aliases) = row?;

                terms.insert(
                    (term_type.clone(), standard_name.clone()),
                    Some(standard_name.clone()),
                );

                if let Some(alias_str) = aliases {
                    for alias in alias_str.split(',').map(str::trim) {
                        if !alias.is_empty() {
                            terms.insert(
                                (term_type.clone(), alias.to_string()),
                                Some(standard_name.clone()),
                            );
                        }
                    }
                }
            }

            Ok(())
        })?;

        Ok(NormalizeCache { terms })
    }

    fn standardize(&self, term_type: &str, input: &str) -> Option<String> {
        self.terms
            .get(&(term_type.to_string(), input.to_string()))
            .and_then(|opt| opt.clone())
    }
}

pub fn normalize_rows_batch(
    database: &Database,
    rows: Vec<Map<String, Value>>,
) -> AppResult<Vec<Map<String, Value>>> {
    let cache = NormalizeCache::load(database)?;

    rows.into_iter()
        .map(|row| normalize_row_with_cache(&cache, row))
        .collect()
}

fn normalize_row_with_cache(
    cache: &NormalizeCache,
    mut row: Map<String, Value>,
) -> AppResult<Map<String, Value>> {
    for value in row.values_mut() {
        if let Value::String(text) = value {
            *text = to_half_width(text).trim().to_string();
        }
    }

    if let Some(Value::String(code)) = row.get_mut("code") {
        *code = code.to_ascii_uppercase();
    }
    if let Some(Value::String(code)) = row.get("code").cloned() {
        row.entry("acupoint_code".to_string())
            .or_insert_with(|| Value::String(code));
    }

    if missing_text(row.get("pinyin")) {
        if let Some(Value::String(name)) = row.get("name") {
            row.insert(
                "pinyin".to_string(),
                Value::String(simple_pinyin_placeholder(name)),
            );
        }
    }

    normalize_tag_field(&mut row, "tags");
    normalize_list_field(&mut row, "alias");
    normalize_list_field(&mut row, "meridians");

    if let Some(Value::String(meridians)) = row.get("meridians").cloned() {
        let normalized = normalize_terms_with_cache(cache, "meridian", &meridians);
        row.insert("meridians".to_string(), Value::String(normalized));
    }

    if let Some(Value::String(category)) = row.get("category").cloned() {
        row.insert(
            "category".to_string(),
            Value::String(normalize_category(&category)),
        );
    }

    if let Some(Value::String(name)) = row.get("name").cloned() {
        if let Some(standard) = cache.standardize("herb_name", &name) {
            row.insert("name".to_string(), Value::String(standard));
        }
    }

    let empty_keys = row
        .iter()
        .filter_map(|(key, value)| missing_text(Some(value)).then(|| key.clone()))
        .collect::<Vec<_>>();
    for key in empty_keys {
        row.insert(key, Value::Null);
    }

    Ok(row)
}

fn normalize_terms_with_cache(cache: &NormalizeCache, term_type: &str, text: &str) -> String {
    let mut terms = Vec::new();
    for part in text
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let standard = cache
            .standardize(term_type, part)
            .unwrap_or_else(|| part.to_string());
        terms.push(standard);
    }
    terms.sort();
    terms.dedup();
    terms.join(",")
}

pub fn normalize_row(
    database: &Database,
    mut row: Map<String, Value>,
) -> AppResult<Map<String, Value>> {
    standard_term_repository::ensure_basic_terms(database)?;

    for value in row.values_mut() {
        if let Value::String(text) = value {
            *text = to_half_width(text).trim().to_string();
        }
    }

    if let Some(Value::String(code)) = row.get_mut("code") {
        *code = code.to_ascii_uppercase();
    }
    if let Some(Value::String(code)) = row.get("code").cloned() {
        row.entry("acupoint_code".to_string())
            .or_insert_with(|| Value::String(code));
    }

    if missing_text(row.get("pinyin")) {
        if let Some(Value::String(name)) = row.get("name") {
            row.insert(
                "pinyin".to_string(),
                Value::String(simple_pinyin_placeholder(name)),
            );
        }
    }

    normalize_tag_field(&mut row, "tags");
    normalize_list_field(&mut row, "alias");
    normalize_list_field(&mut row, "meridians");

    if let Some(Value::String(meridians)) = row.get("meridians").cloned() {
        let normalized = normalize_terms(database, "meridian", &meridians)?;
        row.insert("meridians".to_string(), Value::String(normalized));
    }

    if let Some(Value::String(category)) = row.get("category").cloned() {
        row.insert(
            "category".to_string(),
            Value::String(normalize_category(&category)),
        );
    }

    if let Some(Value::String(name)) = row.get("name").cloned() {
        if let Some(standard) = standard_term_repository::standardize(database, "herb_name", &name)?
        {
            row.insert("name".to_string(), Value::String(standard));
        }
    }

    let empty_keys = row
        .iter()
        .filter_map(|(key, value)| missing_text(Some(value)).then(|| key.clone()))
        .collect::<Vec<_>>();
    for key in empty_keys {
        row.insert(key, Value::Null);
    }

    Ok(row)
}

pub fn to_half_width(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            '\u{3000}' => ' ',
            '\u{ff01}'..='\u{ff5e}' => char::from_u32(ch as u32 - 0xfee0).unwrap_or(ch),
            _ => ch,
        })
        .collect()
}

fn normalize_tag_field(row: &mut Map<String, Value>, field: &str) {
    if let Some(Value::Array(values)) = row.get(field).cloned() {
        let value = values
            .iter()
            .filter_map(value_to_text)
            .collect::<Vec<_>>()
            .join(",");
        row.insert(field.to_string(), Value::String(split_join(&value)));
        return;
    }
    if let Some(Value::String(text)) = row.get(field).cloned() {
        let value = split_join(&text);
        row.insert(field.to_string(), Value::String(value));
    }
}

fn normalize_list_field(row: &mut Map<String, Value>, field: &str) {
    if let Some(Value::Array(values)) = row.get(field).cloned() {
        let value = values
            .iter()
            .filter_map(value_to_text)
            .collect::<Vec<_>>()
            .join(",");
        row.insert(field.to_string(), Value::String(split_join(&value)));
        return;
    }
    if let Some(Value::String(text)) = row.get(field).cloned() {
        row.insert(field.to_string(), Value::String(split_join(&text)));
    }
}

fn value_to_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_string()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn split_join(text: &str) -> String {
    text.split([',', '，', ';', '；', '、', '|'])
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(",")
}

fn normalize_terms(database: &Database, term_type: &str, text: &str) -> AppResult<String> {
    let mut terms = Vec::new();
    for part in text
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
    {
        let standard = standard_term_repository::standardize(database, term_type, part)?
            .unwrap_or_else(|| part.to_string());
        terms.push(standard);
    }
    terms.sort();
    terms.dedup();
    Ok(terms.join(","))
}

fn normalize_category(category: &str) -> String {
    match category.trim() {
        "腧穴" => "穴位".to_string(),
        "草药" => "中药".to_string(),
        value => value.to_string(),
    }
}

fn simple_pinyin_placeholder(name: &str) -> String {
    if name.is_ascii() {
        name.to_ascii_lowercase()
    } else {
        name.to_pinyin()
            .map(|pinyin| {
                pinyin
                    .map(|item| item.plain().to_string())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

fn missing_text(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => true,
        Some(Value::String(text)) => text.trim().is_empty(),
        _ => false,
    }
}
