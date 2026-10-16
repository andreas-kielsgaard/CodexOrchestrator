use rusqlite::Connection;

pub fn initialize(db: &Connection) -> Result<(), String> {
    db.execute_batch(r#"
CREATE TABLE IF NOT EXISTS orchid_instance (singleton INTEGER PRIMARY KEY CHECK(singleton=1), device_id TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS orchid_session_authority(session_id TEXT PRIMARY KEY,owner TEXT NOT NULL,transfers_json TEXT NOT NULL,delivery_pending INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS orchid_invocation_devices(invocation_id TEXT PRIMARY KEY,device_id TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS orchid_sync_changes (sequence INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS orchid_sync_cursors (peer TEXT PRIMARY KEY, sent INTEGER NOT NULL DEFAULT 0, received INTEGER NOT NULL DEFAULT 0);
CREATE TABLE IF NOT EXISTS orchid_session_replicas (session_id TEXT PRIMARY KEY, device_id TEXT NOT NULL, source_sequence INTEGER NOT NULL, payload_json TEXT NOT NULL CHECK(json_valid(payload_json)));
CREATE TABLE IF NOT EXISTS orchid_session_replica_summaries(session_id TEXT PRIMARY KEY,device_id TEXT NOT NULL,source_sequence INTEGER NOT NULL,payload_json TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS orchid_replica_changes (sequence INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS orchid_retention (category TEXT PRIMARY KEY, mode TEXT NOT NULL CHECK(mode IN ('retained','cache')));
INSERT OR IGNORE INTO orchid_retention VALUES('sessions','retained'),('definitions','retained'),('skills','retained');
"#).map_err(|e|e.to_string())?;
    let device_id = uuid::Uuid::new_v4().to_string();
    db.execute(
        "INSERT OR IGNORE INTO orchid_instance VALUES(1,?1)",
        [device_id],
    )
    .map_err(|e| e.to_string())?;
    // Triggers record aggregate identities in the mutation transaction. Exporters own payloads.
    let mut tables = vec![
        ("agent_sessions", "{row}.id".to_string()),
        ("agent_session_invocations", "{row}.session_id".into()),
        ("agent_session_preparations", "{row}.session_id".into()),
        ("agent_session_current_execution", "{row}.session_id".into()),
        ("agent_session_queue_entries", "{row}.session_id".into()),
        ("agent_session_queue_state", "{row}.session_id".into()),
        ("agent_session_target_operations", "{row}.session_id".into()),
        ("agent_session_history_positions", "{row}.session_id".into()),
        ("agent_session_provider_bindings", "{row}.session_id".into()),
        (
            "agent_session_initial_prompt_prefixes",
            "{row}.session_id".into(),
        ),
    ];
    for table in [
        "agent_session_runtime_events",
        "agent_session_invocation_launch_acceptances",
        "agent_session_invocation_execution_snapshots",
        "agent_session_invocation_diagnostics",
    ] {
        tables.push((
            table,
            "(SELECT session_id FROM agent_session_invocations WHERE id={row}.invocation_id)"
                .into(),
        ));
    }
    for (table, identity) in tables {
        for (operation, row) in [("INSERT", "NEW"), ("UPDATE", "NEW"), ("DELETE", "OLD")] {
            let value = identity.replace("{row}", row);
            db.execute_batch(&format!("CREATE TRIGGER IF NOT EXISTS sync_{table}_{operation} AFTER {operation} ON {table} WHEN {value} IS NOT NULL BEGIN INSERT INTO orchid_sync_changes(session_id) VALUES({value}); END;"))
                .map_err(|e|e.to_string())?;
        }
    }
    // Seed existing conversations once when enabling synchronization on an established device.
    db.execute("INSERT INTO orchid_sync_changes(session_id) SELECT id FROM agent_sessions WHERE id NOT IN (SELECT session_id FROM orchid_sync_changes)",[]).map_err(|e|e.to_string())?;
    Ok(())
}
