use rusqlite::Connection;
pub fn initialize(connection: &Connection) -> Result<(), String> {
    let transaction = connection
        .unchecked_transaction()
        .map_err(|e| e.to_string())?;
    initialize_features(&transaction)?;
    transaction
        .execute_batch(crate::agent_sessions::repository::FORK_SCHEMA)
        .map_err(|e| e.to_string())?;
    crate::synchronization::initialize(&transaction)?;
    transaction
        .execute_batch(crate::synchronization::definitions::SCHEMA)
        .map_err(|e| e.to_string())?;
    transaction
        .execute_batch(crate::devices::SCHEMA)
        .map_err(|e| e.to_string())?;
    transaction.execute_batch("CREATE TABLE IF NOT EXISTS orchid_core_migrations(version INTEGER PRIMARY KEY); INSERT OR IGNORE INTO orchid_core_migrations VALUES(1);").map_err(|e|e.to_string())?;
    transaction.commit().map_err(|e| e.to_string())
}
fn initialize_features(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(crate::runtime::providers::codex::profiles::NATIVE_PROFILE_SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(crate::execution_devices::SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(crate::repository_catalog::REPOSITORY_CATALOG_SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(crate::harness_engine::repository::HARNESS_BINDING_SCHEMA)
        .map_err(|e| e.to_string())?;
    initialize_replacement_workflow_schema(connection)?;
    initialize_session_navigation_schema(connection)
}
fn initialize_session_navigation_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(crate::agent_sessions::repository::PREPARATION_SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(crate::agent_sessions::repository::NATIVE_CONVERSATION_SCHEMA)
        .map_err(|e| e.to_string())?;
    migrate_provider_session_bindings(connection)?;
    connection
        .execute_batch(crate::agent_sessions::repository::QUEUE_SCHEMA)
        .map_err(|e| e.to_string())?;
    crate::agent_sessions::repository::migrate_legacy_target_transitions(connection)?;
    crate::agent_sessions::repository::upgrade_target_switches(connection)?;
    connection
        .execute_batch(crate::agent_sessions::repository::HISTORY_SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(crate::agent_sessions::repository::SESSION_ORGANIZATION_SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(crate::session_navigation::order_repository::SCHEMA)
        .map_err(|e| e.to_string())
}

fn migrate_provider_session_bindings(connection: &Connection) -> Result<(), String> {
    let legacy_exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='agent_session_parked_conversations')",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("Unable to inspect legacy provider sessions: {error}"))?
        != 0;
    if legacy_exists {
        connection
            .execute(
                "INSERT OR IGNORE INTO agent_session_provider_bindings(session_id,execution_key,binding_json,updated_at)
                 SELECT session_id,json_extract(conversation_json,'$.location'),conversation_json,parked_at
                 FROM agent_session_parked_conversations",
                [],
            )
            .map_err(|error| format!("Unable to migrate provider session bindings: {error}"))?;
        connection
            .execute("DROP TABLE agent_session_parked_conversations", [])
            .map_err(|error| format!("Unable to retire legacy provider sessions: {error}"))?;
    }
    Ok(())
}

fn initialize_replacement_workflow_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(crate::execution_devices::SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(crate::runtime::providers::claude::setups::SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(crate::repository_catalog::device_locations::DEVICE_LOCATION_SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(crate::execution_targets::sisters::SCHEMA)
        .map_err(|e| e.to_string())?;

    connection
        .execute_batch(crate::agent_sessions::repository::IMPORT_SCHEMA)
        .map_err(|e| e.to_string())?;
    crate::agent_sessions::repository::ensure_agent_session_ownership_schema(connection)?;
    crate::agent_sessions::repository::initialize_session_address_storage(connection)?;
    connection
        .execute_batch(crate::agent_sessions::repository::file_history::SCHEMA)
        .map_err(|error| format!("Unable to initialize Agent Session file history: {error}"))?;
    crate::otp_host::installations::initialize_otp_installation_storage(connection)?;
    crate::execution_configuration::initialize_capability_profile_storage(connection)?;
    crate::harness_engine::catalog_repository::initialize_harness_catalog_storage(connection)?;
    crate::identities::repository::initialize_identity_storage(connection)?;
    crate::session_events::initialize_session_event_storage(connection)?;
    crate::workflows::authoring_repository::initialize_workflow_authoring_storage(connection)?;
    crate::workflows::instances::initialize_workflow_instance_storage(connection)
}
