mod active_database;
mod error;

pub use active_database::{configure_writer_connection, ActiveDatabase};
pub use error::{ManagedOperationError, PersistenceError, PersistencePhase};
