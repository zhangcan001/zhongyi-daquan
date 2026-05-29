use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::ai::{AiProviderSettings, SaveAiProviderSettingsRequest};
use rusqlite::{params, OptionalExtension};

pub fn ai_enabled(database: &Database) -> AppResult<bool> {
    let _ = database;
    Ok(false)
}

pub fn get_provider_settings(database: &Database) -> AppResult<Option<AiProviderSettings>> {
    database.with_connection(|connection| {
        connection
            .query_row(
                "SELECT id, provider_type, provider_name, base_url, model_name,
                    timeout_seconds, max_tokens, temperature, enabled, created_at, updated_at
                 FROM ai_provider_settings
                 ORDER BY id DESC
                 LIMIT 1",
                [],
                |row| {
                    let enabled: i64 = row.get(8)?;
                    Ok(AiProviderSettings {
                        id: row.get(0)?,
                        provider_type: row.get(1)?,
                        provider_name: row.get(2)?,
                        base_url: row.get(3)?,
                        model_name: row.get(4)?,
                        timeout_seconds: row.get(5)?,
                        max_tokens: row.get(6)?,
                        temperature: row.get(7)?,
                        enabled: enabled == 1,
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
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
        connection.execute("DELETE FROM ai_provider_settings", [])?;
        connection.execute(
            "INSERT INTO ai_provider_settings (
                provider_type, provider_name, base_url, api_key_encrypted, model_name,
                timeout_seconds, max_tokens, temperature, enabled, created_at, updated_at
            )
            VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, 0, datetime('now'), datetime('now'))",
            params![
                request.provider_type,
                request.provider_name,
                request.base_url,
                request.model_name,
                request.timeout_seconds,
                request.max_tokens,
                request.temperature,
            ],
        )?;

        let id = connection.last_insert_rowid();
        let settings = connection.query_row(
            "SELECT id, provider_type, provider_name, base_url, model_name,
                timeout_seconds, max_tokens, temperature, enabled, created_at, updated_at
             FROM ai_provider_settings
             WHERE id = ?1",
            params![id],
            |row| {
                let enabled: i64 = row.get(8)?;
                Ok(AiProviderSettings {
                    id: row.get(0)?,
                    provider_type: row.get(1)?,
                    provider_name: row.get(2)?,
                    base_url: row.get(3)?,
                    model_name: row.get(4)?,
                    timeout_seconds: row.get(5)?,
                    max_tokens: row.get(6)?,
                    temperature: row.get(7)?,
                    enabled: enabled == 1,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            },
        )?;

        Ok(settings)
    })
}
