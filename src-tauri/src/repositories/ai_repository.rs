use crate::db::connection::Database;
use crate::errors::AppError;
use crate::errors::AppResult;
use crate::models::ai::{AiProviderSettings, AiTask, SaveAiProviderSettingsRequest};
use rusqlite::{params, OptionalExtension};

pub fn ai_enabled(database: &Database) -> AppResult<bool> {
    Ok(get_provider_settings(database)?
        .map(|settings| settings.enabled && settings.provider_type != "disabled")
        .unwrap_or(false))
}

pub fn get_provider_settings(database: &Database) -> AppResult<Option<AiProviderSettings>> {
    database.with_connection(|connection| {
        connection
            .query_row(
                "SELECT id, provider_type, provider_name, base_url, model_name,
                    api_key_encrypted IS NOT NULL AND length(api_key_encrypted) > 0,
                    timeout_seconds, max_tokens, temperature, max_context_items,
                    max_context_chars, only_use_local_context, safety_mode, enabled,
                    created_at, updated_at
                 FROM ai_provider_settings
                 ORDER BY id DESC
                 LIMIT 1",
                [],
                |row| {
                    let has_api_key: i64 = row.get(5)?;
                    let only_use_local_context: i64 = row.get(11)?;
                    let enabled: i64 = row.get(13)?;
                    Ok(AiProviderSettings {
                        id: row.get(0)?,
                        provider_type: row.get(1)?,
                        provider_name: row.get(2)?,
                        base_url: row.get(3)?,
                        model_name: row.get(4)?,
                        has_api_key: has_api_key == 1,
                        timeout_seconds: row.get(6)?,
                        max_tokens: row.get(7)?,
                        temperature: row.get(8)?,
                        max_context_items: row.get(9)?,
                        max_context_chars: row.get(10)?,
                        only_use_local_context: only_use_local_context == 1,
                        safety_mode: row
                            .get::<_, Option<String>>(12)?
                            .unwrap_or_else(|| "strict".to_string()),
                        enabled: enabled == 1,
                        created_at: row.get(14)?,
                        updated_at: row.get(15)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    })
}

pub fn save_provider_settings(
    database: &Database,
    request: SaveAiProviderSettingsRequest,
) -> AppResult<AiProviderSettings> {
    database.with_connection(|connection| {
        let existing_key: Option<String> = connection
            .query_row(
                "SELECT api_key_encrypted FROM ai_provider_settings ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        let api_key = request
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .or(existing_key);
        let enabled = request
            .enabled
            .unwrap_or(request.provider_type != "disabled")
            && request.provider_type != "disabled";
        connection.execute("DELETE FROM ai_provider_settings", [])?;
        connection.execute(
            "INSERT INTO ai_provider_settings (
                provider_type, provider_name, base_url, api_key_encrypted, model_name,
                timeout_seconds, max_tokens, temperature, max_context_items, max_context_chars,
                only_use_local_context, safety_mode, enabled, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, datetime('now'), datetime('now'))",
            params![
                request.provider_type,
                request.provider_name,
                request.base_url,
                api_key,
                request.model_name,
                request.timeout_seconds,
                request.max_tokens,
                request.temperature,
                request.max_context_items,
                request.max_context_chars,
                i64::from(request.only_use_local_context.unwrap_or(true)),
                request.safety_mode.unwrap_or_else(|| "strict".to_string()),
                i64::from(enabled),
            ],
        )?;

        let id = connection.last_insert_rowid();
        let settings = connection.query_row(
            "SELECT id, provider_type, provider_name, base_url, model_name,
                api_key_encrypted IS NOT NULL AND length(api_key_encrypted) > 0,
                timeout_seconds, max_tokens, temperature, max_context_items,
                max_context_chars, only_use_local_context, safety_mode, enabled,
                created_at, updated_at
             FROM ai_provider_settings
             WHERE id = ?1",
            params![id],
            |row| {
                let has_api_key: i64 = row.get(5)?;
                let only_use_local_context: i64 = row.get(11)?;
                let enabled: i64 = row.get(13)?;
                Ok(AiProviderSettings {
                    id: row.get(0)?,
                    provider_type: row.get(1)?,
                    provider_name: row.get(2)?,
                    base_url: row.get(3)?,
                    model_name: row.get(4)?,
                    has_api_key: has_api_key == 1,
                    timeout_seconds: row.get(6)?,
                    max_tokens: row.get(7)?,
                    temperature: row.get(8)?,
                    max_context_items: row.get(9)?,
                    max_context_chars: row.get(10)?,
                    only_use_local_context: only_use_local_context == 1,
                    safety_mode: row
                        .get::<_, Option<String>>(12)?
                        .unwrap_or_else(|| "strict".to_string()),
                    enabled: enabled == 1,
                    created_at: row.get(14)?,
                    updated_at: row.get(15)?,
                })
            },
        )?;

        Ok(settings)
    })
}

pub fn get_api_key(database: &Database) -> AppResult<Option<String>> {
    database.with_connection(|connection| {
        connection
            .query_row(
                "SELECT api_key_encrypted FROM ai_provider_settings ORDER BY id DESC LIMIT 1",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map(|value| value.flatten())
            .map_err(Into::into)
    })
}

pub fn clear_api_key(database: &Database) -> AppResult<Option<AiProviderSettings>> {
    database.with_connection(|connection| {
        connection.execute(
            "UPDATE ai_provider_settings SET api_key_encrypted = NULL, updated_at = datetime('now')",
            [],
        )?;
        Ok(())
    })?;
    get_provider_settings(database)
}

pub fn create_task(
    database: &Database,
    task_type: &str,
    input_json: Option<&str>,
    related_batch_id: Option<i64>,
    related_row_id: Option<i64>,
    related_item_id: Option<i64>,
) -> AppResult<i64> {
    database.with_connection(|connection| {
        connection.execute(
            "INSERT INTO ai_tasks
             (task_type, status, input_json, related_batch_id, related_row_id, related_item_id, created_at, updated_at)
             VALUES (?1, 'running', ?2, ?3, ?4, ?5, datetime('now'), datetime('now'))",
            params![
                task_type,
                input_json,
                related_batch_id,
                related_row_id,
                related_item_id
            ],
        )?;
        Ok(connection.last_insert_rowid())
    })
}

pub fn complete_task(database: &Database, task_id: i64, output_json: &str) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute(
            "UPDATE ai_tasks
             SET status = 'completed', output_json = ?2, error_message = NULL, updated_at = datetime('now')
             WHERE id = ?1",
            params![task_id, output_json],
        )?;
        Ok(())
    })
}

pub fn fail_task(database: &Database, task_id: i64, error_message: &str) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute(
            "UPDATE ai_tasks
             SET status = 'failed', error_message = ?2, updated_at = datetime('now')
             WHERE id = ?1",
            params![task_id, error_message],
        )?;
        Ok(())
    })
}

pub fn cancel_task(database: &Database, task_id: i64) -> AppResult<AiTask> {
    database.with_connection(|connection| {
        let changed = connection.execute(
            "UPDATE ai_tasks
             SET status = 'cancelled', updated_at = datetime('now')
             WHERE id = ?1 AND status IN ('pending', 'running')",
            params![task_id],
        )?;
        if changed == 0 {
            let task = get_task_by_connection(connection, task_id)?;
            if task.status != "cancelled" {
                return Err(AppError::InvalidInput(
                    "只能取消 pending 或 running 状态的 AI 任务".to_string(),
                ));
            }
            return Ok(task);
        }
        get_task_by_connection(connection, task_id)
    })
}

pub fn get_task(database: &Database, task_id: i64) -> AppResult<AiTask> {
    database.with_connection(|connection| get_task_by_connection(connection, task_id))
}

pub fn insert_call_log(
    database: &Database,
    provider_type: Option<&str>,
    model_name: Option<&str>,
    task_type: &str,
    request_summary: &str,
    response_summary: Option<&str>,
    duration_ms: i64,
    status: &str,
    error_message: Option<&str>,
) -> AppResult<()> {
    database.with_connection(|connection| {
        connection.execute(
            "INSERT INTO ai_call_logs
             (provider_type, model_name, task_type, input_hash, prompt_version,
              request_summary, response_summary, duration_ms, token_usage_json, status, error_message, created_at)
             VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?7, NULL, ?8, ?9, datetime('now'))",
            params![
                provider_type,
                model_name,
                task_type,
                stable_input_hash(request_summary),
                request_summary,
                response_summary,
                duration_ms,
                status,
                error_message,
            ],
        )?;
        Ok(())
    })
}

fn get_task_by_connection(connection: &rusqlite::Connection, task_id: i64) -> AppResult<AiTask> {
    connection
        .query_row(
            "SELECT id, task_type, status, input_json, output_json, error_message,
                    related_batch_id, related_row_id, related_item_id, created_at, updated_at
             FROM ai_tasks WHERE id = ?1",
            params![task_id],
            |row| {
                Ok(AiTask {
                    id: Some(row.get(0)?),
                    task_type: row.get(1)?,
                    status: row.get(2)?,
                    input_json: row.get(3)?,
                    output_json: row.get(4)?,
                    error_message: row.get(5)?,
                    related_batch_id: row.get(6)?,
                    related_row_id: row.get(7)?,
                    related_item_id: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )
        .optional()?
        .ok_or_else(|| AppError::InvalidInput(format!("AI 任务不存在: {task_id}")))
}

fn stable_input_hash(value: &str) -> String {
    let mut hash: u64 = 1469598103934665603;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1099511628211);
    }
    format!("{hash:016x}")
}
