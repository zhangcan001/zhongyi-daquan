use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::ai::{
    AiCitation, AiCommandResponse, AiContextItem, AiProviderSettings, AiTaskRequest,
};
use crate::repositories::{ai_repository, knowledge_repository};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::time::Duration;

const SYSTEM_PROMPT: &str = "你是《中医大全》的本地中医资料助手。你可以基于用户主动提供的问题和本地检索片段做资料问答、条目总结、注解对比、经方组成提取、方剂候选检索、辨证参考和学习笔记草稿。必须引用本地资料来源，不得编造来源。资料不足时明确说明资料不足。";

pub fn test_connection(database: &Database) -> AppResult<AiCommandResponse> {
    let (settings, api_key) = match load_ready_settings(database) {
        Ok(value) => value,
        Err(error) => {
            return Ok(response(
                false,
                "not_configured",
                &error.to_string(),
                None,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ));
        }
    };
    let answer = call_chat(
        &settings,
        &api_key,
        "你是连接测试助手，只需回复 OK。",
        "请回复 OK。",
    )?;
    if answer.trim().is_empty() {
        return Ok(response(
            true,
            "error",
            "连接失败：返回格式异常。",
            None,
            Vec::new(),
            Vec::new(),
            vec!["模型返回为空。".to_string()],
        ));
    }
    Ok(response(
        true,
        "ok",
        "连接成功，模型返回正常。",
        Some(answer),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    ))
}

pub fn run_task(database: &Database, request: AiTaskRequest) -> AppResult<AiCommandResponse> {
    if !ai_repository::ai_enabled(database)? {
        return Ok(response(
            false,
            "disabled",
            "AI 未启用，请先在 AI 设置页配置 provider、base_url、model 和 API Key。",
            None,
            Vec::new(),
            Vec::new(),
            Vec::new(),
        ));
    }
    let (settings, api_key) = load_ready_settings(database)?;
    let task_type = request.task_type.trim();
    let question = extract_question(&request);
    let context = build_context(database, &settings, &question, request.related_item_id)?;
    let citations = context
        .iter()
        .map(|item| AiCitation {
            title: item.source_title.clone(),
            note: item.source_note.clone(),
        })
        .collect::<Vec<_>>();
    let user_prompt = build_user_prompt(task_type, &question, &context);
    let answer = call_chat(&settings, &api_key, SYSTEM_PROMPT, &user_prompt)?;
    Ok(response(
        true,
        "completed",
        "AI 回答已生成。",
        Some(answer.clone()),
        citations,
        context,
        safety_warnings(task_type),
    ))
}

fn load_ready_settings(database: &Database) -> AppResult<(AiProviderSettings, String)> {
    let settings = ai_repository::get_provider_settings(database)?.unwrap_or_default();
    if !settings.enabled || settings.provider_type == "disabled" {
        return Err(AppError::InvalidInput(
            "AI 未启用，请先在 AI 设置页开启并保存配置。".to_string(),
        ));
    }
    if settings.provider_type != "openai_compatible" {
        return Err(AppError::InvalidInput(
            "当前仅支持 openai_compatible provider。".to_string(),
        ));
    }
    if settings
        .base_url
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err(AppError::InvalidInput("请先配置 Base URL。".to_string()));
    }
    if settings
        .model_name
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return Err(AppError::InvalidInput("请先配置 Model Name。".to_string()));
    }
    let Some(api_key) = ai_repository::get_api_key(database)? else {
        return Err(AppError::InvalidInput("请先配置 API Key。".to_string()));
    };
    if api_key.trim().is_empty() {
        return Err(AppError::InvalidInput("请先配置 API Key。".to_string()));
    }
    Ok((settings, api_key))
}

fn call_chat(
    settings: &AiProviderSettings,
    api_key: &str,
    system_prompt: &str,
    user_prompt: &str,
) -> AppResult<String> {
    let base_url = settings.base_url.as_deref().unwrap_or_default().trim();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let client = Client::builder()
        .timeout(Duration::from_secs(
            settings.timeout_seconds.unwrap_or(30).clamp(1, 300) as u64,
        ))
        .build()
        .map_err(|_| AppError::Data("AI 客户端初始化失败。".to_string()))?;
    let result = client
        .post(url)
        .bearer_auth(api_key)
        .json(&json!({
            "model": settings.model_name,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ],
            "temperature": settings.temperature.unwrap_or(0.2),
            "max_tokens": settings.max_tokens.unwrap_or(1200)
        }))
        .send();
    let response = match result {
        Ok(response) => response,
        Err(error) if error.is_timeout() => {
            return Err(AppError::Data(
                "AI 请求超时，请检查网络或调大 timeout。".to_string(),
            ));
        }
        Err(_) => {
            return Err(AppError::Data(
                "AI 网络请求失败，请检查 Base URL、网络或服务状态。".to_string(),
            ));
        }
    };
    let status = response.status();
    let body = response
        .text()
        .map_err(|_| AppError::Data("AI 返回读取失败。".to_string()))?;
    if status.as_u16() == 401 || status.as_u16() == 403 {
        return Err(AppError::Data("AI 认证失败，请检查 API Key。".to_string()));
    }
    if status.as_u16() == 404 {
        return Err(AppError::Data(
            "AI 接口或模型不存在，请检查 Base URL 和 Model。".to_string(),
        ));
    }
    if !status.is_success() {
        return Err(AppError::Data(format!(
            "AI 服务返回错误状态 {}，详情已脱敏。",
            status.as_u16()
        )));
    }
    let value: Value =
        serde_json::from_str(&body).map_err(|_| AppError::Data("AI 返回格式异常。".to_string()))?;
    value
        .get("choices")
        .and_then(|choices| choices.get(0))
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| AppError::Data("AI 返回格式异常。".to_string()))
}

fn build_context(
    database: &Database,
    settings: &AiProviderSettings,
    question: &str,
    related_item_id: Option<i64>,
) -> AppResult<Vec<AiContextItem>> {
    let max_items = settings.max_context_items.unwrap_or(6).clamp(1, 20) as usize;
    let max_chars = settings
        .max_context_chars
        .unwrap_or(6000)
        .clamp(500, 30_000) as usize;
    let mut items = Vec::new();
    let intent = SearchIntent::from_question(question);
    if let Some(item_id) = related_item_id {
        database.with_connection(|connection| {
            if let Some(item) = knowledge_repository::get_by_id(connection, item_id)? {
                if intent.allows_type(&item.item_type) {
                    items.push(AiContextItem {
                        item_id,
                        item_type: item.item_type,
                        name: item.name,
                        source_title: item.source_package,
                        source_note: item.source_note,
                        snippet: truncate(
                            &[item.summary, item.content, item.tags]
                                .into_iter()
                                .flatten()
                                .collect::<Vec<_>>()
                                .join("\n"),
                            900,
                        ),
                    });
                }
            }
            Ok(())
        })?;
    }
    database.with_connection(|connection| {
        for term in query_terms(question) {
            let like = format!("%{}%", term);
            let mut statement = connection.prepare(
                "SELECT id, type, name, summary, content, source_note, tags, source_package, detail
                 FROM knowledge_items
                 WHERE name LIKE ?1 OR summary LIKE ?1 OR content LIKE ?1 OR source_note LIKE ?1 OR tags LIKE ?1 OR detail LIKE ?1
                 ORDER BY CASE WHEN type = 'formula' THEN 0 ELSE 1 END, updated_at DESC
                 LIMIT ?2",
            )?;
            let rows =
                statement.query_map(rusqlite::params![like, (max_items * 3) as i64], |row| {
                    Ok(AiContextItem {
                        item_id: row.get(0)?,
                        item_type: row.get(1)?,
                        name: row.get(2)?,
                        source_title: row.get(7)?,
                        source_note: row.get(5)?,
                        snippet: truncate(
                            &[
                                row.get::<_, Option<String>>(3)?,
                                row.get::<_, Option<String>>(4)?,
                                row.get::<_, Option<String>>(6)?,
                                row.get::<_, Option<String>>(8)?,
                            ]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                            .join("\n"),
                            1200,
                        ),
                    })
                })?;
            for item in rows {
                let item = item?;
                if intent.allows_type(&item.item_type)
                    && !items.iter().any(|existing| existing.item_id == item.item_id)
                {
                    items.push(item);
                }
            }
        }
        Ok(())
    })?;
    let mut total = 0;
    let mut clipped = Vec::new();
    for mut item in items.into_iter().take(max_items) {
        if total >= max_chars {
            break;
        }
        let remaining = max_chars - total;
        item.snippet = truncate(&item.snippet, remaining.min(900));
        total += item.snippet.chars().count();
        clipped.push(item);
    }
    Ok(clipped)
}

fn build_user_prompt(task_type: &str, question: &str, context: &[AiContextItem]) -> String {
    let context_text = context
        .iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "[{}] {}｜{}｜来源：{} {}\n{}",
                index + 1,
                item.name,
                item.item_type,
                item.source_title.as_deref().unwrap_or("未记录"),
                item.source_note.as_deref().unwrap_or(""),
                item.snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "任务类型：{task_type}\n用户问题：{question}\n\n本地检索片段：\n{context_text}\n\n请基于上述本地片段回答，并列出引用来源。资料不足时直接说明资料不足。"
    )
}

fn extract_question(request: &AiTaskRequest) -> String {
    request
        .input_json
        .as_deref()
        .and_then(|text| serde_json::from_str::<Value>(text).ok())
        .and_then(|value| {
            value
                .get("question")
                .or_else(|| value.get("prompt"))
                .or_else(|| value.get("text"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| request.input_json.clone().unwrap_or_default())
}

fn query_terms(question: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let full = question.trim();
    for formula_name in formula_names_from_text(full) {
        if !terms.iter().any(|term| term == &formula_name) {
            terms.push(formula_name);
        }
    }
    for marker in [
        "太阳病",
        "少阳病",
        "阳明病",
        "太阴病",
        "少阴病",
        "厥阴病",
        "上热下寒",
        "长期咳嗽",
        "久咳",
        "咳嗽",
    ] {
        if full.contains(marker) && !terms.iter().any(|term| term == marker) {
            terms.push(marker.to_string());
        }
    }
    for token in full.split(|ch: char| {
        ch.is_whitespace()
            || matches!(
                ch,
                '，' | '。' | '、' | '；' | ';' | ':' | '：' | '？' | '?' | '！' | '!'
            )
    }) {
        let token = token
            .replace("如果", "")
            .replace("请问", "")
            .replace("应该", "")
            .replace("可以", "")
            .replace("什么药", "")
            .replace("用药", "")
            .replace("用什么", "")
            .replace("怎么治", "")
            .replace("治疗", "")
            .replace("长期", "")
            .trim_matches(|ch: char| {
                matches!(
                    ch,
                    '有' | '哪'
                        | '些'
                        | '可'
                        | '以'
                        | '参'
                        | '考'
                        | '什'
                        | '么'
                        | '方'
                        | '向'
                        | '的'
                        | '组'
                        | '成'
                        | '用'
                        | '药'
                        | '治'
                )
            })
            .trim()
            .to_string();
        if token.chars().count() >= 2 && !terms.iter().any(|term| term == &token) {
            terms.push(token);
        }
    }
    if !full.is_empty() && terms.is_empty() {
        terms.push(full.to_string());
    }
    terms
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchIntent {
    MedicineOrFormula,
    Acupuncture,
    General,
}

impl SearchIntent {
    fn from_question(question: &str) -> Self {
        if question.contains("针")
            || question.contains("灸")
            || question.contains("穴")
            || question.contains("经络")
        {
            Self::Acupuncture
        } else if question.contains("药")
            || question.contains("方")
            || question.contains("经方")
            || question.contains("咳嗽")
            || question.contains("治疗")
            || question.contains("怎么治")
        {
            Self::MedicineOrFormula
        } else {
            Self::General
        }
    }

    fn allows_type(self, item_type: &str) -> bool {
        match self {
            Self::MedicineOrFormula => {
                !matches!(item_type, "acupuncture" | "acupoint" | "meridian")
            }
            Self::Acupuncture | Self::General => true,
        }
    }
}

fn formula_names_from_text(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let chars = text.chars().collect::<Vec<_>>();
    for (index, ch) in chars.iter().enumerate() {
        if matches!(ch, '汤' | '丸' | '散' | '方' | '饮' | '剂') {
            let start = index.saturating_sub(8);
            let window = &chars[start..=index];
            let split = window.iter().rposition(|c| {
                c.is_whitespace()
                    || matches!(
                        *c,
                        '，' | '。'
                            | '、'
                            | '；'
                            | ';'
                            | ':'
                            | '：'
                            | '“'
                            | '”'
                            | '"'
                            | '\''
                            | '？'
                            | '?'
                            | '！'
                            | '!'
                    )
            });
            let name_start = split.map(|idx| idx + 1).unwrap_or(0);
            let name = window[name_start..].iter().collect::<String>();
            let name = name.trim().to_string();
            if name.chars().count() >= 2 && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

fn safety_warnings(task_type: &str) -> Vec<String> {
    let _ = task_type;
    Vec::new()
}

fn response(
    enabled: bool,
    status: &str,
    message: &str,
    answer: Option<String>,
    citations: Vec<AiCitation>,
    used_context_items: Vec<AiContextItem>,
    warnings: Vec<String>,
) -> AiCommandResponse {
    AiCommandResponse {
        enabled,
        status: status.to_string(),
        task_id: None,
        message: message.to_string(),
        answer,
        citations,
        used_context_items,
        warnings,
        safety_notice: None,
    }
}

fn truncate(value: &str, limit: usize) -> String {
    let mut output = value.chars().take(limit).collect::<String>();
    if value.chars().count() > limit {
        output.push_str("...");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::ai_provider_service::AiProviderService;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn settings_do_not_return_key_and_can_clear_key() {
        let data_dir = temp_data_dir("ai-key");
        let database = Database::initialize(&data_dir).expect("database");
        let saved = AiProviderService::save_settings(
            &database,
            crate::models::ai::SaveAiProviderSettingsRequest {
                provider_type: "openai_compatible".to_string(),
                provider_name: Some("test".to_string()),
                base_url: Some("http://localhost:9999/v1".to_string()),
                api_key: Some("test-secret-value".to_string()),
                model_name: Some("test-model".to_string()),
                timeout_seconds: Some(3),
                max_tokens: Some(32),
                temperature: Some(0.1),
                max_context_items: Some(2),
                max_context_chars: Some(1000),
                only_use_local_context: Some(true),
                safety_mode: Some("strict".to_string()),
                enabled: Some(true),
            },
        )
        .expect("save");
        assert!(saved.settings.has_api_key);
        assert!(!serde_json::to_string(&saved)
            .unwrap()
            .contains("test-secret-value"));
        let cleared = AiProviderService::clear_api_key(&database).expect("clear");
        assert!(!cleared.settings.has_api_key);
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn disabled_and_missing_key_do_not_call_network() {
        let data_dir = temp_data_dir("ai-disabled");
        let database = Database::initialize(&data_dir).expect("database");
        let disabled = run_task(
            &database,
            AiTaskRequest {
                task_type: "local_qa".to_string(),
                input_json: Some("{\"question\":\"桂枝汤\"}".to_string()),
                related_batch_id: None,
                related_row_id: None,
                related_item_id: None,
            },
        )
        .expect("disabled response");
        assert!(!disabled.enabled);
        assert!(disabled.message.contains("AI 未启用"));
        let missing = test_connection(&database).expect("friendly missing settings response");
        assert!(!missing.enabled);
        assert!(missing.message.contains("AI 未启用") || missing.message.contains("API Key"));
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn rag_context_is_limited_and_contains_sources() {
        let data_dir = temp_data_dir("ai-rag");
        let database = Database::initialize(&data_dir).expect("database");
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, summary, content, source_note, tags, data_status, completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES ('formula', '桂枝汤', '太阳病方剂', '桂枝汤组成：桂枝三两 芍药三两 甘草二两 生姜三两 大枣十二枚。', '4人纪-伤寒论.pdf｜PDF页码12', '桂枝汤,太阳病', 'imported', 'complete', 1, 0, '{}', datetime('now'), datetime('now'))",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let settings = AiProviderSettings {
            max_context_items: Some(1),
            max_context_chars: Some(80),
            ..AiProviderSettings::default()
        };
        let context = build_context(&database, &settings, "桂枝汤", None).expect("context");
        assert_eq!(context.len(), 1);
        assert!(context[0]
            .source_note
            .as_deref()
            .unwrap_or_default()
            .contains("PDF页码12"));
        assert!(context[0].snippet.chars().count() <= 83);
        let prompt = build_user_prompt("local_qa", "桂枝汤组成是什么？", &context);
        assert!(prompt.contains("来源"));
        assert!(prompt.contains("桂枝汤"));
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn rag_context_uses_formula_name_when_question_contains_extra_words() {
        let data_dir = temp_data_dir("ai-rag-formula-term");
        let database = Database::initialize(&data_dir).expect("database");
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, summary, content, source_note, tags, data_status, completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES ('formula', '桂枝汤', '太阳病方剂', '原方组成：桂枝三两 芍药三两 甘草二两 生姜三两 大枣十二枚。', '4人纪-伤寒论.pdf｜PDF页码12', '桂枝汤,太阳病', 'imported', 'complete', 1, 0, '{}', datetime('now'), datetime('now'))",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let settings = AiProviderSettings {
            max_context_items: Some(3),
            max_context_chars: Some(1200),
            ..AiProviderSettings::default()
        };
        let context = build_context(&database, &settings, "桂枝汤什么组成", None).expect("context");
        assert!(context
            .iter()
            .any(|item| { item.name == "桂枝汤" && item.snippet.contains("桂枝三两") }));
        let prompt = build_user_prompt("local_qa", "桂枝汤什么组成", &context);
        assert!(prompt.contains("原方组成"));
        assert!(prompt.contains("4人纪-伤寒论.pdf"));
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn medicine_question_excludes_acupuncture_context_and_uses_cough_terms() {
        let data_dir = temp_data_dir("ai-rag-cough-medicine");
        let database = Database::initialize(&data_dir).expect("database");
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, summary, content, source_note, tags, data_status, completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES ('acupuncture', '怀孕忌针、安胎堕胎与胎位不正', '孕期针灸禁忌', '用药二字只作测试噪声；本文不涉及长期咳嗽。', '针灸讲义｜PDF页码20', '针灸,孕期', 'imported', 'complete', 1, 0, '{}', datetime('now'), datetime('now'))",
                    [],
                )?;
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, summary, content, source_note, tags, data_status, completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES ('formula', '咳嗽参考方', '长期咳嗽资料片段', '本地资料记录：咳嗽相关方剂候选，需辨证参考，不作个人处方。', '本地讲义｜PDF页码55', '咳嗽,方剂', 'imported', 'complete', 1, 0, '{}', datetime('now'), datetime('now'))",
                    [],
                )?;
                Ok(())
            })
            .unwrap();
        let settings = AiProviderSettings {
            max_context_items: Some(5),
            max_context_chars: Some(2000),
            ..AiProviderSettings::default()
        };
        let context =
            build_context(&database, &settings, "如果长期咳嗽，用什么药", None).expect("context");
        assert!(context.iter().any(|item| item.name == "咳嗽参考方"));
        assert!(!context
            .iter()
            .any(|item| item.name.contains("怀孕忌针") || item.item_type == "acupuncture"));
        let _ = fs::remove_dir_all(data_dir);
    }

    fn temp_data_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("zhongyi-daquan-{test_name}-{unique}"))
    }
}
