use super::{
    domain::{
        validate_new_invocation, validate_next_event, validate_runtime_binding_update,
        validate_session, validate_session_update, AgentDiagnostic, AgentInvocation,
        AgentInvocationId, AgentInvocationStatus, AgentRuntimeBinding, AgentRuntimeEvent,
        AgentRuntimeEventId, AgentRuntimeEventSource, AgentRuntimeOptions, AgentSession,
        AgentSessionAvailability, AgentSessionId, ContractViolation, InvocationCompletion,
        NormalizedRuntimeEvent,
    },
    ports::{AgentSessionRepository, ListAgentSessionsQuery, RepositoryError, RepositoryErrorKind},
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension, Row, Transaction};
use std::{path::Path, sync::Mutex};

pub(crate) const AGENT_SESSION_SCHEMA: &str = r#"
CREATE TABLE agent_sessions (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  availability TEXT NOT NULL CHECK (availability IN ('available', 'archived')),
  runtime_kind TEXT NOT NULL CHECK (runtime_kind IN ('codex_cli')),
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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AgentSessionHistory {
    pub(crate) session: AgentSession,
    pub(crate) invocations: Vec<AgentInvocationHistory>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AgentInvocationHistory {
    pub(crate) invocation: AgentInvocation,
    pub(crate) events: Vec<AgentRuntimeEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AgentSessionSummary {
    pub(crate) session: AgentSession,
    pub(crate) invocation_count: u64,
    pub(crate) latest_invocation_status: Option<AgentInvocationStatus>,
    pub(crate) latest_submitted_text: Option<String>,
}

pub(crate) struct SqliteAgentSessionRepository {
    connection: Mutex<Connection>,
}

impl SqliteAgentSessionRepository {
    pub(crate) fn new(connection: Connection) -> Self {
        Self {
            connection: Mutex::new(connection),
        }
    }

    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, RepositoryError> {
        let connection =
            Connection::open(path).map_err(sql_unavailable("open Agent Session database"))?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(sql_unavailable("enable Agent Session foreign keys"))?;
        Ok(Self::new(connection))
    }

    pub(crate) fn load_session_history(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSessionHistory>, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin Agent Session history load"))?;
        let session = match get_session_from(&transaction, session_id)? {
            Some(session) => session,
            None => return Ok(None),
        };
        let invocations = list_invocations_from(&transaction, session_id)?
            .into_iter()
            .map(|invocation| {
                let events = list_events_from(&transaction, &invocation.id)?;
                Ok(AgentInvocationHistory { invocation, events })
            })
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit Agent Session history load"))?;
        Ok(Some(AgentSessionHistory {
            session,
            invocations,
        }))
    }

    pub(crate) fn list_session_summaries(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSessionSummary>, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin Agent Session summary list"))?;
        let sessions = list_sessions_from(&transaction, query)?;
        let summaries = sessions
            .into_iter()
            .map(|session| {
                let invocation_count = transaction
                    .query_row(
                        "SELECT COUNT(*) FROM agent_session_invocations WHERE session_id = ?1",
                        params![session.id.as_str()],
                        |row| row.get::<_, u64>(0),
                    )
                    .map_err(sql_unavailable("count Agent Session invocations"))?;
                let latest = transaction
                    .query_row(
                        "SELECT status, submitted_text FROM agent_session_invocations WHERE session_id = ?1 ORDER BY created_at DESC, id DESC LIMIT 1",
                        params![session.id.as_str()],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                    )
                    .optional()
                    .map_err(sql_unavailable("load latest Agent Session invocation"))?;
                let (latest_invocation_status, latest_submitted_text) = match latest {
                    Some((status, text)) => (Some(parse_status(&status)?), Some(text)),
                    None => (None, None),
                };
                Ok(AgentSessionSummary {
                    session,
                    invocation_count,
                    latest_invocation_status,
                    latest_submitted_text,
                })
            })
            .collect::<Result<Vec<_>, RepositoryError>>()?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit Agent Session summary list"))?;
        Ok(summaries)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, RepositoryError> {
        self.connection.lock().map_err(|_| {
            RepositoryError::new(
                RepositoryErrorKind::Unavailable,
                "Agent Session database lock is poisoned",
            )
        })
    }
}

impl AgentSessionRepository for SqliteAgentSessionRepository {
    fn create_session(&self, session: AgentSession) -> Result<AgentSession, RepositoryError> {
        validate_session(&session).map_err(contract_error)?;
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin session create"))?;
        insert_session(&transaction, &session)?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit session create"))?;
        Ok(session)
    }

    fn get_session(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSession>, RepositoryError> {
        let connection = self.lock()?;
        get_session_from(&connection, session_id)
    }

    fn list_sessions(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSession>, RepositoryError> {
        let connection = self.lock()?;
        list_sessions_from(&connection, query)
    }

    fn set_session_availability(
        &self,
        session_id: &AgentSessionId,
        availability: AgentSessionAvailability,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin session availability update"))?;
        let current = required_session(&transaction, session_id)?;
        let mut candidate = current.clone();
        candidate.availability = availability;
        candidate.updated_at = updated_at;
        validate_session_update(&current, &candidate).map_err(contract_error)?;
        transaction
            .execute(
                "UPDATE agent_sessions SET availability = ?1, updated_at = ?2 WHERE id = ?3",
                params![
                    availability_text(availability),
                    timestamp(updated_at),
                    session_id.as_str()
                ],
            )
            .map_err(sql_unavailable("update session availability"))?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit session availability update"))?;
        Ok(candidate)
    }

    fn update_runtime_binding(
        &self,
        session_id: &AgentSessionId,
        binding: AgentRuntimeBinding,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin runtime binding update"))?;
        let current = required_session(&transaction, session_id)?;
        validate_runtime_binding_update(&current.runtime_binding, &binding)
            .map_err(contract_error)?;
        let mut candidate = current.clone();
        candidate.runtime_binding = binding;
        candidate.updated_at = updated_at;
        validate_session_update(&current, &candidate).map_err(contract_error)?;
        transaction
            .execute(
                "UPDATE agent_sessions SET runtime_kind = ?1, external_context_id = ?2, runtime_version = ?3, updated_at = ?4 WHERE id = ?5",
                params![
                    runtime_kind_text(candidate.runtime_binding.kind),
                    candidate.runtime_binding.external_context_id.as_ref().map(|id| id.as_str()),
                    candidate.runtime_binding.runtime_version,
                    timestamp(updated_at),
                    session_id.as_str()
                ],
            )
            .map_err(sql_unavailable("update runtime binding"))?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit runtime binding update"))?;
        Ok(candidate)
    }

    fn create_pending_invocation(
        &self,
        invocation: AgentInvocation,
    ) -> Result<AgentInvocation, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin invocation create"))?;
        let session = required_session(&transaction, &invocation.session_id)?;
        let active = active_invocation(&transaction, &invocation.session_id)?;
        validate_new_invocation(&session, active.as_ref(), &invocation).map_err(contract_error)?;
        insert_invocation(&transaction, &invocation)?;
        transaction
            .execute(
                "UPDATE agent_sessions SET updated_at = MAX(updated_at, ?1) WHERE id = ?2",
                params![
                    timestamp(invocation.updated_at),
                    invocation.session_id.as_str()
                ],
            )
            .map_err(sql_unavailable("touch Agent Session"))?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit invocation create"))?;
        Ok(invocation)
    }

    fn get_invocation(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<AgentInvocation>, RepositoryError> {
        let connection = self.lock()?;
        get_invocation_from(&connection, invocation_id)
    }

    fn list_invocations(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Vec<AgentInvocation>, RepositoryError> {
        let connection = self.lock()?;
        list_invocations_from(&connection, session_id)
    }

    fn mark_invocation_running(
        &self,
        invocation_id: &AgentInvocationId,
        started_at: DateTime<Utc>,
        effective_options: AgentRuntimeOptions,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin invocation start"))?;
        let current = required_invocation(&transaction, invocation_id)?;
        let updated = current
            .mark_running(started_at, effective_options, updated_at)
            .map_err(contract_error)?;
        update_invocation(&transaction, &updated)?;
        touch_session(&transaction, &updated.session_id, updated_at)?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit invocation start"))?;
        Ok(updated)
    }

    fn finish_invocation(
        &self,
        invocation_id: &AgentInvocationId,
        completion: InvocationCompletion,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin invocation finish"))?;
        let current = required_invocation(&transaction, invocation_id)?;
        let requested_status = AgentInvocationStatus::from(completion.status);
        if current.status == requested_status
            && current.completed_at == Some(completion.completed_at)
            && current.exit_code == completion.exit_code
            && current.signal == completion.signal
            && current.runtime_error == completion.runtime_error
        {
            transaction
                .commit()
                .map_err(sql_unavailable("commit idempotent invocation finish"))?;
            return Ok(current);
        }
        let updated = current
            .finish(completion, updated_at)
            .map_err(contract_error)?;
        update_invocation(&transaction, &updated)?;
        touch_session(&transaction, &updated.session_id, updated_at)?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit invocation finish"))?;
        Ok(updated)
    }

    fn append_invocation_diagnostic(
        &self,
        invocation_id: &AgentInvocationId,
        diagnostic: AgentDiagnostic,
    ) -> Result<AgentInvocation, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin diagnostic append"))?;
        let mut invocation = required_invocation(&transaction, invocation_id)?;
        let sequence = transaction
            .query_row(
                "SELECT COALESCE(MAX(sequence) + 1, 0) FROM agent_session_invocation_diagnostics WHERE invocation_id = ?1",
                params![invocation_id.as_str()],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sql_unavailable("read next diagnostic sequence"))?;
        transaction
            .execute(
                "INSERT INTO agent_session_invocation_diagnostics (invocation_id, sequence, diagnostic_json, recorded_at) VALUES (?1, ?2, ?3, ?4)",
                params![invocation_id.as_str(), sequence, to_json(&diagnostic)?, timestamp(diagnostic.recorded_at)],
            )
            .map_err(sql_write("append invocation diagnostic"))?;
        invocation.diagnostics.push(diagnostic);
        transaction
            .commit()
            .map_err(sql_unavailable("commit diagnostic append"))?;
        Ok(invocation)
    }

    fn append_event(&self, event: AgentRuntimeEvent) -> Result<AgentRuntimeEvent, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin runtime event append"))?;
        if get_invocation_from(&transaction, &event.invocation_id)?.is_none() {
            return Err(not_found("invocation not found"));
        }
        let previous = last_event(&transaction, &event.invocation_id)?;
        validate_next_event(&event.invocation_id, previous.as_ref(), &event)
            .map_err(contract_error)?;
        let sequence = i64::try_from(event.sequence).map_err(|_| {
            RepositoryError::new(
                RepositoryErrorKind::InvalidState,
                "event sequence exceeds SQLite integer range",
            )
        })?;
        transaction
            .execute(
                "INSERT INTO agent_session_runtime_events (id, invocation_id, sequence, source, raw_payload_json, normalized_json, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    event.id.as_str(), event.invocation_id.as_str(), sequence, event_source_text(event.source),
                    event.raw_payload.to_string(), event.normalized.as_ref().map(to_json).transpose()?, timestamp(event.recorded_at)
                ],
            )
            .map_err(sql_write("append runtime event"))?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit runtime event append"))?;
        Ok(event)
    }

    fn list_events(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Vec<AgentRuntimeEvent>, RepositoryError> {
        let connection = self.lock()?;
        list_events_from(&connection, invocation_id)
    }
}

type SessionRow = (
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    String,
    String,
    String,
);
type InvocationRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<String>,
    Option<String>,
    String,
    String,
);
type EventRow = (String, String, i64, String, String, Option<String>, String);

fn session_row(row: &Row<'_>) -> rusqlite::Result<SessionRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
    ))
}

fn invocation_row(row: &Row<'_>) -> rusqlite::Result<InvocationRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
        row.get(12)?,
    ))
}

fn event_row(row: &Row<'_>) -> rusqlite::Result<EventRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    ))
}

fn map_session_row(row: SessionRow) -> Result<AgentSession, RepositoryError> {
    Ok(AgentSession {
        id: AgentSessionId::new(row.0).map_err(contract_error)?,
        title: row.1,
        availability: parse_availability(&row.2)?,
        runtime_binding: AgentRuntimeBinding {
            kind: parse_runtime_kind(&row.3)?,
            external_context_id: row
                .4
                .map(super::domain::ExternalRuntimeContextId::new)
                .transpose()
                .map_err(contract_error)?,
            runtime_version: row.5,
        },
        working_directory: row.6,
        requested_options: from_json(&row.7, "session requested options")?,
        created_at: parse_timestamp(&row.8)?,
        updated_at: parse_timestamp(&row.9)?,
    })
}

fn map_invocation_row(row: InvocationRow) -> Result<AgentInvocation, RepositoryError> {
    Ok(AgentInvocation {
        id: AgentInvocationId::new(row.0).map_err(contract_error)?,
        session_id: AgentSessionId::new(row.1).map_err(contract_error)?,
        submitted_text: row.2,
        status: parse_status(&row.3)?,
        requested_options: from_json(&row.4, "invocation requested options")?,
        effective_options: row
            .5
            .as_deref()
            .map(|value| from_json(value, "invocation effective options"))
            .transpose()?,
        started_at: row.6.as_deref().map(parse_timestamp).transpose()?,
        completed_at: row.7.as_deref().map(parse_timestamp).transpose()?,
        exit_code: row.8,
        signal: row.9,
        runtime_error: row
            .10
            .as_deref()
            .map(|value| from_json(value, "runtime error"))
            .transpose()?,
        diagnostics: Vec::new(),
        created_at: parse_timestamp(&row.11)?,
        updated_at: parse_timestamp(&row.12)?,
    })
}

fn map_event_row(row: EventRow) -> Result<AgentRuntimeEvent, RepositoryError> {
    Ok(AgentRuntimeEvent {
        id: AgentRuntimeEventId::new(row.0).map_err(contract_error)?,
        invocation_id: AgentInvocationId::new(row.1).map_err(contract_error)?,
        sequence: u64::try_from(row.2).map_err(|_| invalid_data("negative event sequence"))?,
        source: parse_event_source(&row.3)?,
        raw_payload: from_json(&row.4, "raw runtime event payload")?,
        normalized: row
            .5
            .as_deref()
            .map(|value| from_json::<NormalizedRuntimeEvent>(value, "normalized runtime event"))
            .transpose()?,
        recorded_at: parse_timestamp(&row.6)?,
    })
}

fn collect_mapped<T, U>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&Row<'_>) -> rusqlite::Result<T>>,
    map: impl Fn(T) -> Result<U, RepositoryError>,
) -> Result<Vec<U>, RepositoryError> {
    rows.map(|row| {
        row.map_err(sql_unavailable("read Agent Session row"))
            .and_then(&map)
    })
    .collect()
}

fn insert_session(
    transaction: &Transaction<'_>,
    session: &AgentSession,
) -> Result<(), RepositoryError> {
    transaction.execute(
        "INSERT INTO agent_sessions (id, title, availability, runtime_kind, external_context_id, runtime_version, working_directory, requested_options_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![session.id.as_str(), session.title, availability_text(session.availability), runtime_kind_text(session.runtime_binding.kind), session.runtime_binding.external_context_id.as_ref().map(|id| id.as_str()), session.runtime_binding.runtime_version, session.working_directory, to_json(&session.requested_options)?, timestamp(session.created_at), timestamp(session.updated_at)],
    ).map_err(sql_write("create Agent Session"))?;
    Ok(())
}

fn insert_invocation(
    transaction: &Transaction<'_>,
    invocation: &AgentInvocation,
) -> Result<(), RepositoryError> {
    transaction.execute(
        "INSERT INTO agent_session_invocations (id, session_id, submitted_text, status, requested_options_json, effective_options_json, started_at, completed_at, exit_code, signal, runtime_error_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![invocation.id.as_str(), invocation.session_id.as_str(), invocation.submitted_text, status_text(invocation.status), to_json(&invocation.requested_options)?, invocation.effective_options.as_ref().map(to_json).transpose()?, invocation.started_at.map(timestamp), invocation.completed_at.map(timestamp), invocation.exit_code, invocation.signal, invocation.runtime_error.as_ref().map(to_json).transpose()?, timestamp(invocation.created_at), timestamp(invocation.updated_at)],
    ).map_err(sql_write("create pending invocation"))?;
    Ok(())
}

fn update_invocation(
    transaction: &Transaction<'_>,
    invocation: &AgentInvocation,
) -> Result<(), RepositoryError> {
    transaction.execute(
        "UPDATE agent_session_invocations SET status = ?1, effective_options_json = ?2, started_at = ?3, completed_at = ?4, exit_code = ?5, signal = ?6, runtime_error_json = ?7, updated_at = ?8 WHERE id = ?9",
        params![status_text(invocation.status), invocation.effective_options.as_ref().map(to_json).transpose()?, invocation.started_at.map(timestamp), invocation.completed_at.map(timestamp), invocation.exit_code, invocation.signal, invocation.runtime_error.as_ref().map(to_json).transpose()?, timestamp(invocation.updated_at), invocation.id.as_str()],
    ).map_err(sql_write("update invocation"))?;
    Ok(())
}

const SESSION_SELECT: &str = "SELECT id, title, availability, runtime_kind, external_context_id, runtime_version, working_directory, requested_options_json, created_at, updated_at FROM agent_sessions";
const INVOCATION_SELECT: &str = "SELECT id, session_id, submitted_text, status, requested_options_json, effective_options_json, started_at, completed_at, exit_code, signal, runtime_error_json, created_at, updated_at FROM agent_session_invocations";

fn list_sessions_from(
    conn: &Connection,
    query: ListAgentSessionsQuery,
) -> Result<Vec<AgentSession>, RepositoryError> {
    let availability = query.availability.map(availability_text);
    let limit = i64::from(query.limit.unwrap_or(u32::MAX));
    let mut statement = conn
        .prepare(
            "SELECT id, title, availability, runtime_kind, external_context_id, runtime_version, working_directory, requested_options_json, created_at, updated_at FROM agent_sessions WHERE (?1 IS NULL OR availability = ?1) ORDER BY updated_at DESC, id ASC LIMIT ?2",
        )
        .map_err(sql_unavailable("prepare Agent Session list"))?;
    let rows = statement
        .query_map(params![availability, limit], session_row)
        .map_err(sql_unavailable("query Agent Session list"))?;
    collect_mapped(rows, map_session_row)
}

fn list_invocations_from(
    conn: &Connection,
    session_id: &AgentSessionId,
) -> Result<Vec<AgentInvocation>, RepositoryError> {
    let mut statement = conn
        .prepare(&format!(
            "{} WHERE session_id = ?1 ORDER BY created_at ASC, id ASC",
            INVOCATION_SELECT
        ))
        .map_err(sql_unavailable("prepare invocation history"))?;
    let rows = statement
        .query_map(params![session_id.as_str()], invocation_row)
        .map_err(sql_unavailable("query invocation history"))?;
    let mut invocations = collect_mapped(rows, map_invocation_row)?;
    for invocation in &mut invocations {
        invocation.diagnostics = load_diagnostics(conn, &invocation.id)?;
    }
    Ok(invocations)
}

fn list_events_from(
    conn: &Connection,
    invocation_id: &AgentInvocationId,
) -> Result<Vec<AgentRuntimeEvent>, RepositoryError> {
    let mut statement = conn
        .prepare("SELECT id, invocation_id, sequence, source, raw_payload_json, normalized_json, recorded_at FROM agent_session_runtime_events WHERE invocation_id = ?1 ORDER BY sequence ASC")
        .map_err(sql_unavailable("prepare runtime event history"))?;
    let rows = statement
        .query_map(params![invocation_id.as_str()], event_row)
        .map_err(sql_unavailable("query runtime event history"))?;
    collect_mapped(rows, map_event_row)
}

fn get_session_from(
    conn: &Connection,
    session_id: &AgentSessionId,
) -> Result<Option<AgentSession>, RepositoryError> {
    let row = conn
        .query_row(
            &format!("{SESSION_SELECT} WHERE id = ?1"),
            params![session_id.as_str()],
            session_row,
        )
        .optional()
        .map_err(sql_unavailable("load Agent Session"))?;
    row.map(map_session_row).transpose()
}

fn required_session(
    conn: &Connection,
    session_id: &AgentSessionId,
) -> Result<AgentSession, RepositoryError> {
    get_session_from(conn, session_id)?.ok_or_else(|| not_found("session not found"))
}

fn get_invocation_from(
    conn: &Connection,
    invocation_id: &AgentInvocationId,
) -> Result<Option<AgentInvocation>, RepositoryError> {
    let row = conn
        .query_row(
            &format!("{INVOCATION_SELECT} WHERE id = ?1"),
            params![invocation_id.as_str()],
            invocation_row,
        )
        .optional()
        .map_err(sql_unavailable("load invocation"))?;
    let mut invocation = row.map(map_invocation_row).transpose()?;
    if let Some(invocation) = &mut invocation {
        invocation.diagnostics = load_diagnostics(conn, &invocation.id)?;
    }
    Ok(invocation)
}

fn required_invocation(
    conn: &Connection,
    invocation_id: &AgentInvocationId,
) -> Result<AgentInvocation, RepositoryError> {
    get_invocation_from(conn, invocation_id)?.ok_or_else(|| not_found("invocation not found"))
}

fn active_invocation(
    conn: &Connection,
    session_id: &AgentSessionId,
) -> Result<Option<AgentInvocation>, RepositoryError> {
    let row = conn.query_row(&format!("{INVOCATION_SELECT} WHERE session_id = ?1 AND status IN ('pending', 'running') LIMIT 1"), params![session_id.as_str()], invocation_row).optional().map_err(sql_unavailable("load active invocation"))?;
    row.map(map_invocation_row).transpose()
}

fn load_diagnostics(
    conn: &Connection,
    invocation_id: &AgentInvocationId,
) -> Result<Vec<AgentDiagnostic>, RepositoryError> {
    let mut statement = conn.prepare("SELECT diagnostic_json FROM agent_session_invocation_diagnostics WHERE invocation_id = ?1 ORDER BY sequence ASC").map_err(sql_unavailable("prepare invocation diagnostics"))?;
    let rows = statement
        .query_map(params![invocation_id.as_str()], |row| {
            row.get::<_, String>(0)
        })
        .map_err(sql_unavailable("query invocation diagnostics"))?;
    rows.map(|row| {
        let json = row.map_err(sql_unavailable("read invocation diagnostic"))?;
        from_json(&json, "invocation diagnostic")
    })
    .collect()
}

fn last_event(
    conn: &Connection,
    invocation_id: &AgentInvocationId,
) -> Result<Option<AgentRuntimeEvent>, RepositoryError> {
    let row = conn.query_row("SELECT id, invocation_id, sequence, source, raw_payload_json, normalized_json, recorded_at FROM agent_session_runtime_events WHERE invocation_id = ?1 ORDER BY sequence DESC LIMIT 1", params![invocation_id.as_str()], event_row).optional().map_err(sql_unavailable("load last runtime event"))?;
    row.map(map_event_row).transpose()
}

fn touch_session(
    conn: &Connection,
    session_id: &AgentSessionId,
    updated_at: DateTime<Utc>,
) -> Result<(), RepositoryError> {
    conn.execute(
        "UPDATE agent_sessions SET updated_at = MAX(updated_at, ?1) WHERE id = ?2",
        params![timestamp(updated_at), session_id.as_str()],
    )
    .map_err(sql_unavailable("touch Agent Session"))?;
    Ok(())
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, RepositoryError> {
    serde_json::to_string(value)
        .map_err(|error| invalid_data(format!("unable to serialize Agent Session data: {error}")))
}

fn from_json<T: serde::de::DeserializeOwned>(
    value: &str,
    context: &str,
) -> Result<T, RepositoryError> {
    serde_json::from_str(value).map_err(|error| invalid_data(format!("invalid {context}: {error}")))
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}
fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, RepositoryError> {
    value
        .parse()
        .map_err(|error| invalid_data(format!("invalid timestamp {value}: {error}")))
}

fn availability_text(value: AgentSessionAvailability) -> &'static str {
    match value {
        AgentSessionAvailability::Available => "available",
        AgentSessionAvailability::Archived => "archived",
    }
}
fn parse_availability(value: &str) -> Result<AgentSessionAvailability, RepositoryError> {
    match value {
        "available" => Ok(AgentSessionAvailability::Available),
        "archived" => Ok(AgentSessionAvailability::Archived),
        _ => Err(invalid_data(format!(
            "unknown session availability {value}"
        ))),
    }
}
fn runtime_kind_text(_: super::domain::AgentRuntimeKind) -> &'static str {
    "codex_cli"
}
fn parse_runtime_kind(value: &str) -> Result<super::domain::AgentRuntimeKind, RepositoryError> {
    match value {
        "codex_cli" => Ok(super::domain::AgentRuntimeKind::CodexCli),
        _ => Err(invalid_data(format!("unknown runtime kind {value}"))),
    }
}
fn status_text(value: AgentInvocationStatus) -> &'static str {
    match value {
        AgentInvocationStatus::Pending => "pending",
        AgentInvocationStatus::Running => "running",
        AgentInvocationStatus::Completed => "completed",
        AgentInvocationStatus::Failed => "failed",
        AgentInvocationStatus::Canceled => "canceled",
        AgentInvocationStatus::Interrupted => "interrupted",
    }
}
fn parse_status(value: &str) -> Result<AgentInvocationStatus, RepositoryError> {
    match value {
        "pending" => Ok(AgentInvocationStatus::Pending),
        "running" => Ok(AgentInvocationStatus::Running),
        "completed" => Ok(AgentInvocationStatus::Completed),
        "failed" => Ok(AgentInvocationStatus::Failed),
        "canceled" => Ok(AgentInvocationStatus::Canceled),
        "interrupted" => Ok(AgentInvocationStatus::Interrupted),
        _ => Err(invalid_data(format!("unknown invocation status {value}"))),
    }
}
fn event_source_text(value: AgentRuntimeEventSource) -> &'static str {
    match value {
        AgentRuntimeEventSource::Stdout => "stdout",
        AgentRuntimeEventSource::Stderr => "stderr",
        AgentRuntimeEventSource::Runtime => "runtime",
    }
}
fn parse_event_source(value: &str) -> Result<AgentRuntimeEventSource, RepositoryError> {
    match value {
        "stdout" => Ok(AgentRuntimeEventSource::Stdout),
        "stderr" => Ok(AgentRuntimeEventSource::Stderr),
        "runtime" => Ok(AgentRuntimeEventSource::Runtime),
        _ => Err(invalid_data(format!(
            "unknown runtime event source {value}"
        ))),
    }
}

fn contract_error(error: ContractViolation) -> RepositoryError {
    let kind = match error {
        ContractViolation::ActiveInvocationExists { .. }
        | ContractViolation::EventSequenceNotIncreasing { .. } => RepositoryErrorKind::Conflict,
        _ => RepositoryErrorKind::InvalidState,
    };
    RepositoryError::new(kind, error.to_string())
}
fn not_found(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::NotFound, message)
}
fn invalid_data(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Unavailable, message)
}
fn sql_unavailable(context: &'static str) -> impl FnOnce(rusqlite::Error) -> RepositoryError {
    move |error| {
        RepositoryError::new(
            RepositoryErrorKind::Unavailable,
            format!("Unable to {context}: {error}"),
        )
    }
}
fn sql_write(context: &'static str) -> impl FnOnce(rusqlite::Error) -> RepositoryError {
    move |error| {
        let kind = match &error {
            rusqlite::Error::SqliteFailure(failure, _)
                if matches!(failure.code, ErrorCode::ConstraintViolation) =>
            {
                RepositoryErrorKind::Conflict
            }
            _ => RepositoryErrorKind::Unavailable,
        };
        RepositoryError::new(kind, format!("Unable to {context}: {error}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_sessions::domain::{
        AgentDiagnosticSeverity, AgentDiagnosticSource, AgentInvocationTerminalStatus,
        AgentRuntimeFailure, AgentRuntimeKind, ExternalRuntimeContextId,
        NormalizedRuntimeEventKind,
    };
    use serde_json::json;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    #[test]
    fn survives_close_and_reopen_with_complete_multi_invocation_history() {
        let path = temporary_database_path();
        let connection = initialized_file_database(&path);
        let repository = SqliteAgentSessionRepository::new(connection);
        let session = repository
            .create_session(test_session("session-b", at(0)))
            .expect("create session");
        repository
            .update_runtime_binding(
                &session.id,
                AgentRuntimeBinding {
                    kind: AgentRuntimeKind::CodexCli,
                    external_context_id: Some(
                        ExternalRuntimeContextId::new("provider-thread").expect("external ID"),
                    ),
                    runtime_version: Some("codex-test".into()),
                },
                at(1),
            )
            .expect("bind runtime context");

        let first = repository
            .create_pending_invocation(test_invocation("invocation-b", &session.id, at(2)))
            .expect("create first invocation");
        repository
            .mark_invocation_running(&first.id, at(3), options(), at(3))
            .expect("start first invocation");
        repository
            .append_event(unknown_event(
                "event-b2",
                &first.id,
                2,
                json!({"type":"future.event","nested":{"unchanged":[1,true,null]}}),
                at(4),
            ))
            .expect("append first unknown event");
        repository
            .append_event(normalized_event("event-b9", &first.id, 9, at(5)))
            .expect("append first normalized event");
        let first_completion = InvocationCompletion {
            status: AgentInvocationTerminalStatus::Completed,
            completed_at: at(6),
            exit_code: Some(0),
            signal: None,
            runtime_error: None,
        };
        repository
            .finish_invocation(&first.id, first_completion.clone(), at(6))
            .expect("finish first invocation");
        repository
            .finish_invocation(&first.id, first_completion, at(30))
            .expect("repeat identical completion idempotently");

        let second = repository
            .create_pending_invocation(test_invocation("invocation-a", &session.id, at(7)))
            .expect("create second invocation");
        repository
            .append_invocation_diagnostic(
                &second.id,
                AgentDiagnostic {
                    source: AgentDiagnosticSource::Repository,
                    severity: AgentDiagnosticSeverity::Warning,
                    code: "persist_warning".into(),
                    message: "Retained independently of runtime outcome".into(),
                    details: Some(json!({"attempt": 1})),
                    recorded_at: at(8),
                },
            )
            .expect("append diagnostic");
        repository
            .append_event(unknown_event(
                "event-a",
                &second.id,
                0,
                json!({"unknown":"raw-only","large":"x".repeat(16_384)}),
                at(9),
            ))
            .expect("append second event");
        repository
            .finish_invocation(
                &second.id,
                InvocationCompletion {
                    status: AgentInvocationTerminalStatus::Interrupted,
                    completed_at: at(10),
                    exit_code: None,
                    signal: Some("shutdown".into()),
                    runtime_error: Some(AgentRuntimeFailure {
                        code: "app_shutdown".into(),
                        message: "Application closed".into(),
                        details: Some(json!({"recoverable": true})),
                    }),
                },
                at(10),
            )
            .expect("interrupt second invocation");
        drop(repository);

        let reopened = SqliteAgentSessionRepository::open(&path).expect("reopen repository");
        let history = reopened
            .load_session_history(&session.id)
            .expect("load complete history")
            .expect("session history");

        assert_eq!(
            history
                .session
                .runtime_binding
                .external_context_id
                .as_ref()
                .map(ExternalRuntimeContextId::as_str),
            Some("provider-thread")
        );
        assert_eq!(
            history
                .invocations
                .iter()
                .map(|entry| entry.invocation.id.as_str())
                .collect::<Vec<_>>(),
            vec!["invocation-b", "invocation-a"]
        );
        assert_eq!(
            history.invocations[0]
                .events
                .iter()
                .map(|event| event.sequence)
                .collect::<Vec<_>>(),
            vec![2, 9]
        );
        assert_eq!(history.invocations[0].events[0].normalized, None);
        assert_eq!(
            history.invocations[0].events[0].raw_payload,
            json!({"type":"future.event","nested":{"unchanged":[1,true,null]}})
        );
        assert_eq!(history.invocations[1].invocation.diagnostics.len(), 1);
        assert_eq!(
            history.invocations[1].events[0].raw_payload["large"]
                .as_str()
                .expect("large raw payload")
                .len(),
            16_384
        );

        let summaries = reopened
            .list_session_summaries(ListAgentSessionsQuery::default())
            .expect("session summaries");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].invocation_count, 2);
        assert_eq!(
            summaries[0].latest_invocation_status,
            Some(AgentInvocationStatus::Interrupted)
        );
        assert_eq!(
            summaries[0].latest_submitted_text.as_deref(),
            Some("input for invocation-a")
        );

        drop(reopened);
        fs::remove_file(path).expect("remove test database");
    }

    #[test]
    fn enforces_one_active_invocation_and_rolls_back_rejected_create() {
        let repository = memory_repository();
        let session = repository
            .create_session(test_session("session", at(0)))
            .expect("create session");
        repository
            .create_pending_invocation(test_invocation("first", &session.id, at(1)))
            .expect("create active invocation");

        let error = repository
            .create_pending_invocation(test_invocation("second", &session.id, at(2)))
            .expect_err("reject second active invocation");

        assert_eq!(error.kind, RepositoryErrorKind::Conflict);
        assert!(repository
            .get_invocation(&AgentInvocationId::new("second").expect("ID"))
            .expect("query rejected invocation")
            .is_none());
        assert_eq!(
            repository
                .list_invocations(&session.id)
                .expect("invocations")
                .len(),
            1
        );
    }

    #[test]
    fn rejects_duplicate_or_reordered_event_sequences_without_partial_write() {
        let repository = memory_repository();
        let session = repository
            .create_session(test_session("session", at(0)))
            .expect("create session");
        let invocation = repository
            .create_pending_invocation(test_invocation("invocation", &session.id, at(1)))
            .expect("create invocation");
        repository
            .append_event(unknown_event(
                "event-10",
                &invocation.id,
                10,
                json!({"first": true}),
                at(2),
            ))
            .expect("append event");

        let error = repository
            .append_event(unknown_event(
                "event-9",
                &invocation.id,
                9,
                json!({"late": true}),
                at(3),
            ))
            .expect_err("reject decreasing event sequence");

        assert_eq!(error.kind, RepositoryErrorKind::Conflict);
        assert_eq!(
            repository
                .list_events(&invocation.id)
                .expect("events")
                .len(),
            1
        );
    }

    #[test]
    fn orders_session_lists_by_update_time_then_id_and_filters_availability() {
        let repository = memory_repository();
        repository
            .create_session(test_session("z-session", at(0)))
            .expect("create z session");
        repository
            .create_session(test_session("a-session", at(0)))
            .expect("create a session");
        let archived = repository
            .create_session(test_session("archived", at(1)))
            .expect("create archived session");
        repository
            .set_session_availability(&archived.id, AgentSessionAvailability::Archived, at(2))
            .expect("archive session");

        assert_eq!(
            repository
                .list_sessions(ListAgentSessionsQuery {
                    availability: Some(AgentSessionAvailability::Available),
                    limit: None,
                })
                .expect("available sessions")
                .iter()
                .map(|session| session.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a-session", "z-session"]
        );
        assert_eq!(
            repository
                .list_sessions(ListAgentSessionsQuery {
                    availability: None,
                    limit: Some(1),
                })
                .expect("limited sessions")[0]
                .id
                .as_str(),
            "archived"
        );
    }

    fn memory_repository() -> SqliteAgentSessionRepository {
        let connection = Connection::open_in_memory().expect("memory database");
        connection
            .execute_batch(&format!("PRAGMA foreign_keys = ON; {AGENT_SESSION_SCHEMA}"))
            .expect("initialize Agent Session schema");
        SqliteAgentSessionRepository::new(connection)
    }

    fn initialized_file_database(path: &PathBuf) -> Connection {
        let connection = Connection::open(path).expect("file database");
        connection
            .execute_batch(&format!("PRAGMA foreign_keys = ON; {AGENT_SESSION_SCHEMA}"))
            .expect("initialize Agent Session schema");
        connection
    }

    fn temporary_database_path() -> PathBuf {
        std::env::temp_dir().join(format!("codex-agent-session-{}.sqlite", Uuid::new_v4()))
    }

    fn test_session(id: &str, created_at: DateTime<Utc>) -> AgentSession {
        AgentSession {
            id: AgentSessionId::new(id).expect("session ID"),
            title: format!("Session {id}"),
            availability: AgentSessionAvailability::Available,
            runtime_binding: AgentRuntimeBinding {
                kind: AgentRuntimeKind::CodexCli,
                external_context_id: None,
                runtime_version: None,
            },
            working_directory: Some("C:/work".into()),
            requested_options: options(),
            created_at,
            updated_at: created_at,
        }
    }

    fn test_invocation(
        id: &str,
        session_id: &AgentSessionId,
        created_at: DateTime<Utc>,
    ) -> AgentInvocation {
        AgentInvocation {
            id: AgentInvocationId::new(id).expect("invocation ID"),
            session_id: session_id.clone(),
            submitted_text: format!("input for {id}"),
            status: AgentInvocationStatus::Pending,
            requested_options: options(),
            effective_options: None,
            started_at: None,
            completed_at: None,
            exit_code: None,
            signal: None,
            runtime_error: None,
            diagnostics: Vec::new(),
            created_at,
            updated_at: created_at,
        }
    }

    fn unknown_event(
        id: &str,
        invocation_id: &AgentInvocationId,
        sequence: u64,
        raw_payload: serde_json::Value,
        recorded_at: DateTime<Utc>,
    ) -> AgentRuntimeEvent {
        AgentRuntimeEvent {
            id: AgentRuntimeEventId::new(id).expect("event ID"),
            invocation_id: invocation_id.clone(),
            sequence,
            source: AgentRuntimeEventSource::Stdout,
            raw_payload,
            normalized: None,
            recorded_at,
        }
    }

    fn normalized_event(
        id: &str,
        invocation_id: &AgentInvocationId,
        sequence: u64,
        recorded_at: DateTime<Utc>,
    ) -> AgentRuntimeEvent {
        AgentRuntimeEvent {
            id: AgentRuntimeEventId::new(id).expect("event ID"),
            invocation_id: invocation_id.clone(),
            sequence,
            source: AgentRuntimeEventSource::Runtime,
            raw_payload: json!({"type":"agent_message","text":"done"}),
            normalized: Some(NormalizedRuntimeEvent {
                kind: NormalizedRuntimeEventKind::AgentMessage,
                text: Some("done".into()),
                external_context_id: None,
                usage: None,
                details: Some(json!({"final": true})),
            }),
            recorded_at,
        }
    }

    fn options() -> AgentRuntimeOptions {
        AgentRuntimeOptions {
            model: Some("test-model".into()),
            sandbox: None,
        }
    }

    fn at(second: u32) -> DateTime<Utc> {
        format!("2026-07-10T12:00:{second:02}Z")
            .parse()
            .expect("timestamp")
    }
}
