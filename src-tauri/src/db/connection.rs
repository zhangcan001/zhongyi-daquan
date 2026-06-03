use crate::db::migrations;
use crate::errors::{AppError, AppResult};
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct Database {
    connection: Mutex<Connection>,
    path: PathBuf,
}

impl Database {
    pub fn initialize(data_dir: &Path) -> AppResult<Self> {
        let database_dir = data_dir.join("database");
        for child in [
            "images", "imports", "exports", "backups", "logs", "config", "temp",
        ] {
            fs::create_dir_all(data_dir.join(child))?;
        }
        fs::create_dir_all(&database_dir)?;

        let path = database_dir.join("zhongyi.db");
        let connection = Connection::open(&path)?;
        initialize_pragmas(&connection)?;
        migrations::run(&connection)?;

        Ok(Self {
            connection: Mutex::new(connection),
            path,
        })
    }

    pub fn with_connection<T>(&self, f: impl FnOnce(&Connection) -> AppResult<T>) -> AppResult<T> {
        let guard = self
            .connection
            .lock()
            .map_err(|_| AppError::DatabaseLock("数据库连接锁已损坏".to_string()))?;
        f(&guard)
    }

    pub fn reopen(&self) -> AppResult<Self> {
        let connection = Connection::open(&self.path)?;
        initialize_pragmas(&connection)?;
        migrations::run(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            path: self.path.clone(),
        })
    }

    pub fn replace_database_file(&self, replacement_path: &Path) -> AppResult<()> {
        let mut guard = self
            .connection
            .lock()
            .map_err(|_| AppError::DatabaseLock("数据库连接锁已损坏".to_string()))?;
        let old_connection = std::mem::replace(&mut *guard, Connection::open_in_memory()?);
        drop(old_connection);

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let _ = fs::remove_file(&self.path);
        let _ = fs::remove_file(self.path.with_extension("db-wal"));
        let _ = fs::remove_file(self.path.with_extension("db-shm"));
        fs::copy(replacement_path, &self.path)?;

        let reopened = Connection::open(&self.path)?;
        initialize_pragmas(&reopened)?;
        migrations::run(&reopened)?;
        *guard = reopened;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn initialize_pragmas(connection: &Connection) -> AppResult<()> {
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "NORMAL")?;
    connection.pragma_update(None, "foreign_keys", "ON")?;
    connection.pragma_update(None, "temp_store", "MEMORY")?;
    connection.pragma_update(None, "cache_size", -64000)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Database;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_data_dir(test_name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be available")
            .as_nanos();
        std::env::temp_dir().join(format!("zhongyi-daquan-{test_name}-{unique}"))
    }

    #[test]
    fn initializes_database_with_required_pragmas_schema_indexes_and_fts() {
        let data_dir = temp_data_dir("init");
        let database = Database::initialize(&data_dir).expect("database should initialize");

        database
            .with_connection(|connection| {
                let journal_mode: String =
                    connection.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
                let synchronous: i64 =
                    connection.query_row("PRAGMA synchronous", [], |row| row.get(0))?;
                let foreign_keys: i64 =
                    connection.query_row("PRAGMA foreign_keys", [], |row| row.get(0))?;
                let temp_store: i64 =
                    connection.query_row("PRAGMA temp_store", [], |row| row.get(0))?;
                let cache_size: i64 =
                    connection.query_row("PRAGMA cache_size", [], |row| row.get(0))?;

                assert_eq!(journal_mode.to_uppercase(), "WAL");
                assert_eq!(synchronous, 1);
                assert_eq!(foreign_keys, 1);
                assert_eq!(temp_store, 2);
                assert_eq!(cache_size, -64000);

                for table in [
                    "knowledge_items",
                    "herb_details",
                    "formula_details",
                    "meridian_details",
                    "acupoint_details",
                    "syndrome_details",
                    "disease_details",
                    "data_import_batches",
                    "data_import_rows",
                    "data_validation_issues",
                    "field_mapping_templates",
                    "standard_terms",
                    "validation_rules",
                    "data_transform_steps",
                    "data_transform_row_changes",
                    "duplicate_candidates",
                    "merge_records",
                    "knowledge_fingerprints",
                    "relation_suggestions",
                    "knowledge_relations",
                    "relation_count_cache",
                    "knowledge_fts",
                    "search_terms",
                    "knowledge_list_view_cache",
                    "knowledge_versions",
                    "background_jobs",
                    "performance_logs",
                    "audit_logs",
                    "ai_provider_settings",
                    "ai_prompt_templates",
                    "ai_tasks",
                    "ai_drafts",
                    "ai_call_logs",
                    "recent_views",
                    "user_favorites",
                    "user_notes",
                ] {
                    let exists: i64 = connection.query_row(
                        "SELECT COUNT(1) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                        [table],
                        |row| row.get(0),
                    )?;
                    assert_eq!(exists, 1, "missing table {table}");
                }

                for index in [
                    "idx_knowledge_type",
                    "idx_knowledge_status",
                    "idx_knowledge_type_status",
                    "idx_knowledge_code",
                    "idx_knowledge_name",
                    "idx_knowledge_pinyin",
                    "idx_knowledge_category",
                    "idx_knowledge_updated_at",
                    "idx_rel_source",
                    "idx_rel_target",
                    "idx_rel_type",
                    "idx_rel_source_type",
                    "idx_import_rows_batch",
                    "idx_import_rows_status",
                    "idx_import_issues_batch",
                    "idx_duplicate_batch",
                    "idx_search_terms_term",
                    "idx_search_terms_item",
                    "idx_search_terms_type",
                ] {
                    let exists: i64 = connection.query_row(
                        "SELECT COUNT(1) FROM sqlite_master WHERE type = 'index' AND name = ?1",
                        [index],
                        |row| row.get(0),
                    )?;
                    assert_eq!(exists, 1, "missing index {index}");
                }

                connection.execute(
                    "INSERT INTO knowledge_fts(rowid, name, code, alias, pinyin, category, summary, content, tags)
                     VALUES (1, '黄芪', 'HERB-HQ', '黄耆', 'huang qi', '补气药', '补气固表', '黄芪资料', '中药')",
                    [],
                )?;
                let hit_count: i64 = connection.query_row(
                    "SELECT COUNT(1) FROM knowledge_fts WHERE knowledge_fts MATCH '黄芪'",
                    [],
                    |row| row.get(0),
                )?;
                assert_eq!(hit_count, 1);

                Ok(())
            })
            .expect("schema checks should pass");

        Database::initialize(&data_dir).expect("database should initialize twice");
        drop(database);
        let _ = fs::remove_dir_all(data_dir);
    }
}
