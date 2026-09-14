use rusqlite::Connection;

pub(crate) const AGENT_SESSION_SCHEMA: &str = r#"
CREATE TABLE agent_sessions (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  availability TEXT NOT NULL CHECK (availability IN ('available', 'archived')),
  external_context_id TEXT,
  runtime_version TEXT,
  working_directory TEXT,
  workspace_origin TEXT,
  requested_options_json TEXT NOT NULL CHECK (json_valid(requested_options_json)),
  session_profile_json TEXT CHECK (session_profile_json IS NULL OR json_valid(session_profile_json)),
  harness_version_ref_json TEXT CHECK (harness_version_ref_json IS NULL OR json_valid(harness_version_ref_json)),
  assigned_identity_json TEXT CHECK (assigned_identity_json IS NULL OR json_valid(assigned_identity_json)),
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

/// Adds nullable ownership columns to databases created before Agent Sessions owned these values.
/// The active schema already contains them; the guards keep repository open backward-compatible.
pub(crate) fn ensure_agent_session_ownership_schema(conn: &Connection) -> Result<(), String> {
    let columns = table_columns(conn, "agent_sessions")?;
    if columns.is_empty() {
        return Ok(());
    }
    if !columns.iter().any(|column| column == "workspace_origin") {
        conn.execute(
            "ALTER TABLE agent_sessions ADD COLUMN workspace_origin TEXT",
            [],
        )
        .map_err(|error| format!("Unable to add Session workspace origin: {error}"))?;
    }
    if !columns
        .iter()
        .any(|column| column == "session_profile_json")
    {
        conn.execute(
            "ALTER TABLE agent_sessions ADD COLUMN session_profile_json TEXT CHECK (session_profile_json IS NULL OR json_valid(session_profile_json))",
            [],
        )
        .map_err(|error| format!("Unable to add pinned Agent Session Profile: {error}"))?;
    }
    if !columns
        .iter()
        .any(|column| column == "harness_version_ref_json")
    {
        conn.execute(
            "ALTER TABLE agent_sessions ADD COLUMN harness_version_ref_json TEXT CHECK (harness_version_ref_json IS NULL OR json_valid(harness_version_ref_json))",
            [],
        )
        .map_err(|error| format!("Unable to add Agent Session Harness ownership: {error}"))?;
    }
    if !columns
        .iter()
        .any(|column| column == "assigned_identity_json")
    {
        conn.execute(
            "ALTER TABLE agent_sessions ADD COLUMN assigned_identity_json TEXT CHECK (assigned_identity_json IS NULL OR json_valid(assigned_identity_json))",
            [],
        )
        .map_err(|error| format!("Unable to add Agent Session identity ownership: {error}"))?;
    }
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
