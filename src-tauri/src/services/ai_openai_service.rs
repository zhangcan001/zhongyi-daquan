use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::ai::{
    AiCitation, AiCommandResponse, AiContextItem, AiProviderSettings, AiTaskRequest,
};
use crate::repositories::{ai_repository, knowledge_repository};
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const SYSTEM_PROMPT: &str = "你是《中医大全》的本地中医资料助手。你可以基于用户主动提供的问题和本地检索片段做资料问答、条目总结、注解对比、经方组成提取、方剂候选检索、辨证参考和学习笔记草稿。必须引用本地资料来源，不得编造来源。资料不足时明确说明资料不足。";
const WEB_SYSTEM_PROMPT: &str = "你是《中医大全》的资料助手。你可以同时使用本地知识库片段和联网检索摘要回答。必须区分本地来源与网页来源，不得编造来源。网页资料只能作为外部参考；当本地资料和网页资料冲突时，要明确标出差异。资料不足时明确说明资料不足。";

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
    let task_id = ai_repository::create_task(
        database,
        task_type,
        request.input_json.as_deref(),
        request.related_batch_id,
        request.related_row_id,
        request.related_item_id,
    )?;
    let started_at = Instant::now();
    let result = (|| -> AppResult<AiCommandResponse> {
        let context = build_context(database, &settings, &question, request.related_item_id)?;
        let mut warnings = Vec::new();
        let web_sources = if settings.only_use_local_context {
            Vec::new()
        } else {
            match web_search(&question, settings.timeout_seconds) {
                Ok(sources) => sources,
                Err(error) => {
                    warnings.push(format!("联网检索失败，已自动降级为本地知识库回答：{error}"));
                    Vec::new()
                }
            }
        };
        let mut citations = context
            .iter()
            .map(|item| AiCitation {
                title: item.source_title.clone(),
                note: item.source_note.clone(),
            })
            .collect::<Vec<_>>();
        citations.extend(web_sources.iter().map(|source| AiCitation {
            title: Some(format!("网页：{}", source.title)),
            note: Some(source.url.clone()),
        }));
        let user_prompt = build_user_prompt(task_type, &question, &context, &web_sources);
        let system_prompt = if settings.only_use_local_context {
            SYSTEM_PROMPT
        } else {
            WEB_SYSTEM_PROMPT
        };
        let answer = call_chat(&settings, &api_key, system_prompt, &user_prompt)?;
        warnings.extend(task_warnings(
            task_type,
            settings.only_use_local_context,
            &web_sources,
        ));
        let mut response = response(
            true,
            "completed",
            if settings.only_use_local_context {
                "AI 回答已生成。"
            } else {
                "AI 回答已生成，已尝试联网检索。"
            },
            Some(answer.clone()),
            citations.clone(),
            context.clone(),
            warnings.clone(),
        );
        response.task_id = Some(task_id);
        let output_json = json!({
            "answer": answer,
            "citations": citations,
            "usedContextItems": context,
            "warnings": warnings,
        })
        .to_string();
        ai_repository::complete_task(database, task_id, &output_json)?;
        let response_summary = response
            .answer
            .as_deref()
            .map(|answer| truncate(answer, 500));
        ai_repository::insert_call_log(
            database,
            Some(settings.provider_type.as_str()),
            settings.model_name.as_deref(),
            task_type,
            &truncate(&question, 300),
            response_summary.as_deref(),
            started_at.elapsed().as_millis() as i64,
            "success",
            None,
        )?;
        Ok(response)
    })();

    match result {
        Ok(response) => Ok(response),
        Err(error) => {
            let message = error.to_string();
            let _ = ai_repository::fail_task(database, task_id, &message);
            let _ = ai_repository::insert_call_log(
                database,
                Some(settings.provider_type.as_str()),
                settings.model_name.as_deref(),
                task_type,
                &truncate(&question, 300),
                None,
                started_at.elapsed().as_millis() as i64,
                "failed",
                Some(&message),
            );
            Err(error)
        }
    }
}

pub fn get_task_status(database: &Database, task_id: i64) -> AppResult<AiCommandResponse> {
    let task = ai_repository::get_task(database, task_id)?;
    let output = task
        .output_json
        .as_deref()
        .and_then(|text| serde_json::from_str::<Value>(text).ok());
    let answer = output
        .as_ref()
        .and_then(|value| value.get("answer"))
        .and_then(Value::as_str)
        .map(str::to_string);
    Ok(AiCommandResponse {
        enabled: true,
        status: task.status.clone(),
        task_id: Some(task_id),
        message: task_status_message(&task.status, task.error_message.as_deref()),
        answer,
        citations: Vec::new(),
        used_context_items: Vec::new(),
        warnings: Vec::new(),
        safety_notice: None,
    })
}

pub fn cancel_task(database: &Database, task_id: i64) -> AppResult<AiCommandResponse> {
    let task = ai_repository::cancel_task(database, task_id)?;
    Ok(AiCommandResponse {
        enabled: true,
        status: task.status.clone(),
        task_id: Some(task_id),
        message: "AI 任务已取消。".to_string(),
        answer: None,
        citations: Vec::new(),
        used_context_items: Vec::new(),
        warnings: Vec::new(),
        safety_notice: None,
    })
}

fn task_status_message(status: &str, error_message: Option<&str>) -> String {
    match status {
        "completed" => "AI 任务已完成。".to_string(),
        "failed" => format!("AI 任务失败：{}", error_message.unwrap_or("未知错误")),
        "cancelled" => "AI 任务已取消。".to_string(),
        "running" => "AI 任务运行中。".to_string(),
        "pending" => "AI 任务等待中。".to_string(),
        other => format!("AI 任务状态：{other}"),
    }
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

fn build_user_prompt(
    task_type: &str,
    question: &str,
    context: &[AiContextItem],
    web_sources: &[WebSearchResult],
) -> String {
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
    let web_text = web_sources
        .iter()
        .enumerate()
        .map(|(index, source)| {
            format!(
                "[W{}] {}｜{}\n{}",
                index + 1,
                source.title,
                source.url,
                source.snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "任务类型：{task_type}\n用户问题：{question}\n\n本地检索片段：\n{context_text}\n\n联网检索摘要：\n{web_text}\n\n请基于上述资料回答，并在文末列出引用来源。若使用网页资料，需标注网页标题或 URL；资料不足时直接说明资料不足。"
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

#[derive(Debug, Clone)]
struct WebSearchResult {
    title: String,
    url: String,
    snippet: String,
}

fn web_search(question: &str, timeout_seconds: Option<i64>) -> AppResult<Vec<WebSearchResult>> {
    let query = question.trim();
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let timeout = timeout_seconds.unwrap_or(30).clamp(1, 8);
    for search in [
        bing_search as fn(&str, i64) -> AppResult<Vec<WebSearchResult>>,
        duckduckgo_lite_search,
        wikipedia_opensearch,
    ] {
        if let Ok(results) = search(query, timeout) {
            if !results.is_empty() {
                return Ok(results);
            }
        }
    }
    Ok(search_entry_results(query))
}

fn http_client(timeout_seconds: i64) -> AppResult<Client> {
    Client::builder()
        .timeout(Duration::from_secs(timeout_seconds as u64))
        .user_agent("Mozilla/5.0 zhongyi-daquan/0.1 ai-web-search")
        .build()
        .map_err(|_| AppError::Data("联网检索客户端初始化失败。".to_string()))
}

fn bing_search(query: &str, timeout_seconds: i64) -> AppResult<Vec<WebSearchResult>> {
    let url = format!("https://www.bing.com/search?q={}", percent_encode(query));
    let response = http_client(timeout_seconds)?
        .get(url)
        .send()
        .map_err(|_| AppError::Data("Bing 联网检索失败。".to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::Data(format!(
            "Bing 联网检索返回错误状态 {}。",
            response.status().as_u16()
        )));
    }
    let html = response
        .text()
        .map_err(|_| AppError::Data("Bing 联网检索结果读取失败。".to_string()))?;
    Ok(parse_bing_results(&html).into_iter().take(5).collect())
}

fn duckduckgo_lite_search(query: &str, timeout_seconds: i64) -> AppResult<Vec<WebSearchResult>> {
    let url = format!(
        "https://lite.duckduckgo.com/lite/?q={}",
        percent_encode(query)
    );
    let response = http_client(timeout_seconds)?
        .get(url)
        .send()
        .map_err(|_| AppError::Data("联网检索失败，请检查网络连接。".to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::Data(format!(
            "联网检索返回错误状态 {}。",
            response.status().as_u16()
        )));
    }
    let html = response
        .text()
        .map_err(|_| AppError::Data("联网检索结果读取失败。".to_string()))?;
    Ok(parse_duckduckgo_lite_results(&html)
        .into_iter()
        .take(5)
        .collect())
}

fn wikipedia_opensearch(query: &str, timeout_seconds: i64) -> AppResult<Vec<WebSearchResult>> {
    let url = format!(
        "https://zh.wikipedia.org/w/api.php?action=opensearch&search={}&limit=5&namespace=0&format=json",
        percent_encode(query)
    );
    let response = http_client(timeout_seconds)?
        .get(url)
        .send()
        .map_err(|_| AppError::Data("联网检索失败，请检查网络连接。".to_string()))?;
    if !response.status().is_success() {
        return Err(AppError::Data(format!(
            "联网检索返回错误状态 {}。",
            response.status().as_u16()
        )));
    }
    let value: Value = response
        .json()
        .map_err(|_| AppError::Data("联网检索结果解析失败。".to_string()))?;
    let titles = value
        .get(1)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let snippets = value
        .get(2)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let urls = value
        .get(3)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut results = Vec::new();
    for index in 0..titles.len().min(urls.len()) {
        let title = titles[index].as_str().unwrap_or_default().trim();
        let url = urls[index].as_str().unwrap_or_default().trim();
        if title.is_empty() || url.is_empty() {
            continue;
        }
        results.push(WebSearchResult {
            title: format!("维基百科：{}", truncate(title, 100)),
            url: url.to_string(),
            snippet: snippets
                .get(index)
                .and_then(Value::as_str)
                .map(|snippet| truncate(snippet.trim(), 500))
                .unwrap_or_default(),
        });
    }
    Ok(results)
}

fn parse_bing_results(html: &str) -> Vec<WebSearchResult> {
    let mut results = Vec::new();
    let mut cursor = 0;
    while let Some(card_offset) = html[cursor..].find("b_algo") {
        let card_start = cursor + card_offset;
        let card_end = html[card_start..]
            .find("</li>")
            .map(|offset| card_start + offset)
            .unwrap_or_else(|| (card_start + 4000).min(html.len()));
        let card = &html[card_start..card_end];
        let Some(href_offset) = card.find("href=\"") else {
            cursor = card_end;
            continue;
        };
        let href_start = href_offset + 6;
        let Some(href_end_offset) = card[href_start..].find('"') else {
            cursor = card_end;
            continue;
        };
        let url = html_unescape(&card[href_start..href_start + href_end_offset]);
        let title = card[href_start + href_end_offset..]
            .find('>')
            .and_then(|start_offset| {
                let start = href_start + href_end_offset + start_offset + 1;
                let end = card[start..].find("</a>").map(|offset| start + offset)?;
                Some(html_unescape(&strip_tags(&card[start..end])))
            })
            .unwrap_or_default();
        let snippet = card
            .find("<p")
            .and_then(|p_offset| {
                let start = card[p_offset..]
                    .find('>')
                    .map(|offset| p_offset + offset + 1)?;
                let end = card[start..].find("</p>").map(|offset| start + offset)?;
                Some(html_unescape(&strip_tags(&card[start..end])))
            })
            .unwrap_or_default();
        if !title.trim().is_empty()
            && url.starts_with("http")
            && !results
                .iter()
                .any(|existing: &WebSearchResult| existing.url == url)
        {
            results.push(WebSearchResult {
                title: truncate(title.trim(), 120),
                url,
                snippet: truncate(snippet.trim(), 500),
            });
        }
        cursor = card_end;
        if results.len() >= 8 {
            break;
        }
    }
    results
}

fn search_entry_results(query: &str) -> Vec<WebSearchResult> {
    let encoded = percent_encode(query);
    vec![
        WebSearchResult {
            title: format!("Bing 在线搜索：{query}"),
            url: format!("https://www.bing.com/search?q={encoded}"),
            snippet: "联网摘要源暂未返回可解析内容，已提供在线搜索入口供核对。".to_string(),
        },
        WebSearchResult {
            title: format!("搜狗在线搜索：{query}"),
            url: format!("https://www.sogou.com/web?query={encoded}"),
            snippet: "备用中文搜索入口。".to_string(),
        },
    ]
}

fn parse_duckduckgo_lite_results(html: &str) -> Vec<WebSearchResult> {
    let mut results = Vec::new();
    let mut cursor = 0;
    while let Some(link_offset) = html[cursor..].find("result-link") {
        let link_start = cursor + link_offset;
        let Some(href_offset) = html[link_start..].find("href=\"") else {
            break;
        };
        let href_start = link_start + href_offset + 6;
        let Some(href_end_offset) = html[href_start..].find('"') else {
            break;
        };
        let href = html_unescape(&html[href_start..href_start + href_end_offset]);
        let Some(title_start_offset) = html[href_start + href_end_offset..].find('>') else {
            break;
        };
        let title_start = href_start + href_end_offset + title_start_offset + 1;
        let Some(title_end_offset) = html[title_start..].find("</a>") else {
            break;
        };
        let title = html_unescape(&strip_tags(
            &html[title_start..title_start + title_end_offset],
        ));
        let next_cursor = title_start + title_end_offset;
        let snippet = html[next_cursor..]
            .find("result-snippet")
            .and_then(|snippet_offset| {
                let start = next_cursor + snippet_offset;
                let tag_end = html[start..].find('>')? + start + 1;
                let end = html[tag_end..].find("</").map(|offset| tag_end + offset)?;
                Some(html_unescape(&strip_tags(&html[tag_end..end])))
            })
            .unwrap_or_default();
        let url = clean_duckduckgo_url(&href);
        if !title.trim().is_empty()
            && !url.trim().is_empty()
            && !results
                .iter()
                .any(|existing: &WebSearchResult| existing.url == url)
        {
            results.push(WebSearchResult {
                title: truncate(title.trim(), 120),
                url,
                snippet: truncate(snippet.trim(), 500),
            });
        }
        cursor = next_cursor;
        if results.len() >= 8 {
            break;
        }
    }
    results
}

fn clean_duckduckgo_url(url: &str) -> String {
    if let Some(index) = url.find("uddg=") {
        let tail = &url[index + 5..];
        let end = tail.find('&').unwrap_or(tail.len());
        return percent_decode(&tail[..end]);
    }
    url.to_string()
}

fn percent_encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~') {
            output.push(*byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn percent_decode(value: &str) -> String {
    let mut bytes = Vec::new();
    let raw = value.as_bytes();
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == b'%' && index + 2 < raw.len() {
            if let Ok(hex) = u8::from_str_radix(&value[index + 1..index + 3], 16) {
                bytes.push(hex);
                index += 3;
                continue;
            }
        }
        bytes.push(raw[index]);
        index += 1;
    }
    String::from_utf8_lossy(&bytes).to_string()
}

fn strip_tags(value: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for ch in value.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output
}

fn html_unescape(value: &str) -> String {
    value
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ")
}

fn task_warnings(
    task_type: &str,
    only_use_local_context: bool,
    web_sources: &[WebSearchResult],
) -> Vec<String> {
    let _ = task_type;
    if only_use_local_context {
        return Vec::new();
    }
    if web_sources.is_empty() {
        vec!["联网检索未返回可用摘要，本次回答主要依赖本地资料。".to_string()]
    } else if web_sources.iter().all(|source| {
        source.snippet.contains("暂未返回可解析内容") || source.snippet.contains("备用中文搜索入口")
    }) {
        vec!["联网检索未返回可解析摘要，已提供在线搜索入口供核对。".to_string()]
    } else {
        vec!["已启用联网检索，网页资料会作为外部来源与本地资料分开引用。".to_string()]
    }
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
    fn ai_task_status_reads_real_task_table_and_cancel_updates_status() {
        let data_dir = temp_data_dir("ai-task-status");
        let database = Database::initialize(&data_dir).expect("database");
        let completed_id = ai_repository::create_task(
            &database,
            "local_qa",
            Some("{\"question\":\"桂枝汤\"}"),
            None,
            None,
            None,
        )
        .expect("create task");
        ai_repository::complete_task(
            &database,
            completed_id,
            r#"{"answer":"桂枝汤资料回答","warnings":[]}"#,
        )
        .expect("complete task");

        let status = get_task_status(&database, completed_id).expect("task status");
        assert_eq!(status.status, "completed");
        assert_eq!(status.task_id, Some(completed_id));
        assert_eq!(status.answer.as_deref(), Some("桂枝汤资料回答"));

        let running_id = ai_repository::create_task(&database, "local_qa", None, None, None, None)
            .expect("create running task");
        let cancelled = cancel_task(&database, running_id).expect("cancel task");
        assert_eq!(cancelled.status, "cancelled");
        let cancelled_status = get_task_status(&database, running_id).expect("cancelled status");
        assert_eq!(cancelled_status.status, "cancelled");

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
        let prompt = build_user_prompt("local_qa", "桂枝汤组成是什么？", &context, &[]);
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
        let prompt = build_user_prompt("local_qa", "桂枝汤什么组成", &context, &[]);
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

    #[test]
    fn web_search_parser_extracts_title_url_and_snippet() {
        let html = r#"
            <a rel="nofollow" class="result-link" href="/l/?kh=-1&amp;uddg=https%3A%2F%2Fexample.com%2Fpage%3Fa%3D1">Example &amp; Title</a>
            <td class="result-snippet">A useful &lt;b&gt;summary&lt;/b&gt; from the web.</td>
        "#;
        let results = parse_duckduckgo_lite_results(html);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Example & Title");
        assert_eq!(results[0].url, "https://example.com/page?a=1");
        assert!(results[0].snippet.contains("summary"));
    }

    #[test]
    fn bing_parser_extracts_result_cards() {
        let html = r#"
          <li class="b_algo">
            <h2><a href="https://example.com/tcm">栝篓桂枝汤方 - 示例</a></h2>
            <p>栝篓桂枝汤方相关资料摘要。</p>
          </li>
        "#;
        let results = parse_bing_results(html);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url, "https://example.com/tcm");
        assert!(results[0].title.contains("栝篓桂枝汤方"));
        assert!(results[0].snippet.contains("相关资料摘要"));
    }

    fn temp_data_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("zhongyi-daquan-{test_name}-{unique}"))
    }
}
