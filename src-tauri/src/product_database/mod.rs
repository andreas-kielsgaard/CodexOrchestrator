mod active_schema;

use crate::persistence::ActiveDatabase;
use std::{path::Path, sync::Arc};

/// Opens the one physical product database without merging the ownership of its capabilities:
/// application configuration, operational control, authored definitions, runtime transactions,
/// and the product-development meta-process.
pub(crate) fn open(path: &Path) -> Result<Arc<ActiveDatabase>, String> {
    ActiveDatabase::open(path, active_schema::initialize)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
pub(crate) fn from_connection(
    connection: rusqlite::Connection,
) -> Result<Arc<ActiveDatabase>, String> {
    ActiveDatabase::from_connection(connection, active_schema::initialize)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}
