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
