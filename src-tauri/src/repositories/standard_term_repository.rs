use crate::db::connection::Database;
use crate::errors::AppResult;
use rusqlite::params;

pub fn ensure_basic_terms(database: &Database) -> AppResult<()> {
    database.with_connection(|connection| {
        for (term_type, standard_name, aliases, code, notes) in [
            (
                "meridian",
                "足阳明胃经",
                "胃经,ST,st",
                Some("ST"),
                Some("线程 C 基础标准词"),
            ),
            ("herb_name", "黄芪", "黄耆", None, Some("线程 C 基础标准词")),
        ] {
            let exists: i64 = connection.query_row(
                "SELECT COUNT(1) FROM standard_terms WHERE term_type = ?1 AND standard_name = ?2",
                params![term_type, standard_name],
                |row| row.get(0),
            )?;
            if exists == 0 {
                connection.execute(
                    "INSERT INTO standard_terms(term_type, standard_name, aliases, code, notes)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![term_type, standard_name, aliases, code, notes],
                )?;
            }
        }
        Ok(())
    })
}

pub fn standardize(database: &Database, term_type: &str, input: &str) -> AppResult<Option<String>> {
    let needle = input.trim();
    if needle.is_empty() {
        return Ok(None);
    }

    database.with_connection(|connection| {
        let mut statement = connection.prepare(
            "SELECT standard_name, aliases, code FROM standard_terms WHERE term_type = ?1",
        )?;
        let terms = statement.query_map([term_type], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;

        for term in terms {
            let (standard_name, aliases, code) = term?;
            if standard_name == needle || code.as_deref() == Some(needle) {
                return Ok(Some(standard_name));
            }
            if let Some(aliases) = aliases {
                if aliases
                    .split(',')
                    .map(str::trim)
                    .any(|alias| alias.eq_ignore_ascii_case(needle))
                {
                    return Ok(Some(standard_name));
                }
            }
        }
        Ok(None)
    })
}
