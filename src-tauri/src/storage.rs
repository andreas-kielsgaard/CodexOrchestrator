use rusqlite::Connection;
use std::time::Duration;

const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

/// Applies the app-wide policy to every SQLite connection before it is used. WAL permits readers
/// while a writer commits, the bounded busy timeout handles brief contention deliberately, and FULL
/// synchronous mode favors durable commits over write throughput.
pub(crate) fn configure_sqlite_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_explicit_connection_policy() {
        let connection = Connection::open_in_memory().expect("memory database");

        configure_sqlite_connection(&connection).expect("configure connection");

        assert_eq!(pragma_i64(&connection, "foreign_keys"), 1);
        assert_eq!(pragma_i64(&connection, "busy_timeout"), 5_000);
        assert_eq!(pragma_i64(&connection, "synchronous"), 2);
    }

    #[test]
    fn selects_wal_for_file_backed_connections() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let connection = Connection::open(directory.path().join("app.sqlite")).expect("database");

        configure_sqlite_connection(&connection).expect("configure connection");

        let journal_mode = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
            .expect("journal mode");
        assert_eq!(journal_mode, "wal");
    }

    fn pragma_i64(connection: &Connection, name: &str) -> i64 {
        connection
            .query_row(&format!("PRAGMA {name}"), [], |row| row.get(0))
            .expect("pragma value")
    }
}
