use crate::errors::AppResult;
use rusqlite::{params, Connection};

struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial_core_schema",
        sql: include_str!("../../migrations/001_initial_core_schema.sql"),
    },
    Migration {
        version: 2,
        name: "ai_reserved_schema",
        sql: include_str!("../../migrations/002_ai_reserved_schema.sql"),
    },
    Migration {
        version: 3,
        name: "thread_d_search_performance",
        sql: include_str!("../../migrations/003_thread_d_search_performance.sql"),
    },
    Migration {
        version: 4,
        name: "thread_e_dedup_relation",
        sql: include_str!("../../migrations/004_thread_e_dedup_relation.sql"),
    },
    Migration {
        version: 5,
        name: "error_logs",
        sql: include_str!("../../migrations/005_error_logs.sql"),
    },
    Migration {
        version: 6,
        name: "import_quality_v1",
        sql: include_str!("../../migrations/006_import_quality_v1.sql"),
    },
    Migration {
        version: 7,
        name: "smart_import_center_v1",
        sql: include_str!("../../migrations/007_smart_import_center_v1.sql"),
    },
    Migration {
        version: 8,
        name: "import_runs_v1",
        sql: include_str!("../../migrations/008_import_runs_v1.sql"),
    },
    Migration {
        version: 9,
        name: "renji_manifest_package_compat",
        sql: include_str!("../../migrations/009_renji_manifest_package_compat.sql"),
    },
    Migration {
        version: 10,
        name: "ux_polish_v012",
        sql: include_str!("../../migrations/010_ux_polish_v012.sql"),
    },
    Migration {
        version: 11,
        name: "ai_openai_compatible_v020",
        sql: include_str!("../../migrations/011_ai_openai_compatible_v020.sql"),
    },
];

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
