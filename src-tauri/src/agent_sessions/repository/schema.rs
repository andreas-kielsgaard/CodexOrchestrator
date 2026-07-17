use rusqlite::Connection;

pub(crate) const AGENT_SESSION_SCHEMA: &str = r#"
CREATE TABLE agent_sessions (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  availability TEXT NOT NULL CHECK (availability IN ('available', 'archived')),
  external_context_id TEXT,
  runtime_version TEXT,
  working_directory TEXT,
  requested_options_json TEXT NOT NULL CHECK (json_valid(requested_options_json)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE agent_session_invocations (
  id TEXT PRIMARY KEY,
  session_id TEXT NOT NULL,
  submitted_text TEXT NOT NULL,
  input_provenance TEXT NOT NULL CHECK (input_provenance IN ('user', 'application')),
  status TEXT NOT NULL CHECK (status IN ('pending', 'running', 'completed', 'failed', 'canceled', 'interrupted')),
  requested_options_json TEXT NOT NULL CHECK (json_valid(requested_options_json)),
  effective_options_json TEXT CHECK (effective_options_json IS NULL OR json_valid(effective_options_json)),
  started_at TEXT,
  completed_at TEXT,
  exit_code INTEGER,
  signal TEXT,
  runtime_error_json TEXT CHECK (runtime_error_json IS NULL OR json_valid(runtime_error_json)),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (session_id) REFERENCES agent_sessions(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX agent_session_one_active_invocation
ON agent_session_invocations(session_id)
WHERE status IN ('pending', 'running');

CREATE INDEX agent_session_invocations_history
ON agent_session_invocations(session_id, created_at, id);

CREATE TABLE agent_session_invocation_launch_acceptances (
  invocation_id TEXT PRIMARY KEY,
  accepted_at TEXT NOT NULL,
  FOREIGN KEY (invocation_id) REFERENCES agent_session_invocations(id) ON DELETE CASCADE
);

CREATE TABLE agent_session_runtime_events (
  id TEXT PRIMARY KEY,
  invocation_id TEXT NOT NULL,
  sequence INTEGER NOT NULL CHECK (sequence >= 0),
  source TEXT NOT NULL CHECK (source IN ('stdout', 'stderr', 'runtime')),
  raw_payload_json TEXT NOT NULL CHECK (json_valid(raw_payload_json)),
  normalized_json TEXT CHECK (normalized_json IS NULL OR json_valid(normalized_json)),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (invocation_id) REFERENCES agent_session_invocations(id) ON DELETE CASCADE,
  UNIQUE (invocation_id, sequence)
);

CREATE INDEX agent_session_runtime_events_history
ON agent_session_runtime_events(invocation_id, sequence);

CREATE TABLE agent_session_invocation_diagnostics (
  invocation_id TEXT NOT NULL,
  sequence INTEGER NOT NULL CHECK (sequence >= 0),
  diagnostic_json TEXT NOT NULL CHECK (json_valid(diagnostic_json)),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (invocation_id) REFERENCES agent_session_invocations(id) ON DELETE CASCADE,
  PRIMARY KEY (invocation_id, sequence)
);
"#;

pub(crate) const AGENT_SESSION_LAUNCH_ACCEPTANCE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS agent_session_invocation_launch_acceptances (
  invocation_id TEXT PRIMARY KEY,
  accepted_at TEXT NOT NULL,
  FOREIGN KEY (invocation_id) REFERENCES agent_session_invocations(id) ON DELETE CASCADE
);
"#;

const PROTOTYPE_SESSION_TABLE: &str = "agent_sessions";
const PROTOTYPE_LOG_TABLE: &str = "agent_session_cli_logs";
const QUARANTINED_SESSION_TABLE: &str = "archived_prototype_agent_sessions_008";
const QUARANTINED_LOG_TABLE: &str = "archived_prototype_agent_session_cli_logs_008";

pub(crate) fn quarantine_archived_prototype_tables(conn: &Connection) -> Result<(), String> {
    let session_columns = table_columns(conn, PROTOTYPE_SESSION_TABLE)?;
    if session_columns.is_empty() {
        if !table_columns(conn, PROTOTYPE_LOG_TABLE)?.is_empty() {
            return Err(
                "Archived prototype log table exists without its agent_sessions table".into(),
            );
        }
        return Ok(());
    }

    let expected = [
        "id",
        "codex_session_id",
        "status",
        "command",
        "args_json",
        "cwd",
        "started_at",
        "completed_at",
        "exit_code",
        "error",
        "created_at",
        "updated_at",
    ];
    if session_columns != expected {
        return Err(format!(
            "Existing agent_sessions table is not the recognized archived 008 prototype shape; found columns {}",
            session_columns.join(", ")
        ));
    }
    if !table_columns(conn, QUARANTINED_SESSION_TABLE)?.is_empty()
        || !table_columns(conn, QUARANTINED_LOG_TABLE)?.is_empty()
    {
        return Err("Archived prototype quarantine table already exists".into());
    }

    let log_columns = table_columns(conn, PROTOTYPE_LOG_TABLE)?;
    let expected_logs = [
        "id",
        "agent_session_id",
        "stream_id",
        "stdout",
        "stderr",
        "created_at",
    ];
    if !log_columns.is_empty() && log_columns != expected_logs {
        return Err(format!(
            "Existing agent_session_cli_logs table is not the recognized archived 008 prototype shape; found columns {}",
            log_columns.join(", ")
        ));
    }

    if !log_columns.is_empty() {
        conn.execute(
            "ALTER TABLE agent_session_cli_logs RENAME TO archived_prototype_agent_session_cli_logs_008",
            [],
        )
        .map_err(|error| format!("Unable to quarantine archived prototype logs: {error}"))?;
    }
    conn.execute(
        "ALTER TABLE agent_sessions RENAME TO archived_prototype_agent_sessions_008",
        [],
    )
    .map_err(|error| format!("Unable to quarantine archived prototype sessions: {error}"))?;
    Ok(())
}

fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|error| format!("Unable to inspect {table}: {error}"))?;
    let columns = statement
        .query_map([], |row| row.get(1))
        .map_err(|error| format!("Unable to inspect {table}: {error}"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Unable to inspect {table}: {error}"))?;
    Ok(columns)
}
