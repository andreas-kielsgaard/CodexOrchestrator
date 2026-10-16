use crate::persistence::ActiveDatabase;
use std::{path::Path, sync::Arc};
pub fn open(path: &Path) -> Result<Arc<ActiveDatabase>, String> {
    ActiveDatabase::open(path, crate::schema::initialize)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}
#[cfg(test)]
pub fn from_connection(connection: rusqlite::Connection) -> Result<Arc<ActiveDatabase>, String> {
    ActiveDatabase::from_connection(connection, crate::schema::initialize)
        .map(Arc::new)
        .map_err(|e| e.to_string())
}
