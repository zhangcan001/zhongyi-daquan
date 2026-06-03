use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::search::{
    EnhancedSearchGroup, EnhancedSearchRequest, EnhancedSearchResponse, EnhancedSearchResult,
    KnowledgeSearchResult, ListCacheRequest, ListCacheResponse, RebuildSearchIndexResponse,
    SearchRequest, SearchResponse, SearchSeedOptions, SearchSeedResponse,
};
use crate::repositories::{performance_repository, search_repository};
use rusqlite::{params, OptionalExtension};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub fn rebuild_search_index(database: &Database) -> AppResult<RebuildSearchIndexResponse> {
    let started_at = Instant::now();
    let (indexed_items, search_terms) =
        search_repository::rebuild_all(database, search_repository::build_terms_from_item)?;
    let duration_ms = started_at.elapsed().as_millis() as i64;
    performance_repository::record(
        database,
        "rebuild_search_index",
        duration_ms,
        Some(indexed_items),
        Some("maintenance"),
    )?;
    Ok(RebuildSearchIndexResponse {
        indexed_items,
        search_terms,
        duration_ms,
    })
}

#[allow(dead_code)]
pub fn index_knowledge_item(database: &Database, item_id: i64) -> AppResult<i64> {
    search_repository::index_item(database, item_id, search_repository::build_terms_from_item)
}

#[allow(dead_code)]
pub fn delete_knowledge_item_index(database: &Database, item_id: i64) -> AppResult<()> {
    search_repository::delete_item_index(database, item_id)
}

#[allow(dead_code)]
pub fn rebuild_relation_count_cache(database: &Database) -> AppResult<()> {
    search_repository::rebuild_relation_count_cache(database)
}

pub fn search(database: &Database, request: SearchRequest) -> AppResult<SearchResponse> {
    let started_at = Instant::now();
    let query = request.query.trim().to_string();
    if query.is_empty() {
        return Err(AppError::InvalidInput("搜索关键词不能为空".to_string()));
    }

    let page = request.page.unwrap_or(1).max(1);
    let page_size = normalize_page_size(request.page_size);
    let candidate_limit = page.saturating_mul(page_size).saturating_add(50).min(500);
    let candidates = search_repository::search_candidates(
        database,
        &query,
        request.item_type.as_deref(),
        candidate_limit,
    )?;
    let total = candidates.len();
    let offset = ((page - 1) * page_size) as usize;
    let page_candidates = candidates
        .iter()
        .skip(offset)
        .take(page_size as usize)
        .cloned()
        .collect::<Vec<_>>();
    let results = search_repository::hydrate_results(database, &page_candidates)?;
    let duration_ms = started_at.elapsed().as_millis() as i64;

    performance_repository::record(
        database,
        "search_knowledge",
        duration_ms,
        Some(results.len() as i64),
        Some(&query),
    )?;

    Ok(SearchResponse {
        query,
        total,
        page,
        page_size,
        duration_ms,
        results,
    })
}

pub fn search_enhanced(
    database: &Database,
    request: EnhancedSearchRequest,
) -> AppResult<EnhancedSearchResponse> {
    let started_at = Instant::now();
    let query = request.query.trim().to_string();
    if query.is_empty() {
        return Err(AppError::InvalidInput("搜索关键词不能为空".to_string()));
    }
    let filter = request.filter.unwrap_or_else(|| "全部".to_string());
    let page = request.page.unwrap_or(1).max(1);
    let page_size = normalize_page_size(request.page_size);

    let basic = search(
        database,
        SearchRequest {
            query: query.clone(),
            item_type: None,
            page: Some(1),
            page_size: Some(200),
        },
    )?;

    let mut seen = HashSet::new();
    let mut candidates = Vec::new();
    for result in basic.results {
        seen.insert(result.item_id);
        candidates.push((result.item_id, result.score, result.matched_by));
    }

    for item_id in annotation_hit_item_ids(database, &query)? {
        if seen.insert(item_id) {
            candidates.push((item_id, 65, "annotation".to_string()));
        }
    }

    let mut results = Vec::new();
    for (item_id, score, matched_by) in candidates {
        if let Some(mut result) =
            load_enhanced_result(database, item_id, &query, score, matched_by)?
        {
            if filter_allows(&filter, &result) {
                if result.matched_by.contains("annotation") && result.annotation_snippet.is_none() {
                    result.annotation_snippet =
                        first_annotation_snippet(database, item_id, &query)?;
                }
                results.push(result);
            }
        }
    }
    results.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.name.cmp(&b.name)));

    let total = results.len();
    let offset = ((page - 1) * page_size) as usize;
    let paged = results
        .into_iter()
        .skip(offset)
        .take(page_size as usize)
        .collect::<Vec<_>>();
    let groups = group_results(paged);
    let duration_ms = started_at.elapsed().as_millis() as i64;

    performance_repository::record(
        database,
        "search_knowledge_enhanced",
        duration_ms,
        Some(total as i64),
        Some(&query),
    )?;

    Ok(EnhancedSearchResponse {
        query,
        filter,
        total,
        page,
        page_size,
        duration_ms,
        groups,
    })
}

pub fn list_cache(database: &Database, request: ListCacheRequest) -> AppResult<ListCacheResponse> {
    let started_at = Instant::now();
    let page = request.page.unwrap_or(1).max(1);
    let page_size = normalize_page_size(request.page_size);
    let (total, results) =
        search_repository::list_cache(database, request.item_type.as_deref(), page, page_size)?;
    let duration_ms = started_at.elapsed().as_millis() as i64;
    performance_repository::record(
        database,
        "list_knowledge_cache",
        duration_ms,
        Some(results.len() as i64),
        request.item_type.as_deref(),
    )?;

    Ok(ListCacheResponse {
        total,
        page,
        page_size,
        duration_ms,
        results,
    })
}

pub fn generate_performance_test_data(
    database: &Database,
    options: SearchSeedOptions,
) -> AppResult<SearchSeedResponse> {
    let item_count = options.item_count.unwrap_or(10_000).clamp(1, 100_000);
    let relation_count = options.relation_count.unwrap_or(50_000).min(500_000);
    let reset_existing = options.reset_existing.unwrap_or(false);
    let response = search_repository::generate_seed_data(
        database,
        item_count,
        relation_count,
        reset_existing,
        search_repository::build_terms_from_item,
    )?;
    performance_repository::record(
        database,
        "generate_search_performance_test_data",
        response.duration_ms,
        Some(item_count as i64),
        Some("seed"),
    )?;
    Ok(response)
}

pub fn smoke_test_searches(database: &Database) -> AppResult<Vec<KnowledgeSearchResult>> {
    let queries = [
        "足三里",
        "ST36",
        "st36",
        "zusanli",
        "胃经",
        "足阳明胃经",
        "黄芪",
        "黄耆",
        "补中益气汤",
    ];
    let mut results = Vec::new();
    for query in queries {
        let response = search(
            database,
            SearchRequest {
                query: query.to_string(),
                item_type: None,
                page: Some(1),
                page_size: Some(1),
            },
        )?;
        if let Some(first) = response.results.into_iter().next() {
            results.push(first);
        }
    }
    Ok(results)
}

fn normalize_page_size(page_size: Option<u32>) -> u32 {
    page_size.unwrap_or(50).clamp(1, 200)
}

fn annotation_hit_item_ids(database: &Database, query: &str) -> AppResult<Vec<i64>> {
    let like = format!("%{query}%");
    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT DISTINCT knowledge_item_id
             FROM knowledge_annotations
             WHERE content LIKE ?1
                OR source_title LIKE ?1
                OR source_note LIKE ?1
                OR tags_json LIKE ?1
             LIMIT 200",
        )?;
        let rows = statement.query_map(params![like], |row| row.get(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    })
}

fn load_enhanced_result(
    database: &Database,
    item_id: i64,
    query: &str,
    score: i64,
    matched_by: String,
) -> AppResult<Option<EnhancedSearchResult>> {
    database.with_connection(|connection| {
        let item = connection
            .query_row(
                "SELECT ki.id, ki.type, ki.code, ki.name, ki.category, ki.summary, ki.content, ki.source_note,
                        ki.tags, ki.detail, ki.import_batch_id, ki.source_package, hd.four_qi
                 FROM knowledge_items ki
                 LEFT JOIN herb_details hd ON hd.item_id = ki.id
                 WHERE ki.id = ?1",
                params![item_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                        row.get::<_, Option<String>>(9)?,
                        row.get::<_, Option<String>>(10)?,
                        row.get::<_, Option<String>>(11)?,
                        row.get::<_, Option<String>>(12)?,
                    ))
                },
            )
            .optional()?;
        let Some((
            item_id,
            item_type,
            code,
            name,
            category,
            summary,
            content,
            source_note,
            tags,
            detail,
            import_batch_id,
            source_package,
            four_qi,
        )) = item
        else {
            return Ok(None);
        };
        let (annotation_count, first_source_title, annotation_snippet) =
            annotation_summary_for_item(connection, item_id, query)?;
        let detail_snippet = detail
            .as_deref()
            .and_then(|value| snippet(value, query))
            .filter(|value| !value.is_empty());
        let content_snippet = content
            .as_deref()
            .and_then(|value| snippet(value, query))
            .or_else(|| summary.as_deref().and_then(|value| snippet(value, query)))
            .or(detail_snippet)
            .or_else(|| content.as_deref().map(|value| truncate(value, 120)));
        let group_name = group_name_for_type(&item_type, category.as_deref());
        Ok(Some(EnhancedSearchResult {
            item_id,
            item_type: item_type.clone(),
            type_label: type_label(&item_type, category.as_deref()).to_string(),
            group_name,
            code,
            name,
            category,
            summary,
            content_snippet,
            source_title: first_source_title.or_else(|| source_package.clone()),
            source_note,
            tags,
            four_qi: four_qi.or_else(|| four_qi_from_detail(detail.as_deref())),
            has_annotations: annotation_count > 0,
            annotation_count,
            annotation_snippet,
            matched_by,
            score,
            import_batch_id,
            source_package,
        }))
    })
}

fn annotation_summary_for_item(
    connection: &rusqlite::Connection,
    item_id: i64,
    query: &str,
) -> AppResult<(i64, Option<String>, Option<String>)> {
    let count: i64 = connection.query_row(
        "SELECT COUNT(1) FROM knowledge_annotations WHERE knowledge_item_id = ?1",
        params![item_id],
        |row| row.get(0),
    )?;
    let like = format!("%{query}%");
    let hit = connection
        .query_row(
            "SELECT source_title, source_note, content
             FROM knowledge_annotations
             WHERE knowledge_item_id = ?1
               AND (content LIKE ?2 OR source_title LIKE ?2 OR source_note LIKE ?2 OR tags_json LIKE ?2)
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
            params![item_id, like],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()?;
    if let Some((title, note, content)) = hit {
        let source = [title.clone(), note]
            .into_iter()
            .flatten()
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" | ");
        let text = snippet(&content, query).unwrap_or_else(|| truncate(&content, 120));
        let combined = if source.is_empty() {
            text
        } else {
            format!("{source} | {text}")
        };
        Ok((count, title, Some(combined)))
    } else {
        let title = connection
            .query_row(
                "SELECT source_title
                 FROM knowledge_annotations
                 WHERE knowledge_item_id = ?1
                 ORDER BY created_at DESC, id DESC
                 LIMIT 1",
                params![item_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        Ok((count, title, None))
    }
}

fn first_annotation_snippet(
    database: &Database,
    item_id: i64,
    query: &str,
) -> AppResult<Option<String>> {
    database.with_connection(|connection| {
        Ok(annotation_summary_for_item(connection, item_id, query)?.2)
    })
}

fn four_qi_from_detail(detail: Option<&str>) -> Option<String> {
    let value = detail.and_then(|text| serde_json::from_str::<Value>(text).ok())?;
    [
        "fourQi",
        "four_qi",
        "四气",
        "natureFlavor",
        "nature_flavor",
        "性味",
    ]
    .iter()
    .find_map(|key| {
        value
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned)
    })
}

fn group_results(results: Vec<EnhancedSearchResult>) -> Vec<EnhancedSearchGroup> {
    let order = [
        "中药",
        "方剂",
        "穴位",
        "经络",
        "原典章节",
        "原典条文",
        "注解资料",
        "其他",
    ];
    let mut grouped: BTreeMap<String, Vec<EnhancedSearchResult>> = BTreeMap::new();
    for result in results {
        grouped
            .entry(result.group_name.clone())
            .or_default()
            .push(result);
    }
    let mut output = Vec::new();
    for name in order {
        if let Some(results) = grouped.remove(name) {
            output.push(EnhancedSearchGroup {
                group_name: name.to_string(),
                results,
            });
        }
    }
    for (group_name, results) in grouped {
        output.push(EnhancedSearchGroup {
            group_name,
            results,
        });
    }
    output
}

fn filter_allows(filter: &str, result: &EnhancedSearchResult) -> bool {
    match filter {
        "全部" | "" => true,
        "中药" => result.item_type == "herb" || result.group_name == "中药",
        "方剂" => result.item_type == "formula" || result.group_name == "方剂",
        "穴位" => result.item_type == "acupoint" || result.group_name == "穴位",
        "经络" => result.item_type == "meridian" || result.group_name == "经络",
        "针灸" => {
            matches!(
                result.item_type.as_str(),
                "acupuncture" | "acupoint" | "meridian"
            ) || matches!(result.group_name.as_str(), "穴位" | "经络")
        }
        "原典" => {
            matches!(result.item_type.as_str(), "theory" | "syndrome")
                || matches!(result.group_name.as_str(), "原典章节" | "原典条文")
        }
        "注解" => result.has_annotations || result.matched_by.contains("annotation"),
        _ => true,
    }
}

fn group_name_for_type(item_type: &str, category: Option<&str>) -> String {
    match item_type {
        "herb" => "中药",
        "formula" => "方剂",
        "acupoint" => "穴位",
        "meridian" => "经络",
        "acupuncture" => {
            let category = category.unwrap_or_default();
            if category.contains("经") {
                "经络"
            } else {
                "穴位"
            }
        }
        "theory" => "原典章节",
        "syndrome" => "原典条文",
        "note" => "注解资料",
        _ => "其他",
    }
    .to_string()
}

fn type_label(item_type: &str, category: Option<&str>) -> &'static str {
    match group_name_for_type(item_type, category).as_str() {
        "中药" => "中药",
        "方剂" => "方剂",
        "穴位" => "穴位",
        "经络" => "经络",
        "原典章节" => "原典章节",
        "原典条文" => "原典条文",
        "注解资料" => "注解资料",
        _ => "其他",
    }
}

fn snippet(value: &str, query: &str) -> Option<String> {
    let index = value.find(query)?;
    let start = value[..index]
        .char_indices()
        .rev()
        .nth(28)
        .map(|(idx, _)| idx)
        .unwrap_or(0);
    let end = value[index + query.len()..]
        .char_indices()
        .nth(90)
        .map(|(idx, _)| index + query.len() + idx)
        .unwrap_or(value.len());
    Some(truncate(&value[start..end], 140))
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
    use crate::db::connection::Database;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn search_performance_seed_smoke() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("zhongyi-search-test-{unique}"));
        let database = Database::initialize(&data_dir).expect("database initializes");

        let seed = generate_performance_test_data(
            &database,
            SearchSeedOptions {
                item_count: Some(10_000),
                relation_count: Some(50_000),
                reset_existing: Some(true),
            },
        )
        .expect("seed data");
        println!(
            "seed: {} items, {} relations, {}ms",
            seed.inserted_items, seed.inserted_relations, seed.duration_ms
        );

        rebuild_search_index(&database).expect("rebuild index");

        for query in [
            "足三里",
            "ST36",
            "st36",
            "zusanli",
            "胃经",
            "足阳明胃经",
            "黄芪",
            "黄耆",
            "补中益气汤",
        ] {
            let response = search(
                &database,
                SearchRequest {
                    query: query.to_string(),
                    item_type: None,
                    page: Some(1),
                    page_size: Some(10),
                },
            )
            .expect("search succeeds");
            println!(
                "search {query}: {} hits, {}ms",
                response.results.len(),
                response.duration_ms
            );
            assert!(
                !response.results.is_empty(),
                "expected at least one hit for {query}"
            );
            assert!(
                response.duration_ms < 500,
                "expected {query} search under 500ms, got {}ms",
                response.duration_ms
            );
        }

        let list = list_cache(
            &database,
            ListCacheRequest {
                item_type: None,
                page: Some(20),
                page_size: Some(50),
            },
        )
        .expect("list cache");
        println!(
            "list page {} size {}: {} rows, {}ms",
            list.page,
            list.page_size,
            list.results.len(),
            list.duration_ms
        );
        assert_eq!(list.results.len(), 50);
        assert!(
            list.duration_ms < 300,
            "expected list paging under 300ms, got {}ms",
            list.duration_ms
        );

        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn enhanced_search_hits_renji_reading_keywords() {
        let data_dir = temp_data_dir("enhanced-search");
        let database = Database::initialize(&data_dir).expect("database initializes");
        let renshen = seed_item(
            &database,
            "herb",
            "人参",
            Some("神农本草经 上品"),
            Some("黄芪,黄耆"),
            "神农本草经 人参 甘草 黄芪 黄耆",
        );
        seed_item(
            &database,
            "formula",
            "桂枝汤",
            Some("伤寒论"),
            None,
            "太阳病 桂枝汤 方剂",
        );
        seed_item(
            &database,
            "theory",
            "上古天真论篇第一",
            Some("黄帝内经"),
            None,
            "上古天真论 原典章节",
        );
        seed_item(
            &database,
            "acupuncture",
            "足三里",
            Some("穴位"),
            None,
            "足三里 足阳明胃经",
        );
        seed_item(
            &database,
            "acupuncture",
            "足阳明胃经",
            Some("经络"),
            None,
            "足阳明胃经 经络",
        );
        seed_item(
            &database,
            "formula",
            "理中丸",
            Some("金匮要略"),
            None,
            "金匮要略 理中丸",
        );
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_annotations
                     (knowledge_item_id, annotation_type, source_title, source_note, content, detail_json, tags_json, created_at, updated_at)
                     VALUES (?1, 'source_annotation', '3人纪-神农本草经.pdf', 'PDF页码26-27', '倪注：人参大补元气。', '{}', '倪注', datetime('now'), datetime('now'))",
                    params![renshen],
                )?;
                Ok(())
            })
            .unwrap();
        rebuild_search_index(&database).unwrap();

        for query in [
            "人参",
            "甘草",
            "黄耆",
            "黄芪",
            "倪注",
            "神农本草经",
            "桂枝汤",
            "太阳病",
            "上古天真论",
            "足三里",
            "足阳明胃经",
            "理中丸",
            "金匮要略",
        ] {
            let response = search_enhanced(
                &database,
                EnhancedSearchRequest {
                    query: query.to_string(),
                    filter: Some("全部".to_string()),
                    page: Some(1),
                    page_size: Some(20),
                },
            )
            .unwrap();
            assert!(response.total > 0, "expected hit for {query}");
        }

        let annotation_response = search_enhanced(
            &database,
            EnhancedSearchRequest {
                query: "倪注".to_string(),
                filter: Some("注解".to_string()),
                page: Some(1),
                page_size: Some(20),
            },
        )
        .unwrap();
        let first = annotation_response.groups[0].results[0].clone();
        assert!(first.has_annotations);
        assert!(first
            .annotation_snippet
            .unwrap_or_default()
            .contains("倪注"));

        let _ = fs::remove_dir_all(data_dir);
    }

    fn seed_item(
        database: &Database,
        item_type: &str,
        name: &str,
        category: Option<&str>,
        alias: Option<&str>,
        content: &str,
    ) -> i64 {
        database
            .with_connection(|connection| {
                connection.execute(
                    "INSERT INTO knowledge_items
                     (type, name, alias, category, summary, content, source_note, tags, data_status,
                      completeness_status, content_version, is_favorite, detail, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'imported', 'partial', 1, 0, '{}', datetime('now'), datetime('now'))",
                    params![
                        item_type,
                        name,
                        alias,
                        category,
                        format!("{name} 摘要"),
                        content,
                        category.unwrap_or("测试来源"),
                        category.unwrap_or("测试")
                    ],
                )?;
                Ok(connection.last_insert_rowid())
            })
            .unwrap()
    }

    fn temp_data_dir(test_name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("zhongyi-search-test-{test_name}-{unique}"))
    }
}

/// 重建单个条目的搜索索引（用于版本回滚后同步）
pub fn rebuild_item_index(database: &Database, item_id: i64) -> AppResult<()> {
    // 先删除旧索引
    delete_knowledge_item_index(database, item_id)?;
    // 重新索引
    index_knowledge_item(database, item_id)?;
    Ok(())
}

lazy_static::lazy_static! {
    static ref SEARCH_CACHE: Arc<Mutex<HashMap<String, (SearchResponse, std::time::SystemTime)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    static ref SEARCH_STATS: Arc<Mutex<HashMap<String, usize>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

pub fn search_with_cache(database: &Database, request: SearchRequest) -> AppResult<SearchResponse> {
    let cache_key = format!(
        "{}:{}:{}:{}",
        request.query.trim(),
        request.item_type.as_deref().unwrap_or("all"),
        request.page.unwrap_or(1),
        request.page_size.unwrap_or(50)
    );

    // 记录搜索统计
    {
        let mut stats = SEARCH_STATS.lock().unwrap();
        *stats.entry(request.query.trim().to_string()).or_insert(0) += 1;
    }

    // 检查缓存（5分钟有效期）
    {
        let cache = SEARCH_CACHE.lock().unwrap();
        if let Some((cached_response, cached_time)) = cache.get(&cache_key) {
            if cached_time
                .elapsed()
                .unwrap_or(std::time::Duration::from_secs(301))
                < std::time::Duration::from_secs(300)
            {
                return Ok(cached_response.clone());
            }
        }
    }

    // 执行搜索
    let response = search(database, request)?;

    // 更新缓存
    {
        let mut cache = SEARCH_CACHE.lock().unwrap();
        cache.insert(cache_key, (response.clone(), std::time::SystemTime::now()));

        // 限制缓存大小
        if cache.len() > 100 {
            // 移除最旧的条目
            if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, (_, time))| time)
                .map(|(k, _)| k.clone())
            {
                cache.remove(&oldest_key);
            }
        }
    }

    Ok(response)
}

pub fn get_hot_search_terms(limit: usize) -> Vec<HotSearchTerm> {
    let stats = SEARCH_STATS.lock().unwrap();

    let mut terms: Vec<(String, usize)> = stats.iter().map(|(k, v)| (k.clone(), *v)).collect();

    terms.sort_by(|a, b| b.1.cmp(&a.1));

    terms
        .into_iter()
        .take(limit)
        .map(|(term, count)| HotSearchTerm { term, count })
        .collect()
}

pub fn clear_search_cache() {
    let mut cache = SEARCH_CACHE.lock().unwrap();
    cache.clear();
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HotSearchTerm {
    pub term: String,
    pub count: usize,
}
