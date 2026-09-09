use rusqlite::Connection;

/// Product-level assembly only. Individual schemas remain owned by their capabilities.
pub(super) fn initialize(connection: &Connection) -> Result<(), String> {
    crate::storage::initialize_active_database(connection)?;
    crate::orchestration::bootstrap_transition::initialize_bootstrap_transition_schema(connection)?;
    crate::orchestration::sprint_runner_transition::storage::initialize(connection)
}
