mod mapping;
mod schema;

#[cfg(test)]
mod tests;

pub(crate) use schema::{
    quarantine_archived_prototype_tables, AGENT_SESSION_LAUNCH_ACCEPTANCE_SCHEMA,
    AGENT_SESSION_SCHEMA,
};

use self::mapping::*;
use super::{
    domain::{
        validate_new_invocation, validate_next_event, validate_runtime_binding_update,
        validate_session, validate_session_update, AgentDiagnostic, AgentInvocation,
        AgentInvocationId, AgentInvocationStatus, AgentRuntimeBinding, AgentRuntimeEvent,
        AgentRuntimeOptions, AgentSession, AgentSessionAvailability, AgentSessionId,
        InvocationCompletion,
    },
    ports::{
        AgentInvocationHistory, AgentSessionHistory, AgentSessionRepository, AgentSessionSummary,
        ListAgentSessionsQuery, RepositoryError, RepositoryErrorKind,
    },
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

pub(crate) struct SqliteAgentSessionRepository {
    connection: Mutex<Connection>,
}

impl SqliteAgentSessionRepository {
    pub(crate) fn new(connection: Connection) -> Result<Self, RepositoryError> {
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(sql_unavailable("configure Agent Session database"))?;
        let foreign_keys_enabled = connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .map_err(sql_unavailable("verify Agent Session foreign keys"))?;
        if foreign_keys_enabled != 1 {
            return Err(RepositoryError::new(
                RepositoryErrorKind::Unavailable,
                "Agent Session database did not enable foreign keys",
            ));
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, RepositoryError> {
        let connection =
            Connection::open(path).map_err(sql_unavailable("open Agent Session database"))?;
        Self::new(connection)
    }

    fn load_session_history_snapshot(
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
                let launch_accepted_at =
                    invocation_launch_accepted_at_from(&transaction, &invocation.id)?;
                let events = list_events_from(&transaction, &invocation.id)?;
                Ok(AgentInvocationHistory {
                    invocation,
                    launch_accepted_at,
                    events,
                })
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

    fn list_session_summaries_snapshot(
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

    fn load_session_history(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSessionHistory>, RepositoryError> {
        self.load_session_history_snapshot(session_id)
    }

    fn list_session_summaries(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSessionSummary>, RepositoryError> {
        self.list_session_summaries_snapshot(query)
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
                "UPDATE agent_sessions SET external_context_id = ?1, runtime_version = ?2, updated_at = ?3 WHERE id = ?4",
                params![
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

    fn record_invocation_launch_accepted(
        &self,
        invocation_id: &AgentInvocationId,
        accepted_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT OR IGNORE INTO agent_session_invocation_launch_acceptances (invocation_id, accepted_at) VALUES (?1, ?2)",
                params![invocation_id.as_str(), timestamp(accepted_at)],
            )
            .map_err(sql_write("record invocation launch acceptance"))?;
        Ok(())
    }

    fn invocation_launch_accepted_at(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        let connection = self.lock()?;
        invocation_launch_accepted_at_from(&connection, invocation_id)
    }

    fn recover_pre_acceptance_interruption(
        &self,
        invocation_id: &AgentInvocationId,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_unavailable("begin pre-acceptance invocation recovery"))?;
        if invocation_launch_accepted_at_from(&transaction, invocation_id)?.is_some() {
            return Err(RepositoryError::new(
                RepositoryErrorKind::Conflict,
                "launch-accepted invocation cannot return to pre-acceptance state",
            ));
        }
        let current = required_invocation(&transaction, invocation_id)?;
        let updated = current
            .recover_pre_acceptance_interruption(updated_at)
            .map_err(contract_error)?;
        update_invocation(&transaction, &updated)?;
        touch_session(&transaction, &updated.session_id, updated_at)?;
        transaction
            .commit()
            .map_err(sql_unavailable("commit pre-acceptance invocation recovery"))?;
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
