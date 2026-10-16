pub const ACTIVE_SCHEMA_VERSION: i64 = 66;
pub fn initialize_active_database(c: &rusqlite::Connection) -> Result<(), String> {
    crate::schema::initialize(c)
}
pub fn open_active_database(p: &std::path::Path) -> Result<rusqlite::Connection, String> {
    let c = rusqlite::Connection::open(p).map_err(|e| e.to_string())?;
    initialize_active_database(&c)?;
    Ok(c)
}
