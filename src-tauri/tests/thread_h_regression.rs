use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn open_test_database(name: &str) -> (PathBuf, Connection) {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("zhongyi-thread-h-{name}-{unique}"));
    fs::create_dir_all(root.join("database")).expect("create test database dir");
    let connection = Connection::open(root.join("database/zhongyi.db")).expect("open sqlite");
    initialize(&connection).expect("initialize schema");
    (root, connection)
}

fn initialize(connection: &Connection) -> rusqlite::Result<()> {
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "temp_store", "MEMORY")?;
    connection.pragma_update(None, "cache_size", -64000)?;
    connection.execute_batch(include_str!("../migrations/001_initial_core_schema.sql"))?;
    connection.execute_batch(include_str!("../migrations/002_ai_reserved_schema.sql"))?;
    connection.execute_batch(include_str!(
        "../migrations/003_thread_d_search_performance.sql"
    ))?;
    Ok(())
}

#[test]
fn database_initialization_creates_core_schema() {
    let (root, connection) = open_test_database("init");

    let foreign_keys: i64 = connection
        .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
        .expect("foreign keys pragma");
    assert_eq!(foreign_keys, 1);

    for table in [
        "knowledge_items",
        "knowledge_relations",
        "data_import_rows",
        "duplicate_candidates",
        "relation_suggestions",
        "knowledge_fts",
        "search_terms",
        "ai_provider_settings",
    ] {
        let exists: i64 = connection
            .query_row(
                "SELECT COUNT(1) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                params![table],
                |row| row.get(0),
            )
            .expect("sqlite_master query");
        assert_eq!(exists, 1, "missing table {table}");
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn search_smoke_hits_seeded_terms() {
    let (root, connection) = open_test_database("search");
    seed_search_item(&connection).expect("seed search item");

    let term_hit: i64 = connection
        .query_row(
            "SELECT COUNT(1)
             FROM search_terms st
             JOIN knowledge_list_view_cache lc ON lc.item_id = st.item_id
             WHERE lc.data_status IN ('validated', 'ready') AND st.term = '足三里'",
            [],
            |row| row.get(0),
        )
        .expect("term search");
    assert_eq!(term_hit, 1);

    let fts_hit: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM knowledge_fts WHERE knowledge_fts MATCH '\"足三里\"'",
            [],
            |row| row.get(0),
        )
        .expect("fts search");
    assert_eq!(fts_hit, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn import_flow_stages_rows_and_validation_issues() {
    let (root, connection) = open_test_database("import");
    connection
        .execute(
            "INSERT INTO data_import_batches
             (file_name, import_type, target_type, status, total_count, parsed_count,
              valid_count, warning_count, error_count, created_at)
             VALUES ('sample.csv', 'csv', 'herb', 'staged', 2, 2, 1, 1, 0, datetime('now'))",
            [],
        )
        .expect("insert batch");
    let batch_id = connection.last_insert_rowid();

    connection
        .execute(
            "INSERT INTO data_import_rows
             (batch_id, row_index, raw_json, mapped_json, normalized_json, status, warning_message)
             VALUES (?1, 1, '{\"药名\":\"黄芪\"}', '{\"name\":\"黄芪\"}',
                     '{\"name\":\"黄芪\",\"type\":\"herb\"}', 'valid', NULL)",
            params![batch_id],
        )
        .expect("insert valid row");
    let row_id = connection.last_insert_rowid();
    connection
        .execute(
            "INSERT INTO data_validation_issues
             (batch_id, row_id, severity, issue_code, field_name, message, suggestion)
             VALUES (?1, ?2, 'warning', 'MISSING_PINYIN', 'pinyin', '缺少拼音', '可后续补充')",
            params![batch_id, row_id],
        )
        .expect("insert issue");

    let rows: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM data_import_rows WHERE batch_id = ?1 AND status = 'valid'",
            params![batch_id],
            |row| row.get(0),
        )
        .expect("count import rows");
    let issues: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM data_validation_issues WHERE batch_id = ?1",
            params![batch_id],
            |row| row.get(0),
        )
        .expect("count issues");
    assert_eq!(rows, 1);
    assert_eq!(issues, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn backup_restore_preserves_searchable_data() {
    let (root, connection) = open_test_database("backup");
    seed_search_item(&connection).expect("seed search item");
    drop(connection);

    let db_path = root.join("database/zhongyi.db");
    let backup_path = root.join("backups/zhongyi-backup.db");
    fs::create_dir_all(root.join("backups")).expect("create backups dir");
    fs::copy(&db_path, &backup_path).expect("copy backup");
    fs::remove_file(&db_path).expect("remove original db");
    fs::copy(&backup_path, &db_path).expect("restore backup");

    let restored = Connection::open(&db_path).expect("open restored db");
    let hit: i64 = restored
        .query_row(
            "SELECT COUNT(1)
             FROM search_terms st
             JOIN knowledge_list_view_cache lc ON lc.item_id = st.item_id
             WHERE st.term = 'st36' AND lc.name = '足三里'",
            [],
            |row| row.get(0),
        )
        .expect("restored search terms");
    assert_eq!(hit, 1);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn ai_placeholder_defaults_are_disabled() {
    let (root, connection) = open_test_database("ai");
    let enabled_count: i64 = connection
        .query_row(
            "SELECT COUNT(1) FROM ai_provider_settings WHERE enabled = 1",
            [],
            |row| row.get(0),
        )
        .expect("count enabled providers");
    assert_eq!(enabled_count, 0);

    let _ = fs::remove_dir_all(root);
}

fn seed_search_item(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO knowledge_items
         (type, code, name, alias, pinyin, category, summary, content, tags,
          data_status, completeness_status, created_at, updated_at)
         VALUES ('acupoint', 'ST36', '足三里', '足三里穴', 'zusanli', '足阳明胃经',
                 '常用穴', '足三里用于资料整理测试。', '胃经,常用穴',
                 'ready', 'complete', datetime('now'), datetime('now'))",
        [],
    )?;
    let item_id = connection.last_insert_rowid();
    connection.execute(
        "INSERT INTO knowledge_fts
         (rowid, name, code, alias, pinyin, category, summary, content, tags)
         VALUES (?1, '足三里', 'ST36', '足三里穴', 'zusanli', '足阳明胃经',
                 '常用穴', '足三里用于资料整理测试。', '胃经,常用穴')",
        params![item_id],
    )?;
    connection.execute(
        "INSERT INTO knowledge_list_view_cache
         (item_id, type, code, name, pinyin, category, summary, tags,
          data_status, is_favorite, relation_count, updated_at)
         VALUES (?1, 'acupoint', 'ST36', '足三里', 'zusanli', '足阳明胃经',
                 '常用穴', '胃经,常用穴', 'ready', 0, 0, datetime('now'))",
        params![item_id],
    )?;
    for (term, term_type, weight) in [
        ("足三里", "name", 100),
        ("st36", "code", 95),
        ("zusanli", "pinyin", 90),
        ("足阳明胃经", "category", 55),
        ("胃经", "tags", 45),
    ] {
        connection.execute(
            "INSERT INTO search_terms (item_id, term, term_type, weight)
             VALUES (?1, ?2, ?3, ?4)",
            params![item_id, term, term_type, weight],
        )?;
    }
    Ok(())
}
