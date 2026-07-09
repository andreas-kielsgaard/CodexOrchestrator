use super::*;

pub(crate) fn initialize_database(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE (position)
);
",
    )
    .map_err(sql_error("initialize app database"))?;

    for (position, migration) in app_migrations().iter().enumerate() {
        let already_applied = conn
            .query_row(
                "SELECT 1 FROM schema_migrations WHERE id = ?1",
                params![migration.id],
                |_| Ok(()),
            )
            .optional()
            .map_err(sql_error("read schema migration state"))?
            .is_some();

        if already_applied {
            continue;
        }

        let tx = conn
            .unchecked_transaction()
            .map_err(sql_error("begin schema migration"))?;
        tx.execute_batch(migration.sql)
            .map_err(|error| format!("Unable to apply migration {}: {error}", migration.id))?;
        tx.execute(
            "INSERT INTO schema_migrations (id, applied_at, position) VALUES (?1, ?2, ?3)",
            params![migration.id, now_iso(), position as i64],
        )
        .map_err(sql_error("record schema migration"))?;
        tx.commit().map_err(sql_error("commit schema migration"))?;
    }

    Ok(())
}
