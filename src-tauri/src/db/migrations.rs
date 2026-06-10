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
    Migration {
        version: 12,
        name: "herb_structured_properties",
        sql: include_str!("../../migrations/012_herb_structured_properties.sql"),
    },
    Migration {
        version: 13,
        name: "herb_classic_sections",
        sql: include_str!("../../migrations/013_herb_classic_sections.sql"),
    },
    Migration {
        version: 14,
        name: "user_notes_multi_entry",
        sql: include_str!("../../migrations/014_user_notes_multi_entry.sql"),
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

#[cfg(test)]
mod tests {
    use super::run;
    use rusqlite::Connection;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn user_notes_allow_multiple_notes_per_item_after_migrations() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let data_dir = std::env::temp_dir().join(format!("zhongyi-migration-notes-{unique}"));
        fs::create_dir_all(&data_dir).expect("create temp dir");
        let database_path = data_dir.join("test.db");
        let connection = Connection::open(&database_path).expect("open db");

        run(&connection).expect("migrations run");
        connection
            .execute(
                "INSERT INTO knowledge_items
                 (type, name, data_status, completeness_status, created_at, updated_at)
                 VALUES ('note', '笔记迁移测试', 'imported', 'partial', datetime('now'), datetime('now'))",
                [],
            )
            .expect("insert item");
        let item_id = connection.last_insert_rowid();

        for text in ["第一条", "第二条"] {
            connection
                .execute(
                    "INSERT INTO user_notes (item_id, note_text, created_at, updated_at)
                     VALUES (?1, ?2, datetime('now'), datetime('now'))",
                    rusqlite::params![item_id, text],
                )
                .expect("insert note");
        }

        let count: i64 = connection
            .query_row(
                "SELECT COUNT(1) FROM user_notes WHERE item_id = ?1",
                [item_id],
                |row| row.get(0),
            )
            .expect("count notes");
        assert_eq!(count, 2);

        drop(connection);
        let _ = fs::remove_dir_all(data_dir);
    }
}
