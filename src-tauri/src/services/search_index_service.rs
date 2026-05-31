use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::search::{
    KnowledgeSearchResult, ListCacheRequest, ListCacheResponse, RebuildSearchIndexResponse,
    SearchRequest, SearchResponse, SearchSeedOptions, SearchSeedResponse,
};
use crate::repositories::{performance_repository, search_repository};
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
}

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
            if cached_time.elapsed().unwrap_or(std::time::Duration::from_secs(301))
                < std::time::Duration::from_secs(300) {
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
            if let Some(oldest_key) = cache.iter()
                .min_by_key(|(_, (_, time))| time)
                .map(|(k, _)| k.clone()) {
                cache.remove(&oldest_key);
            }
        }
    }

    Ok(response)
}

pub fn get_hot_search_terms(limit: usize) -> Vec<HotSearchTerm> {
    let stats = SEARCH_STATS.lock().unwrap();

    let mut terms: Vec<(String, usize)> = stats.iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    terms.sort_by(|a, b| b.1.cmp(&a.1));

    terms.into_iter()
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
