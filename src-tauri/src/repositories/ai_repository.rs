use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::ai::{AiProviderSettings, SaveAiProviderSettingsRequest};
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
