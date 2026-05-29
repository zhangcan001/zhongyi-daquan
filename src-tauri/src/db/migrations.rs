use crate::errors::AppResult;
use rusqlite::{params, Connection};

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "initial_core_schema",
    sql: include_str!("../../migrations/001_initial_core_schema.sql"),
}];

pub fn run(connection: &Connection) -> AppResult<()> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at TEXT NOT NULL
        );",
    )?;

    for migration in MIGRATIONS {
        let exists: i64 = connection.query_row(
            "SELECT COUNT(1) FROM schema_migrations WHERE version = ?1",
            params![migration.version],
            |row| row.get(0),
        )?;

        if exists == 0 {
            let transaction = connection.unchecked_transaction()?;
            transaction.execute_batch(migration.sql)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version, name, applied_at)
                 VALUES (?1, ?2, datetime('now'))",
                params![migration.version, migration.name],
            )?;
            transaction.commit()?;
        }
    }

    Ok(())
}
