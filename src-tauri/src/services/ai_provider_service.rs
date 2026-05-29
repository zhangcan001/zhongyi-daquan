use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::ai::{
    AiProviderSettings, AiProviderSettingsResponse, SaveAiProviderSettingsRequest,
    AI_DISABLED_MESSAGE,
};
use crate::repositories::ai_repository;

pub struct AiProviderService;

impl AiProviderService {
    pub fn get_settings(database: &Database) -> AppResult<AiProviderSettingsResponse> {
        Ok(AiProviderSettingsResponse {
            settings: ai_repository::get_provider_settings(database)?.unwrap_or_default(),
            message: AI_DISABLED_MESSAGE.to_string(),
        })
    }

    pub fn save_settings(
        database: &Database,
        request: SaveAiProviderSettingsRequest,
    ) -> AppResult<AiProviderSettingsResponse> {
        let settings = ai_repository::save_provider_settings(database, normalize_request(request))?;
        Ok(AiProviderSettingsResponse {
            settings,
            message: AI_DISABLED_MESSAGE.to_string(),
        })
    }
}

fn normalize_request(request: SaveAiProviderSettingsRequest) -> SaveAiProviderSettingsRequest {
    SaveAiProviderSettingsRequest {
        provider_type: empty_to_default(request.provider_type, "disabled"),
        provider_name: normalize_optional(request.provider_name),
        base_url: normalize_optional(request.base_url),
        model_name: normalize_optional(request.model_name),
        timeout_seconds: request.timeout_seconds,
        max_tokens: request.max_tokens,
        temperature: request.temperature,
    }
}

fn empty_to_default(value: String, default_value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default_value.to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|inner| {
        let trimmed = inner.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[allow(dead_code)]
pub fn default_settings() -> AiProviderSettings {
    AiProviderSettings::default()
}
