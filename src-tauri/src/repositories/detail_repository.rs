use crate::errors::AppResult;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Map, Value};

pub fn get_detail(connection: &Connection, item_type: &str, item_id: i64) -> AppResult<Value> {
    let mut main_detail = connection
        .query_row(
            "SELECT detail FROM knowledge_items WHERE id = ?1",
            params![item_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .unwrap_or_else(|| json!({}));
    let Some(schema) = schema_for(item_type) else {
        return Ok(main_detail);
    };
    let sql = format!(
        "SELECT {} FROM {} WHERE item_id = ?1",
        schema.fields.join(", "),
        schema.table
    );
    let detail = connection
        .query_row(&sql, params![item_id], |row| {
            let mut map = Map::new();
            for (index, field) in schema.fields.iter().enumerate() {
                if field.ends_with("_item_id") {
                    let value: Option<i64> = row.get(index)?;
                    map.insert(
                        camel_case(field),
                        value.map(Value::from).unwrap_or(Value::Null),
                    );
                } else {
                    let value: Option<String> = row.get(index)?;
                    map.insert(
                        camel_case(field),
                        value.map(Value::from).unwrap_or(Value::Null),
                    );
                }
            }
            Ok(Value::Object(map))
        })
        .optional()?;
    if let Some(table_detail) = detail {
        if let (Some(base), Some(extra)) = (main_detail.as_object_mut(), table_detail.as_object()) {
            for (key, value) in extra {
                base.entry(key.clone()).or_insert_with(|| value.clone());
            }
        }
    }
    Ok(main_detail)
}

pub fn upsert_detail_tx(
    connection: &Connection,
    item_type: &str,
    item_id: i64,
    detail: &Value,
) -> AppResult<()> {
    connection.execute(
        "UPDATE knowledge_items SET detail = ?2 WHERE id = ?1",
        params![item_id, normalize_detail_json(detail)],
    )?;
    let Some(schema) = schema_for(item_type) else {
        return Ok(());
    };
    connection.execute(
        &format!("DELETE FROM {} WHERE item_id = ?1", schema.table),
        params![item_id],
    )?;
    let columns = std::iter::once("item_id")
        .chain(schema.fields.iter().copied())
        .collect::<Vec<_>>();
    let placeholders = (1..=columns.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        schema.table,
        columns.join(", "),
        placeholders.join(", ")
    );
    let mut values = vec![rusqlite::types::Value::Integer(item_id)];
    for field in schema.fields {
        values.push(json_field(detail, field));
    }
    connection.execute(&sql, rusqlite::params_from_iter(values.iter()))?;
    Ok(())
}

fn normalize_detail_json(detail: &Value) -> String {
    match detail {
        Value::Object(_) => detail.to_string(),
        Value::String(text) => serde_json::from_str::<Value>(text)
            .map(|value| value.to_string())
            .unwrap_or_else(|_| json!({ "raw_detail": text, "parse_error": true }).to_string()),
        Value::Null => "{}".to_string(),
        other => other.to_string(),
    }
}

pub fn delete_detail_tx(connection: &Connection, item_type: &str, item_id: i64) -> AppResult<()> {
    if let Some(schema) = schema_for(item_type) {
        connection.execute(
            &format!("DELETE FROM {} WHERE item_id = ?1", schema.table),
            params![item_id],
        )?;
    }
    Ok(())
}

struct DetailSchema {
    table: &'static str,
    fields: &'static [&'static str],
}

fn schema_for(item_type: &str) -> Option<DetailSchema> {
    match item_type {
        "herb" => Some(DetailSchema {
            table: "herb_details",
            fields: &[
                "nature_flavor",
                "meridians",
                "effects",
                "indications",
                "dosage",
                "contraindications",
                "compatibility",
                "notes",
            ],
        }),
        "formula" => Some(DetailSchema {
            table: "formula_details",
            fields: &[
                "source_text",
                "composition",
                "usage",
                "effects",
                "indications",
                "explanation",
                "modifications",
                "contraindications",
                "notes",
            ],
        }),
        "meridian" => Some(DetailSchema {
            table: "meridian_details",
            fields: &[
                "meridian_code",
                "category",
                "yin_yang",
                "hand_foot",
                "organ_relation",
                "paired_meridian",
                "pathway_text",
                "main_indications",
                "notes",
            ],
        }),
        "acupoint" => Some(DetailSchema {
            table: "acupoint_details",
            fields: &[
                "acupoint_code",
                "meridian_item_id",
                "body_region",
                "body_subregion",
                "side_type",
                "standard_location",
                "locating_method",
                "bone_cun",
                "anatomy",
                "functions",
                "indications",
                "needling_summary",
                "moxibustion_summary",
                "massage_summary",
                "contraindications",
                "precautions",
                "risk_level",
            ],
        }),
        "syndrome" => Some(DetailSchema {
            table: "syndrome_details",
            fields: &[
                "symptoms",
                "tongue",
                "pulse",
                "pathogenesis",
                "treatment_principle",
                "notes",
            ],
        }),
        "disease" => Some(DetailSchema {
            table: "disease_details",
            fields: &[
                "symptoms",
                "common_syndromes",
                "care_advice",
                "medical_warning",
                "notes",
            ],
        }),
        _ => None,
    }
}

fn json_field(detail: &Value, field: &str) -> rusqlite::types::Value {
    let key = camel_case(field);
    let value = detail.get(&key).or_else(|| detail.get(field));
    match value {
        Some(Value::Number(number)) => number
            .as_i64()
            .map(rusqlite::types::Value::Integer)
            .unwrap_or(rusqlite::types::Value::Null),
        Some(Value::String(text)) if !text.trim().is_empty() => {
            rusqlite::types::Value::Text(text.trim().to_string())
        }
        Some(Value::Bool(value)) => rusqlite::types::Value::Integer(i64::from(*value)),
        _ => rusqlite::types::Value::Null,
    }
}

fn camel_case(field: &str) -> String {
    let mut result = String::new();
    let mut uppercase_next = false;
    for ch in field.chars() {
        if ch == '_' {
            uppercase_next = true;
        } else if uppercase_next {
            result.extend(ch.to_uppercase());
            uppercase_next = false;
        } else {
            result.push(ch);
        }
    }
    result
}
