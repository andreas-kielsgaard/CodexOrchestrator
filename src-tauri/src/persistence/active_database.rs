use super::{ManagedOperationError, PersistenceError, PersistencePhase};
use rusqlite::{Connection, Transaction, TransactionBehavior};
use std::{
    cell::Cell,
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};

const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);

thread_local! {
    static MANAGED_WRITE_DEPTH: Cell<u8> = const { Cell::new(0) };
}

struct ManagedWriteGuard;

impl ManagedWriteGuard {
    fn enter() -> Self {
        MANAGED_WRITE_DEPTH.with(|depth| depth.set(depth.get() + 1));
        Self
    }
}

impl Drop for ManagedWriteGuard {
    fn drop(&mut self) {
        MANAGED_WRITE_DEPTH.with(|depth| depth.set(depth.get() - 1));
    }
}

enum ReaderSource {
    File(PathBuf),
    SharedWriter,
}

/// Application-owned access to one SQLite database.
///
/// Product code may run concurrently, but all writes in one process enter through the single
/// immediate-transaction boundary. File-backed readers use independent query-only connections.
pub(crate) struct ActiveDatabase {
    writer: Mutex<Connection>,
    readers: ReaderSource,
}

impl ActiveDatabase {
    pub(crate) fn open(
        path: impl AsRef<Path>,
        initialize: impl FnOnce(&Connection) -> Result<(), String>,
    ) -> Result<Self, PersistenceError> {
        let path = path.as_ref().to_path_buf();
        let connection = Connection::open(&path).map_err(|error| {
            PersistenceError::new(
                "open active database",
                PersistencePhase::Open,
                error.to_string(),
            )
        })?;
        configure_writer_connection(&connection).map_err(|error| {
            PersistenceError::new(
                "configure active database writer",
                PersistencePhase::Configure,
                error.to_string(),
            )
        })?;
        initialize(&connection).map_err(|error| {
            PersistenceError::new(
                "initialize active database",
                PersistencePhase::Configure,
                error,
            )
        })?;
        Ok(Self {
            writer: Mutex::new(connection),
            readers: ReaderSource::File(path),
        })
    }

    /// Supports repository unit tests that intentionally use one in-memory connection.
    pub(crate) fn from_connection(
        connection: Connection,
        initialize: impl FnOnce(&Connection) -> Result<(), String>,
    ) -> Result<Self, PersistenceError> {
        configure_writer_connection(&connection).map_err(|error| {
            PersistenceError::new(
                "configure active database writer",
                PersistencePhase::Configure,
                error.to_string(),
            )
        })?;
        initialize(&connection).map_err(|error| {
            PersistenceError::new(
                "initialize active database",
                PersistencePhase::Configure,
                error,
            )
        })?;
        Ok(Self {
            writer: Mutex::new(connection),
            readers: ReaderSource::SharedWriter,
        })
    }

    pub(crate) fn read<T, E>(
        &self,
        operation: &'static str,
        read: impl FnOnce(&Connection) -> Result<T, E>,
    ) -> Result<T, ManagedOperationError<E>> {
        reject_nested_operation(operation)?;
        match &self.readers {
            ReaderSource::File(path) => {
                let connection = Connection::open(path).map_err(|error| {
                    ManagedOperationError::Infrastructure(PersistenceError::new(
                        operation,
                        PersistencePhase::Open,
                        error.to_string(),
                    ))
                })?;
                configure_reader_connection(&connection).map_err(|error| {
                    ManagedOperationError::Infrastructure(PersistenceError::new(
                        operation,
                        PersistencePhase::Configure,
                        error.to_string(),
                    ))
                })?;
                read(&connection).map_err(ManagedOperationError::Domain)
            }
            ReaderSource::SharedWriter => {
                let connection = self.writer.lock().map_err(|_| {
                    ManagedOperationError::Infrastructure(PersistenceError::new(
                        operation,
                        PersistencePhase::Lock,
                        "active database writer lock is poisoned",
                    ))
                })?;
                read(&connection).map_err(ManagedOperationError::Domain)
            }
        }
    }

    pub(crate) fn write<T, E>(
        &self,
        operation: &'static str,
        write: impl FnOnce(&Transaction<'_>) -> Result<T, E>,
    ) -> Result<T, ManagedOperationError<E>> {
        reject_nested_operation(operation)?;
        let mut connection = self.writer.lock().map_err(|_| {
            ManagedOperationError::Infrastructure(PersistenceError::new(
                operation,
                PersistencePhase::Lock,
                "active database writer lock is poisoned",
            ))
        })?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| {
                ManagedOperationError::Infrastructure(PersistenceError::new(
                    operation,
                    PersistencePhase::Begin,
                    error.to_string(),
                ))
            })?;
        let _managed_write = ManagedWriteGuard::enter();
        let result = match write(&transaction) {
            Ok(result) => result,
            Err(error) => {
                transaction.rollback().map_err(|rollback_error| {
                    ManagedOperationError::Infrastructure(PersistenceError::new(
                        operation,
                        PersistencePhase::Rollback,
                        rollback_error.to_string(),
                    ))
                })?;
                return Err(ManagedOperationError::Domain(error));
            }
        };
        transaction.commit().map_err(|error| {
            ManagedOperationError::Infrastructure(PersistenceError::new(
                operation,
                PersistencePhase::Commit,
                error.to_string(),
            ))
        })?;
        Ok(result)
    }
}

fn reject_nested_operation<E>(operation: &'static str) -> Result<(), ManagedOperationError<E>> {
    MANAGED_WRITE_DEPTH.with(|depth| {
        if depth.get() == 0 {
            Ok(())
        } else {
            Err(ManagedOperationError::Infrastructure(
                PersistenceError::new(
                    operation,
                    PersistencePhase::NestedOperation,
                    "managed database operations cannot be nested inside a managed write",
                ),
            ))
        }
    })
}

pub(crate) fn configure_writer_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    connection.pragma_update(None, "synchronous", "FULL")?;
    Ok(())
}

pub(crate) fn configure_reader_connection(connection: &Connection) -> rusqlite::Result<()> {
    connection.busy_timeout(SQLITE_BUSY_TIMEOUT)?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "query_only", true)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};
    use tempfile::TempDir;

    fn open(path: &Path) -> ActiveDatabase {
        ActiveDatabase::open(path, |connection| {
            connection
                .execute_batch("CREATE TABLE counters(value INTEGER NOT NULL);")
                .map_err(|error| error.to_string())?;
            Ok(())
        })
        .unwrap()
    }

    #[test]
    fn managed_writes_serialize_read_then_write_operations() {
        let directory = TempDir::new().unwrap();
        let database = Arc::new(open(&directory.path().join("managed.sqlite")));
        database
            .write("seed counter", |transaction| {
                transaction.execute("INSERT INTO counters VALUES(0)", [])?;
                Ok::<_, rusqlite::Error>(())
            })
            .unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let workers = (0..2)
            .map(|_| {
                let database = database.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    database.write("increment counter", |transaction| {
                        let value: i64 =
                            transaction
                                .query_row("SELECT value FROM counters", [], |row| row.get(0))?;
                        transaction.execute("UPDATE counters SET value=?1", [value + 1])?;
                        Ok::<_, rusqlite::Error>(())
                    })
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        for worker in workers {
            worker.join().unwrap().unwrap();
        }
        let value = database
            .read("load counter", |connection| {
                connection.query_row("SELECT value FROM counters", [], |row| row.get::<_, i64>(0))
            })
            .unwrap();
        assert_eq!(value, 2);
    }

    #[test]
    fn independent_database_handles_wait_for_the_file_writer() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("independent.sqlite");
        let first = Arc::new(open(&path));
        let second =
            Arc::new(ActiveDatabase::open(&path, |_| Ok(())).expect("open second database handle"));
        first
            .write("seed counter", |transaction| {
                transaction.execute("INSERT INTO counters VALUES(0)", [])?;
                Ok::<_, rusqlite::Error>(())
            })
            .unwrap();
        let barrier = Arc::new(Barrier::new(3));
        let workers = [first, second]
            .into_iter()
            .map(|database| {
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    database.write("increment counter", |transaction| {
                        let value: i64 =
                            transaction
                                .query_row("SELECT value FROM counters", [], |row| row.get(0))?;
                        transaction.execute("UPDATE counters SET value=?1", [value + 1])?;
                        Ok::<_, rusqlite::Error>(())
                    })
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        for worker in workers {
            worker.join().unwrap().unwrap();
        }
        let connection = Connection::open(path).unwrap();
        let value: i64 = connection
            .query_row("SELECT value FROM counters", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, 2);
    }

    #[test]
    fn readers_are_query_only() {
        let directory = TempDir::new().unwrap();
        let database = open(&directory.path().join("reader.sqlite"));
        let result = database.read("attempt reader write", |connection| {
            connection.execute("INSERT INTO counters VALUES(1)", [])
        });
        assert!(matches!(result, Err(ManagedOperationError::Domain(_))));
    }

    #[test]
    fn domain_failure_rolls_back_the_managed_write() {
        let directory = TempDir::new().unwrap();
        let database = open(&directory.path().join("rollback.sqlite"));
        let result = database.write("rejected counter insert", |transaction| {
            transaction
                .execute("INSERT INTO counters VALUES(1)", [])
                .map_err(|error| error.to_string())?;
            Err::<(), _>("reject operation".to_string())
        });
        assert!(matches!(result, Err(ManagedOperationError::Domain(_))));
        let count = database
            .read("count counters", |connection| {
                connection.query_row("SELECT COUNT(*) FROM counters", [], |row| {
                    row.get::<_, i64>(0)
                })
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn nested_managed_operations_fail_without_deadlocking() {
        let directory = TempDir::new().unwrap();
        let database = open(&directory.path().join("nested.sqlite"));
        let result = database.write("outer write", |transaction| {
            transaction.execute("INSERT INTO counters VALUES(1)", [])?;
            let nested = database.read("nested read", |connection| {
                connection.query_row("SELECT COUNT(*) FROM counters", [], |row| {
                    row.get::<_, i64>(0)
                })
            });
            assert!(matches!(
                nested,
                Err(ManagedOperationError::Infrastructure(error))
                    if error.phase() == PersistencePhase::NestedOperation
            ));
            let nested_write =
                database.write("nested write", |_transaction| Ok::<_, rusqlite::Error>(()));
            assert!(matches!(
                nested_write,
                Err(ManagedOperationError::Infrastructure(error))
                    if error.phase() == PersistencePhase::NestedOperation
            ));
            Err::<(), _>(rusqlite::Error::InvalidQuery)
        });
        assert!(matches!(result, Err(ManagedOperationError::Domain(_))));

        let count = database
            .read("count after rejected nested operation", |connection| {
                connection.query_row("SELECT COUNT(*) FROM counters", [], |row| {
                    row.get::<_, i64>(0)
                })
            })
            .unwrap();
        assert_eq!(count, 0);
    }
}
