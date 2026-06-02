use crate::db::connection::Database;
use crate::errors::{AppError, AppResult};
use crate::models::search::{KnowledgeSearchResult, SearchSeedResponse};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct IndexableKnowledgeItem {
    pub id: i64,
    pub item_type: String,
    pub code: Option<String>,
    pub name: String,
    pub alias: Option<String>,
    pub pinyin: Option<String>,
    pub category: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub source_note: Option<String>,
    pub tags: Option<String>,
    pub detail: Option<String>,
    pub data_status: String,
    pub is_favorite: i64,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct SearchTerm {
    pub term: String,
    pub term_type: String,
    pub weight: i64,
}

#[derive(Debug, Clone)]
pub struct SearchCandidate {
    pub item_id: i64,
    pub score: i64,
    pub matched_by: String,
}

pub fn rebuild_all(
    database: &Database,
    term_builder: impl Fn(&IndexableKnowledgeItem) -> Vec<SearchTerm>,
) -> AppResult<(i64, i64)> {
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM knowledge_fts", [])?;
        transaction.execute("DELETE FROM search_terms", [])?;
        transaction.execute("DELETE FROM relation_count_cache", [])?;
        transaction.execute("DELETE FROM knowledge_list_view_cache", [])?;

        rebuild_relation_count_cache_tx(&transaction)?;

        let items = load_indexable_items(&transaction)?;
        let mut term_count = 0_i64;
        for item in &items {
            upsert_fts_tx(&transaction, item)?;
            term_count += replace_terms_tx(&transaction, item.id, &term_builder(item))?;
            upsert_list_cache_tx(&transaction, item)?;
        }

        transaction.commit()?;
        Ok((items.len() as i64, term_count))
    })
}

pub fn index_item(
    database: &Database,
    item_id: i64,
    term_builder: impl Fn(&IndexableKnowledgeItem) -> Vec<SearchTerm>,
) -> AppResult<i64> {
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        let item = load_indexable_item(&transaction, item_id)?.ok_or_else(|| {
            AppError::InvalidInput(format!("knowledge item {item_id} does not exist"))
        })?;
        upsert_fts_tx(&transaction, &item)?;
        let term_count = replace_terms_tx(&transaction, item.id, &term_builder(&item))?;
        upsert_list_cache_tx(&transaction, &item)?;
        transaction.commit()?;
        Ok(term_count)
    })
}

pub fn delete_item_index(database: &Database, item_id: i64) -> AppResult<()> {
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM knowledge_fts WHERE rowid = ?1",
            params![item_id],
        )?;
        transaction.execute(
            "DELETE FROM search_terms WHERE item_id = ?1",
            params![item_id],
        )?;
        transaction.execute(
            "DELETE FROM knowledge_list_view_cache WHERE item_id = ?1",
            params![item_id],
        )?;
        transaction.execute(
            "DELETE FROM relation_count_cache WHERE item_id = ?1",
            params![item_id],
        )?;
        transaction.commit()?;
        Ok(())
    })
}

pub fn rebuild_relation_count_cache(database: &Database) -> AppResult<()> {
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM relation_count_cache", [])?;
        rebuild_relation_count_cache_tx(&transaction)?;
        refresh_all_list_relation_counts_tx(&transaction)?;
        transaction.commit()?;
        Ok(())
    })
}

pub fn search_candidates(
    database: &Database,
    query: &str,
    item_type: Option<&str>,
    limit: u32,
) -> AppResult<Vec<SearchCandidate>> {
    let normalized = normalize_for_search(query);
    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    database.with_connection(|connection| {
        let mut candidates: HashMap<i64, SearchCandidate> = HashMap::new();
        collect_term_candidates(connection, &normalized, item_type, limit, &mut candidates)?;
        collect_fts_candidates(connection, &normalized, item_type, limit, &mut candidates)?;

        let mut values = candidates.into_values().collect::<Vec<_>>();
        values.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.item_id.cmp(&right.item_id))
        });
        values.truncate(limit as usize);
        Ok(values)
    })
}

pub fn hydrate_results(
    database: &Database,
    candidates: &[SearchCandidate],
) -> AppResult<Vec<KnowledgeSearchResult>> {
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    database.with_connection(|connection| {
        let mut results = Vec::with_capacity(candidates.len());
        let mut statement = connection.prepare(
            "SELECT item_id, type, code, name, pinyin, category, summary, tags,
                    data_status, relation_count
             FROM knowledge_list_view_cache
             WHERE item_id = ?1",
        )?;

        for candidate in candidates {
            if let Some(mut result) = statement
                .query_row(params![candidate.item_id], |row| {
                    Ok(KnowledgeSearchResult {
                        item_id: row.get(0)?,
                        item_type: row.get(1)?,
                        code: row.get(2)?,
                        name: row.get(3)?,
                        pinyin: row.get(4)?,
                        category: row.get(5)?,
                        summary: row.get(6)?,
                        tags: row.get(7)?,
                        data_status: row.get(8)?,
                        relation_count: row.get(9)?,
                        score: candidate.score,
                        matched_by: candidate.matched_by.clone(),
                    })
                })
                .optional()?
            {
                result.score = candidate.score;
                results.push(result);
            }
        }

        Ok(results)
    })
}

pub fn list_cache(
    database: &Database,
    item_type: Option<&str>,
    page: u32,
    page_size: u32,
) -> AppResult<(i64, Vec<KnowledgeSearchResult>)> {
    let offset = (page.saturating_sub(1) * page_size) as i64;
    database.with_connection(|connection| {
        let (total, statement_sql) = if item_type.is_some() {
            (
                connection.query_row(
                    "SELECT COUNT(1) FROM knowledge_list_view_cache
                     WHERE type = ?1 AND data_status IN ('validated', 'ready', 'imported', 'reviewed', 'pending_review', 'needs_check')",
                    params![item_type],
                    |row| row.get(0),
                )?,
                "SELECT item_id, type, code, name, pinyin, category, summary, tags,
                        data_status, relation_count
                 FROM knowledge_list_view_cache
                 WHERE type = ?1 AND data_status IN ('validated', 'ready', 'imported', 'reviewed', 'pending_review', 'needs_check')
                 ORDER BY updated_at DESC, item_id DESC
                 LIMIT ?2 OFFSET ?3"
                    .to_string(),
            )
        } else {
            (
                connection.query_row(
                    "SELECT COUNT(1) FROM knowledge_list_view_cache
                     WHERE data_status IN ('validated', 'ready', 'imported', 'reviewed', 'pending_review', 'needs_check')",
                    [],
                    |row| row.get(0),
                )?,
                "SELECT item_id, type, code, name, pinyin, category, summary, tags,
                        data_status, relation_count
                 FROM knowledge_list_view_cache
                 WHERE data_status IN ('validated', 'ready', 'imported', 'reviewed', 'pending_review', 'needs_check')
                 ORDER BY updated_at DESC, item_id DESC
                 LIMIT ?1 OFFSET ?2"
                    .to_string(),
            )
        };

        let mut statement = connection.prepare(&statement_sql)?;
        let rows = if let Some(kind) = item_type {
            statement.query_map(params![kind, page_size, offset], map_list_row)?
        } else {
            statement.query_map(params![page_size, offset], map_list_row)?
        };

        let results = rows.collect::<Result<Vec<_>, _>>()?;
        Ok((total, results))
    })
}

pub fn generate_seed_data(
    database: &Database,
    item_count: u32,
    relation_count: u32,
    reset_existing: bool,
    term_builder: impl Fn(&IndexableKnowledgeItem) -> Vec<SearchTerm>,
) -> AppResult<SearchSeedResponse> {
    let started_at = Instant::now();
    database.with_connection(|connection| {
        let transaction = connection.unchecked_transaction()?;
        if reset_existing {
            transaction.execute("DELETE FROM knowledge_relations", [])?;
            transaction.execute("DELETE FROM knowledge_items", [])?;
            transaction.execute("DELETE FROM knowledge_fts", [])?;
            transaction.execute("DELETE FROM search_terms", [])?;
            transaction.execute("DELETE FROM relation_count_cache", [])?;
            transaction.execute("DELETE FROM knowledge_list_view_cache", [])?;
        }

        let base_items = [
            (
                "acupoint",
                "ST36",
                "足三里",
                "足三里穴",
                "zusanli",
                "足阳明胃经",
                "胃经,常用穴",
            ),
            (
                "meridian",
                "ST",
                "足阳明胃经",
                "胃经",
                "zuyangmingweijing",
                "十二经脉",
                "胃经,阳明",
            ),
            (
                "herb",
                "H0001",
                "黄芪",
                "黄耆",
                "huangqi",
                "补气药",
                "中药,补气",
            ),
            (
                "formula",
                "F0001",
                "补中益气汤",
                "",
                "buzhongyiqitang",
                "补益剂",
                "方剂,补气",
            ),
        ];
        let types = [
            "herb", "formula", "meridian", "acupoint", "syndrome", "disease",
        ];

        for index in 0..item_count {
            let now = "datetime('now')";
            if (index as usize) < base_items.len() {
                let item = base_items[index as usize];
                transaction.execute(
                    "INSERT INTO knowledge_items
                     (type, code, name, alias, pinyin, category, summary, content, tags,
                      data_status, completeness_status, is_favorite, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                             'ready', 'complete', 0, datetime('now'), datetime('now'))",
                    params![
                        item.0,
                        item.1,
                        item.2,
                        empty_to_none(item.3),
                        item.4,
                        item.5,
                        format!("{} 的测试摘要", item.2),
                        format!("{} 用于搜索与性能测试。", item.2),
                        item.6
                    ],
                )?;
            } else {
                let kind = types[index as usize % types.len()];
                let code = format!("T{:05}", index + 1);
                let name = format!("测试知识{:05}", index + 1);
                let pinyin = format!("ceshizhishi{:05}", index + 1);
                transaction.execute(
                    &format!(
                        "INSERT INTO knowledge_items
                         (type, code, name, alias, pinyin, category, summary, content, tags,
                          data_status, completeness_status, is_favorite, created_at, updated_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                                 'ready', 'partial', 0, {now}, {now})"
                    ),
                    params![
                        kind,
                        code,
                        name,
                        format!("别名{:05}", index + 1),
                        pinyin,
                        format!("分类{}", index % 20),
                        format!("第 {} 条搜索性能测试知识。", index + 1),
                        format!("内容字段用于 FTS5 性能验证，编号为 {}。", code),
                        format!("标签{},性能测试", index % 30)
                    ],
                )?;
            }
        }

        let max_id: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(id), 0) FROM knowledge_items",
            [],
            |row| row.get(0),
        )?;
        let min_id = max_id - item_count as i64 + 1;
        if item_count > 1 {
            for index in 0..relation_count {
                let source = min_id + (index as i64 % item_count as i64);
                let target = min_id + ((index as i64 * 37 + 11) % item_count as i64);
                if source != target {
                    transaction.execute(
                        "INSERT INTO knowledge_relations
                         (source_item_id, target_item_id, relation_type, note)
                         VALUES (?1, ?2, ?3, ?4)",
                        params![source, target, relation_type_for(index), "性能测试关系"],
                    )?;
                }
            }
        }

        rebuild_relation_count_cache_tx(&transaction)?;
        let items = load_indexable_items_since(&transaction, min_id)?;
        for item in &items {
            upsert_fts_tx(&transaction, item)?;
            replace_terms_tx(&transaction, item.id, &term_builder(item))?;
            upsert_list_cache_tx(&transaction, item)?;
        }
        refresh_all_list_relation_counts_tx(&transaction)?;
        transaction.commit()?;

        Ok(SearchSeedResponse {
            inserted_items: item_count,
            inserted_relations: relation_count,
            duration_ms: started_at.elapsed().as_millis() as i64,
        })
    })
}

fn collect_term_candidates(
    connection: &Connection,
    normalized: &str,
    item_type: Option<&str>,
    limit: u32,
    candidates: &mut HashMap<i64, SearchCandidate>,
) -> AppResult<()> {
    let contains = format!("%{normalized}%");
    let prefix = format!("{normalized}%");
    let sql = if item_type.is_some() {
        "SELECT st.item_id,
                MAX(st.weight +
                    CASE
                      WHEN st.term = ?1 THEN 120
                      WHEN st.term LIKE ?2 THEN 70
                      WHEN st.term LIKE ?3 THEN 25
                      ELSE 0
                    END) AS score,
                GROUP_CONCAT(DISTINCT st.term_type) AS matched_by
         FROM search_terms st
         JOIN knowledge_list_view_cache lc ON lc.item_id = st.item_id
         WHERE lc.type = ?4
           AND lc.data_status IN ('validated', 'ready', 'imported', 'reviewed', 'pending_review', 'needs_check')
           AND (st.term = ?1 OR st.term LIKE ?2 OR st.term LIKE ?3)
         GROUP BY st.item_id
         ORDER BY score DESC
         LIMIT ?5"
    } else {
        "SELECT st.item_id,
                MAX(st.weight +
                    CASE
                      WHEN st.term = ?1 THEN 120
                      WHEN st.term LIKE ?2 THEN 70
                      WHEN st.term LIKE ?3 THEN 25
                      ELSE 0
                    END) AS score,
                GROUP_CONCAT(DISTINCT st.term_type) AS matched_by
         FROM search_terms st
         JOIN knowledge_list_view_cache lc ON lc.item_id = st.item_id
         WHERE lc.data_status IN ('validated', 'ready', 'imported', 'reviewed', 'pending_review', 'needs_check')
           AND (st.term = ?1 OR st.term LIKE ?2 OR st.term LIKE ?3)
         GROUP BY st.item_id
         ORDER BY score DESC
         LIMIT ?4"
    };

    let mut statement = connection.prepare(sql)?;
    if let Some(kind) = item_type {
        let rows = statement.query_map(
            params![normalized, prefix, contains, kind, limit],
            map_term_candidate_row,
        )?;
        for row in rows {
            merge_candidate(candidates, row?);
        }
    } else {
        let rows = statement.query_map(
            params![normalized, prefix, contains, limit],
            map_term_candidate_row,
        )?;
        for row in rows {
            merge_candidate(candidates, row?);
        }
    }
    Ok(())
}

fn collect_fts_candidates(
    connection: &Connection,
    normalized: &str,
    item_type: Option<&str>,
    limit: u32,
    candidates: &mut HashMap<i64, SearchCandidate>,
) -> AppResult<()> {
    let fts_query = make_fts_query(normalized);
    let sql = if item_type.is_some() {
        "SELECT f.rowid, 40 - CAST(bm25(knowledge_fts) AS INTEGER) AS score
         FROM knowledge_fts f
         JOIN knowledge_list_view_cache lc ON lc.item_id = f.rowid
         WHERE knowledge_fts MATCH ?1
           AND lc.type = ?2
           AND lc.data_status IN ('validated', 'ready', 'imported', 'reviewed', 'pending_review', 'needs_check')
         ORDER BY bm25(knowledge_fts)
         LIMIT ?3"
    } else {
        "SELECT f.rowid, 40 - CAST(bm25(knowledge_fts) AS INTEGER) AS score
         FROM knowledge_fts f
         JOIN knowledge_list_view_cache lc ON lc.item_id = f.rowid
         WHERE knowledge_fts MATCH ?1
           AND lc.data_status IN ('validated', 'ready', 'imported', 'reviewed', 'pending_review', 'needs_check')
         ORDER BY bm25(knowledge_fts)
         LIMIT ?2"
    };

    let mut statement = connection.prepare(sql)?;
    let result = if let Some(kind) = item_type {
        statement
            .query_map(params![fts_query, kind, limit], map_fts_candidate_row)
            .and_then(|rows| {
                for row in rows {
                    merge_candidate(candidates, row?);
                }
                Ok(())
            })
    } else {
        statement
            .query_map(params![fts_query, limit], map_fts_candidate_row)
            .and_then(|rows| {
                for row in rows {
                    merge_candidate(candidates, row?);
                }
                Ok(())
            })
    };

    match result {
        Ok(()) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(_, _)) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn load_indexable_items(connection: &Connection) -> AppResult<Vec<IndexableKnowledgeItem>> {
    let mut statement = connection.prepare(
        "SELECT id, type, code, name, alias, pinyin, category, summary, content,
                source_note, tags, detail, data_status, is_favorite, updated_at
         FROM knowledge_items",
    )?;
    let rows = statement.query_map([], map_indexable_row)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn load_indexable_items_since(
    connection: &Connection,
    min_id: i64,
) -> AppResult<Vec<IndexableKnowledgeItem>> {
    let mut statement = connection.prepare(
        "SELECT id, type, code, name, alias, pinyin, category, summary, content,
                source_note, tags, detail, data_status, is_favorite, updated_at
         FROM knowledge_items
         WHERE id >= ?1",
    )?;
    let rows = statement.query_map(params![min_id], map_indexable_row)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn load_indexable_item(
    connection: &Connection,
    item_id: i64,
) -> AppResult<Option<IndexableKnowledgeItem>> {
    connection
        .query_row(
            "SELECT id, type, code, name, alias, pinyin, category, summary, content,
                    source_note, tags, detail, data_status, is_favorite, updated_at
             FROM knowledge_items
             WHERE id = ?1",
            params![item_id],
            map_indexable_row,
        )
        .optional()
        .map_err(Into::into)
}

fn upsert_fts_tx(connection: &Connection, item: &IndexableKnowledgeItem) -> AppResult<()> {
    let annotation_text = annotation_search_text(connection, item.id)?;
    let content = [
        item.content.as_deref().unwrap_or_default(),
        annotation_text.as_str(),
    ]
    .into_iter()
    .filter(|value| !value.trim().is_empty())
    .collect::<Vec<_>>()
    .join("\n");
    connection.execute(
        "DELETE FROM knowledge_fts WHERE rowid = ?1",
        params![item.id],
    )?;
    connection.execute(
        "INSERT INTO knowledge_fts
        (rowid, name, code, alias, pinyin, category, summary, content, source_note, tags, detail_text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            item.id,
            item.name,
            item.code,
            item.alias,
            item.pinyin,
            item.category,
            item.summary,
            content,
            item.source_note,
            item.tags,
            detail_text(item.detail.as_deref())
        ],
    )?;
    Ok(())
}

fn replace_terms_tx(connection: &Connection, item_id: i64, terms: &[SearchTerm]) -> AppResult<i64> {
    connection.execute(
        "DELETE FROM search_terms WHERE item_id = ?1",
        params![item_id],
    )?;
    for term in terms {
        connection.execute(
            "INSERT INTO search_terms (item_id, term, term_type, weight)
             VALUES (?1, ?2, ?3, ?4)",
            params![item_id, term.term, term.term_type, term.weight],
        )?;
    }
    Ok(terms.len() as i64)
}

fn upsert_list_cache_tx(connection: &Connection, item: &IndexableKnowledgeItem) -> AppResult<()> {
    connection.execute(
        "INSERT INTO knowledge_list_view_cache
         (item_id, type, code, name, pinyin, category, summary, tags, data_status,
          is_favorite, relation_count, updated_at)
         VALUES (
          ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10,
          COALESCE((SELECT SUM(count) FROM relation_count_cache WHERE item_id = ?1), 0),
          ?11
         )
         ON CONFLICT(item_id) DO UPDATE SET
          type = excluded.type,
          code = excluded.code,
          name = excluded.name,
          pinyin = excluded.pinyin,
          category = excluded.category,
          summary = excluded.summary,
          tags = excluded.tags,
          data_status = excluded.data_status,
          is_favorite = excluded.is_favorite,
          relation_count = excluded.relation_count,
          updated_at = excluded.updated_at",
        params![
            item.id,
            item.item_type,
            item.code,
            item.name,
            item.pinyin,
            item.category,
            item.summary,
            item.tags,
            item.data_status,
            item.is_favorite,
            item.updated_at
        ],
    )?;
    Ok(())
}

fn rebuild_relation_count_cache_tx(connection: &Connection) -> AppResult<()> {
    connection.execute(
        "INSERT INTO relation_count_cache (item_id, relation_type, count, updated_at)
         SELECT item_id, relation_type, COUNT(1), datetime('now')
         FROM (
           SELECT source_item_id AS item_id, relation_type FROM knowledge_relations
           UNION ALL
           SELECT target_item_id AS item_id, relation_type FROM knowledge_relations
         )
         GROUP BY item_id, relation_type",
        [],
    )?;
    Ok(())
}

fn refresh_all_list_relation_counts_tx(connection: &Connection) -> AppResult<()> {
    connection.execute(
        "UPDATE knowledge_list_view_cache
         SET relation_count = COALESCE((
           SELECT SUM(count)
           FROM relation_count_cache
           WHERE relation_count_cache.item_id = knowledge_list_view_cache.item_id
         ), 0)",
        [],
    )?;
    Ok(())
}

fn map_indexable_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IndexableKnowledgeItem> {
    Ok(IndexableKnowledgeItem {
        id: row.get(0)?,
        item_type: row.get(1)?,
        code: row.get(2)?,
        name: row.get(3)?,
        alias: row.get(4)?,
        pinyin: row.get(5)?,
        category: row.get(6)?,
        summary: row.get(7)?,
        content: row.get(8)?,
        source_note: row.get(9)?,
        tags: row.get(10)?,
        detail: row.get(11)?,
        data_status: row.get(12)?,
        is_favorite: row.get(13)?,
        updated_at: row.get(14)?,
    })
}

fn map_list_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<KnowledgeSearchResult> {
    Ok(KnowledgeSearchResult {
        item_id: row.get(0)?,
        item_type: row.get(1)?,
        code: row.get(2)?,
        name: row.get(3)?,
        pinyin: row.get(4)?,
        category: row.get(5)?,
        summary: row.get(6)?,
        tags: row.get(7)?,
        data_status: row.get(8)?,
        relation_count: row.get(9)?,
        score: 0,
        matched_by: "list_cache".to_string(),
    })
}

fn map_term_candidate_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchCandidate> {
    Ok(SearchCandidate {
        item_id: row.get(0)?,
        score: row.get(1)?,
        matched_by: row
            .get::<_, Option<String>>(2)?
            .unwrap_or_else(|| "term".into()),
    })
}

fn map_fts_candidate_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SearchCandidate> {
    Ok(SearchCandidate {
        item_id: row.get(0)?,
        score: row.get(1)?,
        matched_by: "fts".to_string(),
    })
}

fn merge_candidate(candidates: &mut HashMap<i64, SearchCandidate>, incoming: SearchCandidate) {
    candidates
        .entry(incoming.item_id)
        .and_modify(|existing| {
            existing.score += incoming.score;
            if !existing.matched_by.contains(&incoming.matched_by) {
                existing.matched_by = format!("{},{}", existing.matched_by, incoming.matched_by);
            }
        })
        .or_insert(incoming);
}

pub fn normalize_for_search(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(normalize_char)
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
}

fn make_fts_query(normalized: &str) -> String {
    if normalized.chars().all(|ch| ch.is_ascii_alphanumeric()) {
        format!("{normalized}*")
    } else {
        format!("\"{}\"", normalized.replace('"', "\"\""))
    }
}

fn normalize_char(ch: char) -> char {
    match ch {
        '\u{3000}' => ' ',
        'Ａ'..='Ｚ' => char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch),
        'ａ'..='ｚ' => char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch),
        '０'..='９' => char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch),
        _ => ch,
    }
}

fn empty_to_none(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn relation_type_for(index: u32) -> &'static str {
    match index % 5 {
        0 => "contains",
        1 => "belongs_to",
        2 => "related_to",
        3 => "similar_to",
        _ => "references",
    }
}

pub fn build_terms_from_item(item: &IndexableKnowledgeItem) -> Vec<SearchTerm> {
    let mut seen = HashSet::new();
    let mut terms = Vec::new();
    push_term(&mut terms, &mut seen, &item.name, "name", 100);
    push_optional(&mut terms, &mut seen, item.code.as_deref(), "code", 95);
    push_optional(&mut terms, &mut seen, item.pinyin.as_deref(), "pinyin", 90);
    push_optional(
        &mut terms,
        &mut seen,
        item.category.as_deref(),
        "category",
        55,
    );
    push_optional(
        &mut terms,
        &mut seen,
        item.source_note.as_deref(),
        "source_note",
        60,
    );
    push_split_terms(&mut terms, &mut seen, item.alias.as_deref(), "alias", 85);
    push_split_terms(&mut terms, &mut seen, item.tags.as_deref(), "tags", 45);
    let detail_search = detail_text(item.detail.as_deref());
    push_split_terms(&mut terms, &mut seen, Some(&detail_search), "detail", 35);
    push_known_normalized_terms(&mut terms, &mut seen, item);
    terms
}

fn annotation_search_text(connection: &Connection, item_id: i64) -> AppResult<String> {
    let mut statement = connection.prepare(
        "SELECT source_title, source_note, content, tags_json
         FROM knowledge_annotations
         WHERE knowledge_item_id = ?1",
    )?;
    let rows = statement.query_map(params![item_id], |row| {
        Ok([
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" "))
    })?;
    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }
    Ok(values.join(" "))
}

fn push_optional(
    terms: &mut Vec<SearchTerm>,
    seen: &mut HashSet<String>,
    value: Option<&str>,
    term_type: &str,
    weight: i64,
) {
    if let Some(value) = value {
        push_term(terms, seen, value, term_type, weight);
    }
}

fn push_split_terms(
    terms: &mut Vec<SearchTerm>,
    seen: &mut HashSet<String>,
    value: Option<&str>,
    term_type: &str,
    weight: i64,
) {
    if let Some(value) = value {
        for part in value.split([',', '，', ';', '；', '|', '/', '、', ' ']) {
            push_term(terms, seen, part, term_type, weight);
        }
        push_term(terms, seen, value, term_type, weight - 5);
    }
}

fn push_known_normalized_terms(
    terms: &mut Vec<SearchTerm>,
    seen: &mut HashSet<String>,
    item: &IndexableKnowledgeItem,
) {
    let haystack = [
        item.name.as_str(),
        item.code.as_deref().unwrap_or_default(),
        item.alias.as_deref().unwrap_or_default(),
        item.pinyin.as_deref().unwrap_or_default(),
        item.category.as_deref().unwrap_or_default(),
        item.source_note.as_deref().unwrap_or_default(),
        item.tags.as_deref().unwrap_or_default(),
        item.detail.as_deref().unwrap_or_default(),
    ]
    .join(" ");

    let normalized = normalize_for_search(&haystack);
    let known_terms = [
        ("足三里", "normalized", 110),
        ("st36", "normalized", 110),
        ("zusanli", "normalized", 110),
        ("足阳明胃经", "normalized", 100),
        ("胃经", "normalized", 100),
        ("黄芪", "normalized", 100),
        ("黄耆", "normalized", 100),
        ("huangqi", "normalized", 95),
        ("补中益气汤", "normalized", 100),
        ("buzhongyiqitang", "normalized", 95),
    ];

    for (term, term_type, weight) in known_terms {
        let normalized_term = normalize_for_search(term);
        if normalized.contains(&normalized_term) {
            push_term(terms, seen, term, term_type, weight);
        }
    }
}

fn detail_text(value: Option<&str>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    let parsed = serde_json::from_str::<serde_json::Value>(value);
    match parsed {
        Ok(value) => flatten_json_strings(&value).join(" "),
        Err(_) => value.to_string(),
    }
}

fn flatten_json_strings(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(text) if !text.trim().is_empty() => vec![text.trim().to_string()],
        serde_json::Value::Number(number) => vec![number.to_string()],
        serde_json::Value::Bool(value) => vec![value.to_string()],
        serde_json::Value::Array(values) => values.iter().flat_map(flatten_json_strings).collect(),
        serde_json::Value::Object(map) => map.values().flat_map(flatten_json_strings).collect(),
        _ => Vec::new(),
    }
}

fn push_term(
    terms: &mut Vec<SearchTerm>,
    seen: &mut HashSet<String>,
    value: &str,
    term_type: &str,
    weight: i64,
) {
    let normalized = normalize_for_search(value);
    if normalized.is_empty() || !seen.insert(format!("{term_type}:{normalized}")) {
        return;
    }
    terms.push(SearchTerm {
        term: normalized,
        term_type: term_type.to_string(),
        weight,
    });
}
