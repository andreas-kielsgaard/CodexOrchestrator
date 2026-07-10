use super::super::{
    domain::{
        AgentDiagnostic, AgentInvocation, AgentInvocationId, AgentInvocationStatus,
        AgentRuntimeBinding, AgentRuntimeEvent, AgentRuntimeEventId, AgentRuntimeEventSource,
        AgentRuntimeKind, AgentSession, AgentSessionAvailability, AgentSessionId,
        ContractViolation, ExternalRuntimeContextId, NormalizedRuntimeEvent,
    },
    ports::{ListAgentSessionsQuery, RepositoryError, RepositoryErrorKind},
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension, Row, Transaction};

pub(super) type SessionRow = (
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
pub(super) type InvocationRow = (
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
pub(super) type EventRow = (String, String, i64, String, String, Option<String>, String);

pub(super) fn session_row(row: &Row<'_>) -> rusqlite::Result<SessionRow> {
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

pub(super) fn invocation_row(row: &Row<'_>) -> rusqlite::Result<InvocationRow> {
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

pub(super) fn event_row(row: &Row<'_>) -> rusqlite::Result<EventRow> {
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

pub(super) fn map_session_row(row: SessionRow) -> Result<AgentSession, RepositoryError> {
    Ok(AgentSession {
        id: AgentSessionId::new(row.0).map_err(contract_error)?,
        title: row.1,
        availability: parse_availability(&row.2)?,
        runtime_binding: AgentRuntimeBinding {
            kind: parse_runtime_kind(&row.3)?,
            external_context_id: row
                .4
                .map(ExternalRuntimeContextId::new)
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

pub(super) fn map_invocation_row(row: InvocationRow) -> Result<AgentInvocation, RepositoryError> {
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

pub(super) fn map_event_row(row: EventRow) -> Result<AgentRuntimeEvent, RepositoryError> {
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

pub(super) fn collect_mapped<T, U>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&Row<'_>) -> rusqlite::Result<T>>,
    map: impl Fn(T) -> Result<U, RepositoryError>,
) -> Result<Vec<U>, RepositoryError> {
    rows.map(|row| {
        row.map_err(sql_unavailable("read Agent Session row"))
            .and_then(&map)
    })
    .collect()
}

pub(super) fn insert_session(
    transaction: &Transaction<'_>,
    session: &AgentSession,
) -> Result<(), RepositoryError> {
    transaction.execute(
        "INSERT INTO agent_sessions (id, title, availability, runtime_kind, external_context_id, runtime_version, working_directory, requested_options_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![session.id.as_str(), session.title, availability_text(session.availability), runtime_kind_text(session.runtime_binding.kind), session.runtime_binding.external_context_id.as_ref().map(|id| id.as_str()), session.runtime_binding.runtime_version, session.working_directory, to_json(&session.requested_options)?, timestamp(session.created_at), timestamp(session.updated_at)],
    ).map_err(sql_write("create Agent Session"))?;
    Ok(())
}

pub(super) fn insert_invocation(
    transaction: &Transaction<'_>,
    invocation: &AgentInvocation,
) -> Result<(), RepositoryError> {
    transaction.execute(
        "INSERT INTO agent_session_invocations (id, session_id, submitted_text, status, requested_options_json, effective_options_json, started_at, completed_at, exit_code, signal, runtime_error_json, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![invocation.id.as_str(), invocation.session_id.as_str(), invocation.submitted_text, status_text(invocation.status), to_json(&invocation.requested_options)?, invocation.effective_options.as_ref().map(to_json).transpose()?, invocation.started_at.map(timestamp), invocation.completed_at.map(timestamp), invocation.exit_code, invocation.signal, invocation.runtime_error.as_ref().map(to_json).transpose()?, timestamp(invocation.created_at), timestamp(invocation.updated_at)],
    ).map_err(sql_write("create pending invocation"))?;
    Ok(())
}

pub(super) fn update_invocation(
    transaction: &Transaction<'_>,
    invocation: &AgentInvocation,
) -> Result<(), RepositoryError> {
    transaction.execute(
        "UPDATE agent_session_invocations SET status = ?1, effective_options_json = ?2, started_at = ?3, completed_at = ?4, exit_code = ?5, signal = ?6, runtime_error_json = ?7, updated_at = ?8 WHERE id = ?9",
        params![status_text(invocation.status), invocation.effective_options.as_ref().map(to_json).transpose()?, invocation.started_at.map(timestamp), invocation.completed_at.map(timestamp), invocation.exit_code, invocation.signal, invocation.runtime_error.as_ref().map(to_json).transpose()?, timestamp(invocation.updated_at), invocation.id.as_str()],
    ).map_err(sql_write("update invocation"))?;
    Ok(())
}

pub(super) const SESSION_SELECT: &str = "SELECT id, title, availability, runtime_kind, external_context_id, runtime_version, working_directory, requested_options_json, created_at, updated_at FROM agent_sessions";
pub(super) const INVOCATION_SELECT: &str = "SELECT id, session_id, submitted_text, status, requested_options_json, effective_options_json, started_at, completed_at, exit_code, signal, runtime_error_json, created_at, updated_at FROM agent_session_invocations";

pub(super) fn list_sessions_from(
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

pub(super) fn list_invocations_from(
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

pub(super) fn list_events_from(
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

pub(super) fn get_session_from(
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

pub(super) fn required_session(
    conn: &Connection,
    session_id: &AgentSessionId,
) -> Result<AgentSession, RepositoryError> {
    get_session_from(conn, session_id)?.ok_or_else(|| not_found("session not found"))
}

pub(super) fn get_invocation_from(
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

pub(super) fn required_invocation(
    conn: &Connection,
    invocation_id: &AgentInvocationId,
) -> Result<AgentInvocation, RepositoryError> {
    get_invocation_from(conn, invocation_id)?.ok_or_else(|| not_found("invocation not found"))
}

pub(super) fn active_invocation(
    conn: &Connection,
    session_id: &AgentSessionId,
) -> Result<Option<AgentInvocation>, RepositoryError> {
    let row = conn.query_row(&format!("{INVOCATION_SELECT} WHERE session_id = ?1 AND status IN ('pending', 'running') LIMIT 1"), params![session_id.as_str()], invocation_row).optional().map_err(sql_unavailable("load active invocation"))?;
    row.map(map_invocation_row).transpose()
}

pub(super) fn load_diagnostics(
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

pub(super) fn last_event(
    conn: &Connection,
    invocation_id: &AgentInvocationId,
) -> Result<Option<AgentRuntimeEvent>, RepositoryError> {
    let row = conn.query_row("SELECT id, invocation_id, sequence, source, raw_payload_json, normalized_json, recorded_at FROM agent_session_runtime_events WHERE invocation_id = ?1 ORDER BY sequence DESC LIMIT 1", params![invocation_id.as_str()], event_row).optional().map_err(sql_unavailable("load last runtime event"))?;
    row.map(map_event_row).transpose()
}

pub(super) fn touch_session(
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

pub(super) fn to_json<T: serde::Serialize>(value: &T) -> Result<String, RepositoryError> {
    serde_json::to_string(value)
        .map_err(|error| invalid_data(format!("unable to serialize Agent Session data: {error}")))
}

pub(super) fn from_json<T: serde::de::DeserializeOwned>(
    value: &str,
    context: &str,
) -> Result<T, RepositoryError> {
    serde_json::from_str(value).map_err(|error| invalid_data(format!("invalid {context}: {error}")))
}

pub(super) fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Nanos, true)
}
pub(super) fn parse_timestamp(value: &str) -> Result<DateTime<Utc>, RepositoryError> {
    value
        .parse()
        .map_err(|error| invalid_data(format!("invalid timestamp {value}: {error}")))
}

pub(super) fn availability_text(value: AgentSessionAvailability) -> &'static str {
    match value {
        AgentSessionAvailability::Available => "available",
        AgentSessionAvailability::Archived => "archived",
    }
}
pub(super) fn parse_availability(value: &str) -> Result<AgentSessionAvailability, RepositoryError> {
    match value {
        "available" => Ok(AgentSessionAvailability::Available),
        "archived" => Ok(AgentSessionAvailability::Archived),
        _ => Err(invalid_data(format!(
            "unknown session availability {value}"
        ))),
    }
}
pub(super) fn runtime_kind_text(_: AgentRuntimeKind) -> &'static str {
    "codex_cli"
}
pub(super) fn parse_runtime_kind(value: &str) -> Result<AgentRuntimeKind, RepositoryError> {
    match value {
        "codex_cli" => Ok(AgentRuntimeKind::CodexCli),
        _ => Err(invalid_data(format!("unknown runtime kind {value}"))),
    }
}
pub(super) fn status_text(value: AgentInvocationStatus) -> &'static str {
    match value {
        AgentInvocationStatus::Pending => "pending",
        AgentInvocationStatus::Running => "running",
        AgentInvocationStatus::Completed => "completed",
        AgentInvocationStatus::Failed => "failed",
        AgentInvocationStatus::Canceled => "canceled",
        AgentInvocationStatus::Interrupted => "interrupted",
    }
}
pub(super) fn parse_status(value: &str) -> Result<AgentInvocationStatus, RepositoryError> {
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
pub(super) fn event_source_text(value: AgentRuntimeEventSource) -> &'static str {
    match value {
        AgentRuntimeEventSource::Stdout => "stdout",
        AgentRuntimeEventSource::Stderr => "stderr",
        AgentRuntimeEventSource::Runtime => "runtime",
    }
}
pub(super) fn parse_event_source(value: &str) -> Result<AgentRuntimeEventSource, RepositoryError> {
    match value {
        "stdout" => Ok(AgentRuntimeEventSource::Stdout),
        "stderr" => Ok(AgentRuntimeEventSource::Stderr),
        "runtime" => Ok(AgentRuntimeEventSource::Runtime),
        _ => Err(invalid_data(format!(
            "unknown runtime event source {value}"
        ))),
    }
}

pub(super) fn contract_error(error: ContractViolation) -> RepositoryError {
    let kind = match error {
        ContractViolation::ActiveInvocationExists { .. }
        | ContractViolation::EventSequenceNotIncreasing { .. } => RepositoryErrorKind::Conflict,
        _ => RepositoryErrorKind::InvalidState,
    };
    RepositoryError::new(kind, error.to_string())
}
pub(super) fn not_found(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::NotFound, message)
}
pub(super) fn invalid_data(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Unavailable, message)
}
pub(super) fn sql_unavailable(
    context: &'static str,
) -> impl FnOnce(rusqlite::Error) -> RepositoryError {
    move |error| {
        RepositoryError::new(
            RepositoryErrorKind::Unavailable,
            format!("Unable to {context}: {error}"),
        )
    }
}
pub(super) fn sql_write(context: &'static str) -> impl FnOnce(rusqlite::Error) -> RepositoryError {
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
