use super::*;

pub(crate) const APP_DATABASE_FILE_NAME: &str = "codex-orchestrator.sqlite";

pub(crate) fn with_app_database<T>(
    app: &AppHandle,
    operation: impl FnOnce(&Connection) -> Result<T, String>,
) -> Result<T, String> {
    let database_path = app_database_path(app)?;
    let conn = open_initialized_database(database_path)?;
    operation(&conn)
}

pub(crate) fn app_database_path(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Unable to resolve app data directory: {error}"))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|error| format!("Unable to create app data directory: {error}"))?;

    Ok(app_data_dir.join(APP_DATABASE_FILE_NAME))
}

pub(crate) fn open_initialized_database(database_path: PathBuf) -> Result<Connection, String> {
    let conn = Connection::open(database_path)
        .map_err(|error| format!("Unable to open app SQLite database: {error}"))?;
    initialize_database(&conn)?;
    Ok(conn)
}
