use crate::db::connection::Database;
use crate::errors::AppResult;
use crate::models::ai::{
    AiProviderSettings, AiProviderSettingsResponse, SaveAiProviderSettingsRequest,
};
use crate::repositories::ai_repository;

pub struct AiProviderService;

impl AiProviderService {
    pub fn get_settings(database: &Database) -> AppResult<AiProviderSettingsResponse> {
        Ok(AiProviderSettingsResponse {
            settings: ai_repository::get_provider_settings(database)?.unwrap_or_default(),
            message: "AI 设置已读取，API Key 不会回显。".to_string(),
        })
    }

    pub fn save_settings(
        database: &Database,
        request: SaveAiProviderSettingsRequest,
    ) -> AppResult<AiProviderSettingsResponse> {
        let settings = ai_repository::save_provider_settings(database, normalize_request(request))?;
        let message = if settings.has_api_key {
            "AI 设置已保存，API Key 已配置。".to_string()
        } else {
            "AI 设置已保存，API Key 未配置。".to_string()
        };
        Ok(AiProviderSettingsResponse { settings, message })
    }

    pub fn clear_api_key(database: &Database) -> AppResult<AiProviderSettingsResponse> {
        Ok(AiProviderSettingsResponse {
            settings: ai_repository::clear_api_key(database)?.unwrap_or_default(),
            message: "API Key 已清除。".to_string(),
        })
    }
}

fn normalize_request(request: SaveAiProviderSettingsRequest) -> SaveAiProviderSettingsRequest {
    SaveAiProviderSettingsRequest {
        provider_type: empty_to_default(request.provider_type, "disabled"),
        provider_name: normalize_optional(request.provider_name),
        base_url: normalize_optional(request.base_url),
        api_key: normalize_optional(request.api_key),
        model_name: normalize_optional(request.model_name),
        timeout_seconds: request.timeout_seconds.or(Some(30)),
        max_tokens: request.max_tokens.or(Some(1200)),
        temperature: request.temperature.or(Some(0.2)),
        max_context_items: request.max_context_items.or(Some(6)),
        max_context_chars: request.max_context_chars.or(Some(6000)),
        only_use_local_context: Some(request.only_use_local_context.unwrap_or(true)),
        safety_mode: normalize_optional(request.safety_mode).or(Some("strict".to_string())),
        enabled: request.enabled,
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
