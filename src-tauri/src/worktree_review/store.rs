use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoredReviewSession {
    pub(crate) instance_ref: String,
    pub(crate) name: String,
    pub(crate) source_ref: String,
    pub(crate) source_label: String,
    pub(crate) built: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StoredReviewLifecycleEvent {
    pub(crate) occurred_at_ms: u64,
    pub(crate) kind: String,
    pub(crate) summary: String,
}

pub(crate) trait WorktreeReviewStore: Send + Sync {
    fn sessions(&self) -> Result<Vec<StoredReviewSession>, String>;
    fn upsert_session(&self, session: &StoredReviewSession) -> Result<(), String>;
    fn delete_session(&self, instance_ref: &str) -> Result<(), String>;
    fn lifecycle_history(
        &self,
        instance_ref: &str,
    ) -> Result<Vec<StoredReviewLifecycleEvent>, String>;
    fn append_lifecycle_event(
        &self,
        instance_ref: &str,
        event: &StoredReviewLifecycleEvent,
    ) -> Result<(), String>;
    fn source_cache(&self, fingerprint: &str) -> Result<Option<String>, String>;
    fn replace_source_cache(&self, fingerprint: &str, payload: &str) -> Result<(), String>;
    fn cleanup_detached_builds(&self) -> Result<bool, String>;
    fn set_cleanup_detached_builds(&self, enabled: bool) -> Result<(), String>;
}

pub(crate) struct SqliteWorktreeReviewStore {
    connection: Mutex<Connection>,
}

impl SqliteWorktreeReviewStore {
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path)
            .map_err(|error| format!("open Worktree Review state: {error}"))?;
        Self::initialize(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    #[cfg(test)]
    fn in_memory() -> Result<Self, String> {
        let connection = Connection::open_in_memory()
            .map_err(|error| format!("open Worktree Review state: {error}"))?;
        Self::initialize(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn initialize(connection: &Connection) -> Result<(), String> {
        connection
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS review_sessions (
                    instance_ref TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    source_ref TEXT NOT NULL DEFAULT '',
                    source_label TEXT NOT NULL,
                    built INTEGER NOT NULL CHECK (built IN (0, 1))
                );
                CREATE TABLE IF NOT EXISTS review_history (
                    event_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    instance_ref TEXT NOT NULL,
                    occurred_at_ms INTEGER NOT NULL,
                    kind TEXT NOT NULL,
                    summary TEXT NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_review_history_instance
                    ON review_history(instance_ref, event_id);
                CREATE TABLE IF NOT EXISTS review_source_cache (
                    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                    fingerprint TEXT NOT NULL,
                    payload TEXT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS review_settings (
                    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                    cleanup_detached_builds INTEGER NOT NULL DEFAULT 0
                        CHECK (cleanup_detached_builds IN (0, 1))
                );",
            )
            .map_err(|error| format!("initialize Worktree Review state: {error}"))?;
        let has_source_ref = {
            let mut statement = connection
                .prepare("PRAGMA table_info(review_sessions)")
                .map_err(|error| format!("inspect Worktree Review state: {error}"))?;
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|error| format!("inspect Worktree Review columns: {error}"))?;
            columns
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("read Worktree Review columns: {error}"))?
                .iter()
                .any(|column| column == "source_ref")
        };
        if !has_source_ref {
            connection
                .execute(
                    "ALTER TABLE review_sessions ADD COLUMN source_ref TEXT NOT NULL DEFAULT ''",
                    [],
                )
                .map_err(|error| format!("migrate Worktree Review state: {error}"))?;
        }
        Ok(())
    }

    fn connection(
        &self,
        unavailable: &str,
    ) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.connection.lock().map_err(|_| unavailable.to_string())
    }
}

impl WorktreeReviewStore for SqliteWorktreeReviewStore {
    fn sessions(&self) -> Result<Vec<StoredReviewSession>, String> {
        let connection = self.connection("Worktree Review state is unavailable.")?;
        let mut statement = connection
            .prepare(
                "SELECT instance_ref, name, source_ref, source_label, built FROM review_sessions",
            )
            .map_err(|error| format!("read Worktree Review state: {error}"))?;
        let rows = statement
            .query_map([], |row| {
                Ok(StoredReviewSession {
                    instance_ref: row.get(0)?,
                    name: row.get(1)?,
                    source_ref: row.get(2)?,
                    source_label: row.get(3)?,
                    built: row.get::<_, i64>(4)? != 0,
                })
            })
            .map_err(|error| format!("read Worktree Review sessions: {error}"))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("decode Worktree Review session: {error}"))
    }

    fn upsert_session(&self, session: &StoredReviewSession) -> Result<(), String> {
        self.connection("Worktree Review state is unavailable.")?
            .execute(
                "INSERT INTO review_sessions (instance_ref, name, source_ref, source_label, built)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(instance_ref) DO UPDATE SET
                    name = excluded.name,
                    source_ref = excluded.source_ref,
                    source_label = excluded.source_label,
                    built = excluded.built",
                params![
                    session.instance_ref,
                    session.name,
                    session.source_ref,
                    session.source_label,
                    i64::from(session.built)
                ],
            )
            .map_err(|_| "Worktree Review state could not be saved.".to_string())?;
        Ok(())
    }

    fn delete_session(&self, instance_ref: &str) -> Result<(), String> {
        let mut connection = self.connection("Worktree Review state is unavailable.")?;
        let transaction = connection
            .transaction()
            .map_err(|_| "Worktree Review cleanup could not begin.".to_string())?;
        transaction
            .execute(
                "DELETE FROM review_history WHERE instance_ref = ?1",
                [instance_ref],
            )
            .and_then(|_| {
                transaction.execute(
                    "DELETE FROM review_sessions WHERE instance_ref = ?1",
                    [instance_ref],
                )
            })
            .map_err(|_| "Worktree Review cleanup could not be saved.".to_string())?;
        transaction
            .commit()
            .map_err(|_| "Worktree Review cleanup could not be saved.".to_string())?;
        Ok(())
    }

    fn lifecycle_history(
        &self,
        instance_ref: &str,
    ) -> Result<Vec<StoredReviewLifecycleEvent>, String> {
        let connection = self.connection("Worktree Review history is unavailable.")?;
        let mut statement = connection
            .prepare(
                "SELECT occurred_at_ms, kind, summary FROM review_history
                 WHERE instance_ref = ?1 ORDER BY event_id",
            )
            .map_err(|_| "Worktree Review history is unavailable.".to_string())?;
        let rows = statement
            .query_map([instance_ref], |row| {
                Ok(StoredReviewLifecycleEvent {
                    occurred_at_ms: row.get::<_, i64>(0)?.max(0) as u64,
                    kind: row.get(1)?,
                    summary: row.get(2)?,
                })
            })
            .map_err(|_| "Worktree Review history is unavailable.".to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Worktree Review history is unavailable.".to_string())
    }

    fn append_lifecycle_event(
        &self,
        instance_ref: &str,
        event: &StoredReviewLifecycleEvent,
    ) -> Result<(), String> {
        self.connection("Worktree Review history is unavailable.")?
            .execute(
                "INSERT INTO review_history (instance_ref, occurred_at_ms, kind, summary)
                 VALUES (?1, ?2, ?3, ?4)",
                params![
                    instance_ref,
                    event.occurred_at_ms as i64,
                    event.kind,
                    event.summary
                ],
            )
            .map_err(|_| "Worktree Review history could not be saved.".to_string())?;
        Ok(())
    }

    fn source_cache(&self, fingerprint: &str) -> Result<Option<String>, String> {
        self.connection("Worktree Review source cache is unavailable.")?
            .query_row(
                "SELECT payload FROM review_source_cache WHERE singleton = 1 AND fingerprint = ?1",
                [fingerprint],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| format!("read Worktree Review source cache: {error}"))
    }

    fn replace_source_cache(&self, fingerprint: &str, payload: &str) -> Result<(), String> {
        self.connection("Worktree Review source cache is unavailable.")?
            .execute(
                "INSERT INTO review_source_cache (singleton, fingerprint, payload)
                 VALUES (1, ?1, ?2)
                 ON CONFLICT(singleton) DO UPDATE SET
                    fingerprint = excluded.fingerprint,
                    payload = excluded.payload",
                params![fingerprint, payload],
            )
            .map_err(|_| "Worktree Review source cache could not be saved.".to_string())?;
        Ok(())
    }

    fn cleanup_detached_builds(&self) -> Result<bool, String> {
        self.connection("Worktree Review settings are unavailable.")?
            .query_row(
                "SELECT cleanup_detached_builds FROM review_settings WHERE singleton = 1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|value| value.unwrap_or_default() != 0)
            .map_err(|_| "Worktree Review settings are unavailable.".to_string())
    }

    fn set_cleanup_detached_builds(&self, enabled: bool) -> Result<(), String> {
        self.connection("Worktree Review settings are unavailable.")?
            .execute(
                "INSERT INTO review_settings (singleton, cleanup_detached_builds) VALUES (1, ?1)
                 ON CONFLICT(singleton) DO UPDATE SET
                    cleanup_detached_builds = excluded.cleanup_detached_builds",
                [i64::from(enabled)],
            )
            .map_err(|_| "Worktree Review settings could not be saved.".to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persists_sessions_settings_history_and_cache_behind_one_port() {
        let store = SqliteWorktreeReviewStore::in_memory().unwrap();
        let session = StoredReviewSession {
            instance_ref: "wt-one".into(),
            name: "Review one".into(),
            source_ref: "source-one".into(),
            source_label: "Source one".into(),
            built: true,
        };
        store.upsert_session(&session).unwrap();
        store.set_cleanup_detached_builds(true).unwrap();
        store
            .append_lifecycle_event(
                &session.instance_ref,
                &StoredReviewLifecycleEvent {
                    occurred_at_ms: 42,
                    kind: "Prepared".into(),
                    summary: "Prepared safely.".into(),
                },
            )
            .unwrap();
        store.replace_source_cache("fingerprint", "[]").unwrap();

        assert_eq!(store.sessions().unwrap(), vec![session.clone()]);
        assert!(store.cleanup_detached_builds().unwrap());
        assert_eq!(
            store
                .lifecycle_history(&session.instance_ref)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            store.source_cache("fingerprint").unwrap().as_deref(),
            Some("[]")
        );

        store.delete_session(&session.instance_ref).unwrap();
        assert!(store.sessions().unwrap().is_empty());
        assert!(store
            .lifecycle_history(&session.instance_ref)
            .unwrap()
            .is_empty());
    }
}
