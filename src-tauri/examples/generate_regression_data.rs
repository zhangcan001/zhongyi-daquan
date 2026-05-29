use rusqlite::{params, Connection};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

const KNOWLEDGE_COUNT: u32 = 10_000;
const RELATION_COUNT: u32 = 50_000;
const IMPORT_ROW_COUNT: u32 = 10_000;
const DUPLICATE_COUNT: u32 = 1_000;
const SUGGESTION_COUNT: u32 = 1_000;

fn main() -> rusqlite::Result<()> {
    let options = Options::from_env();
    if let Some(parent) = options.db_path.parent() {
        fs::create_dir_all(parent).expect("failed to create database directory");
    }
    if options.reset && options.db_path.exists() {
        fs::remove_file(&options.db_path).expect("failed to remove old database");
    }

    let started_at = Instant::now();
    let mut connection = Connection::open(&options.db_path)?;
    initialize_pragmas(&connection)?;
    run_migrations(&connection)?;
    let counts = seed(&mut connection, options.reset)?;

    println!("database: {}", options.db_path.display());
    println!("knowledge_items: {}", counts.knowledge_items);
    println!("knowledge_relations: {}", counts.knowledge_relations);
    println!("data_import_rows: {}", counts.data_import_rows);
    println!("duplicate_candidates: {}", counts.duplicate_candidates);
    println!("relation_suggestions: {}", counts.relation_suggestions);
    println!("duration_ms: {}", started_at.elapsed().as_millis());

    if options.check_performance {
        check_performance(&connection)?;
    }

    Ok(())
}

struct Options {
    db_path: PathBuf,
    reset: bool,
    check_performance: bool,
}

impl Options {
    fn from_env() -> Self {
        let mut db_path = PathBuf::from("local-data/database/thread_h_regression.db");
        let mut reset = true;
        let mut check_performance = false;
        let args = env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--db" => {
                    if let Some(value) = args.get(index + 1) {
                        db_path = PathBuf::from(value);
                        index += 1;
                    }
                }
                "--append" => reset = false,
                "--check-performance" => check_performance = true,
                _ => {}
            }
            index += 1;
        }

        Self {
            db_path,
            reset,
            check_performance,
        }
    }
}

struct SeedCounts {
    knowledge_items: i64,
    knowledge_relations: i64,
    data_import_rows: i64,
    duplicate_candidates: i64,
    relation_suggestions: i64,
}

fn initialize_pragmas(connection: &Connection) -> rusqlite::Result<()> {
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "temp_store", "MEMORY")?;
    connection.pragma_update(None, "cache_size", -64000)?;
    Ok(())
}

fn run_migrations(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(include_str!("../migrations/001_initial_core_schema.sql"))?;
    connection.execute_batch(include_str!("../migrations/002_ai_reserved_schema.sql"))?;
    connection.execute_batch(include_str!(
        "../migrations/003_thread_d_search_performance.sql"
    ))?;
    Ok(())
}

fn seed(connection: &mut Connection, reset: bool) -> rusqlite::Result<SeedCounts> {
    let tx = connection.transaction()?;
    if reset {
        clear_seed_tables(&tx)?;
    }

    let batch_id = insert_import_batch(&tx)?;
    let first_item_id = insert_knowledge_items(&tx)?;
    insert_import_rows(&tx, batch_id)?;
    insert_relations(&tx, first_item_id)?;
    insert_duplicate_candidates(&tx, batch_id, first_item_id)?;
    insert_relation_suggestions(&tx, first_item_id)?;
    rebuild_relation_cache(&tx)?;
    tx.commit()?;

    Ok(SeedCounts {
        knowledge_items: count(connection, "knowledge_items")?,
        knowledge_relations: count(connection, "knowledge_relations")?,
        data_import_rows: count(connection, "data_import_rows")?,
        duplicate_candidates: count(connection, "duplicate_candidates")?,
        relation_suggestions: count(connection, "relation_suggestions")?,
    })
}

fn clear_seed_tables(connection: &Connection) -> rusqlite::Result<()> {
    for table in [
        "relation_suggestions",
        "duplicate_candidates",
        "data_import_rows",
        "data_import_batches",
        "knowledge_relations",
        "knowledge_list_view_cache",
        "relation_count_cache",
        "search_terms",
        "knowledge_fts",
        "knowledge_items",
    ] {
        connection.execute(&format!("DELETE FROM {table}"), [])?;
    }
    Ok(())
}

fn insert_import_batch(connection: &Connection) -> rusqlite::Result<i64> {
    connection.execute(
        "INSERT INTO data_import_batches
         (file_name, import_type, target_type, status, total_count, parsed_count,
          valid_count, warning_count, error_count, created_at)
         VALUES ('thread_h_seed.csv', 'csv', 'mixed', 'staged', ?1, ?1, ?1, 0, 0, datetime('now'))",
        params![IMPORT_ROW_COUNT],
    )?;
    Ok(connection.last_insert_rowid())
}

fn insert_knowledge_items(connection: &Connection) -> rusqlite::Result<i64> {
    let first_id = connection.query_row(
        "SELECT COALESCE(MAX(id), 0) + 1 FROM knowledge_items",
        [],
        |row| row.get::<_, i64>(0),
    )?;
    let base = [
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

    for index in 0..KNOWLEDGE_COUNT {
        let row = if (index as usize) < base.len() {
            let item = base[index as usize];
            (
                item.0.to_string(),
                item.1.to_string(),
                item.2.to_string(),
                item.3.to_string(),
                item.4.to_string(),
                item.5.to_string(),
                format!("{} 的回归测试摘要", item.2),
                format!("{} 用于搜索、导入、关系和备份恢复测试。", item.2),
                item.6.to_string(),
            )
        } else {
            let no = index + 1;
            (
                types[index as usize % types.len()].to_string(),
                format!("T{no:05}"),
                format!("测试知识{no:05}"),
                format!("别名{no:05}"),
                format!("ceshizhishi{no:05}"),
                format!("分类{}", index % 20),
                format!("第 {no} 条测试知识摘要"),
                format!("内容字段用于回归数据生成器验证，编号为 T{no:05}。"),
                format!("标签{},回归测试", index % 30),
            )
        };

        connection.execute(
            "INSERT INTO knowledge_items
             (type, code, name, alias, pinyin, category, summary, content, tags,
              data_status, completeness_status, is_favorite, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9,
                     'ready', 'complete', 0, datetime('now'), datetime('now'))",
            params![
                row.0,
                row.1,
                row.2,
                empty_to_null(&row.3),
                row.4,
                row.5,
                row.6,
                row.7,
                row.8
            ],
        )?;
        let item_id = connection.last_insert_rowid();
        insert_search_documents(connection, item_id, &row)?;
    }

    Ok(first_id)
}

fn insert_search_documents(
    connection: &Connection,
    item_id: i64,
    row: &(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ),
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO knowledge_fts
         (rowid, name, code, alias, pinyin, category, summary, content, tags)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            item_id,
            row.2,
            row.1,
            empty_to_null(&row.3),
            row.4,
            row.5,
            row.6,
            row.7,
            row.8
        ],
    )?;
    connection.execute(
        "INSERT INTO knowledge_list_view_cache
         (item_id, type, code, name, pinyin, category, summary, tags,
          data_status, is_favorite, relation_count, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'ready', 0, 0, datetime('now'))",
        params![item_id, row.0, row.1, row.2, row.4, row.5, row.6, row.8],
    )?;

    for (term, term_type, weight) in search_terms(row) {
        connection.execute(
            "INSERT INTO search_terms (item_id, term, term_type, weight)
             VALUES (?1, ?2, ?3, ?4)",
            params![item_id, term, term_type, weight],
        )?;
    }
    Ok(())
}

fn search_terms(
    row: &(
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ),
) -> Vec<(String, &'static str, i64)> {
    let mut terms = vec![
        (normalize(&row.2), "name", 100),
        (normalize(&row.1), "code", 95),
        (normalize(&row.4), "pinyin", 90),
        (normalize(&row.5), "category", 55),
    ];
    for part in row.3.split(['，', ',', '、', ';', '；', '|', '/']) {
        terms.push((normalize(part), "alias", 85));
    }
    for part in row.8.split(['，', ',', '、', ';', '；', '|', '/']) {
        terms.push((normalize(part), "tags", 45));
    }
    terms.retain(|(term, _, _)| !term.is_empty());
    terms.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(right.1)));
    terms.dedup_by(|left, right| left.0 == right.0 && left.1 == right.1);
    terms
}

fn insert_import_rows(connection: &Connection, batch_id: i64) -> rusqlite::Result<()> {
    for index in 0..IMPORT_ROW_COUNT {
        let no = index + 1;
        connection.execute(
            "INSERT INTO data_import_rows
             (batch_id, row_index, raw_json, mapped_json, normalized_json, status,
              error_message, warning_message)
             VALUES (?1, ?2, ?3, ?4, ?5, 'valid', NULL, NULL)",
            params![
                batch_id,
                no,
                format!(r#"{{"name":"导入知识{no:05}","code":"I{no:05}"}}"#),
                format!(r#"{{"name":"导入知识{no:05}","type":"herb"}}"#),
                format!(r#"{{"name":"导入知识{no:05}","type":"herb","status":"ready"}}"#)
            ],
        )?;
    }
    Ok(())
}

fn insert_relations(connection: &Connection, first_item_id: i64) -> rusqlite::Result<()> {
    for index in 0..RELATION_COUNT {
        let source = first_item_id + i64::from(index % KNOWLEDGE_COUNT);
        let mut target = first_item_id + i64::from((index * 37 + 11) % KNOWLEDGE_COUNT);
        if source == target {
            target = first_item_id + i64::from((index + 1) % KNOWLEDGE_COUNT);
        }
        connection.execute(
            "INSERT INTO knowledge_relations
             (source_item_id, target_item_id, relation_type, note)
             VALUES (?1, ?2, ?3, '线程 H 回归关系')",
            params![source, target, relation_type(index)],
        )?;
    }
    Ok(())
}

fn insert_duplicate_candidates(
    connection: &Connection,
    batch_id: i64,
    first_item_id: i64,
) -> rusqlite::Result<()> {
    let first_row_id: i64 = connection.query_row(
        "SELECT MIN(id) FROM data_import_rows WHERE batch_id = ?1",
        params![batch_id],
        |row| row.get(0),
    )?;
    for index in 0..DUPLICATE_COUNT {
        connection.execute(
            "INSERT INTO duplicate_candidates
             (batch_id, existing_item_id, imported_row_id, match_type, match_score, reason, status, created_at)
             VALUES (?1, ?2, ?3, 'name_code', ?4, '线程 H 测试重复候选', 'pending', datetime('now'))",
            params![
                batch_id,
                first_item_id + i64::from(index % KNOWLEDGE_COUNT),
                first_row_id + i64::from(index),
                0.70 + f64::from(index % 30) / 100.0
            ],
        )?;
    }
    Ok(())
}

fn insert_relation_suggestions(
    connection: &Connection,
    first_item_id: i64,
) -> rusqlite::Result<()> {
    for index in 0..SUGGESTION_COUNT {
        let source = first_item_id + i64::from(index % KNOWLEDGE_COUNT);
        let target = first_item_id + i64::from((index * 17 + 5) % KNOWLEDGE_COUNT);
        connection.execute(
            "INSERT INTO relation_suggestions
             (source_item_id, target_item_id, relation_type, confidence, reason, status, created_at)
             VALUES (?1, ?2, ?3, ?4, '线程 H 测试关系建议', 'pending', datetime('now'))",
            params![
                source,
                target,
                relation_type(index),
                0.60 + f64::from(index % 35) / 100.0
            ],
        )?;
    }
    Ok(())
}

fn rebuild_relation_cache(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute("DELETE FROM relation_count_cache", [])?;
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

fn check_performance(connection: &Connection) -> rusqlite::Result<()> {
    let search_ms = elapsed_ms(|| {
        let count: i64 = connection.query_row(
            "SELECT COUNT(1)
             FROM search_terms st
             JOIN knowledge_list_view_cache lc ON lc.item_id = st.item_id
             WHERE lc.data_status IN ('validated', 'ready') AND st.term LIKE '测试知识09999%'",
            [],
            |row| row.get(0),
        )?;
        assert!(count >= 1);
        Ok(())
    })?;

    let list_ms = elapsed_ms(|| {
        let mut statement = connection.prepare(
            "SELECT item_id FROM knowledge_list_view_cache
             WHERE data_status IN ('validated', 'ready')
             ORDER BY updated_at DESC, item_id DESC
             LIMIT 50 OFFSET 950",
        )?;
        let rows = statement.query_map([], |row| row.get::<_, i64>(0))?;
        assert_eq!(rows.collect::<Result<Vec<_>, _>>()?.len(), 50);
        Ok(())
    })?;

    let relation_ms = elapsed_ms(|| {
        let mut statement = connection.prepare(
            "SELECT target_item_id, relation_type
             FROM knowledge_relations
             WHERE source_item_id = (SELECT MIN(id) FROM knowledge_items)
             ORDER BY id
             LIMIT 50",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        assert!(!rows.collect::<Result<Vec<_>, _>>()?.is_empty());
        Ok(())
    })?;

    println!("performance_search_ms: {search_ms}");
    println!("performance_list_page_ms: {list_ms}");
    println!("performance_relation_first_page_ms: {relation_ms}");
    assert_threshold(search_ms, 500, "search 10,000 knowledge");
    assert_threshold(list_ms, 300, "knowledge list paging");
    assert_threshold(relation_ms, 500, "50,000 relation first page");
    Ok(())
}

fn elapsed_ms(action: impl FnOnce() -> rusqlite::Result<()>) -> rusqlite::Result<u128> {
    let started_at = Instant::now();
    action()?;
    Ok(started_at.elapsed().as_millis())
}

fn assert_threshold(actual: u128, threshold: u128, label: &str) {
    if actual >= threshold {
        panic!("{label} expected < {threshold}ms, got {actual}ms");
    }
}

fn count(connection: &Connection, table: &str) -> rusqlite::Result<i64> {
    connection.query_row(&format!("SELECT COUNT(1) FROM {table}"), [], |row| {
        row.get(0)
    })
}

fn relation_type(index: u32) -> &'static str {
    match index % 5 {
        0 => "contains",
        1 => "belongs_to",
        2 => "related_to",
        3 => "similar_to",
        _ => "references",
    }
}

fn empty_to_null(value: &str) -> Option<&str> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn normalize(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| match ch {
            '\u{3000}' => ' ',
            'Ａ'..='Ｚ' | 'ａ'..='ｚ' | '０'..='９' => {
                char::from_u32(ch as u32 - 0xFEE0).unwrap_or(ch)
            }
            _ => ch,
        })
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
}

#[allow(dead_code)]
fn path_exists(path: &Path) -> bool {
    path.exists()
}
