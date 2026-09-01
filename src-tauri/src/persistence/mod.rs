mod active_database;
mod error;

pub(crate) use active_database::{configure_writer_connection, ActiveDatabase};
pub(crate) use error::{ManagedOperationError, PersistenceError, PersistencePhase};
