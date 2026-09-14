mod addressing;
mod mapping;
mod organization;
pub(crate) use organization::write_placement;
pub(crate) use organization::SCHEMA as SESSION_ORGANIZATION_SCHEMA;
mod schema;

#[cfg(test)]
mod tests;

pub(crate) use schema::{
    ensure_agent_session_ownership_schema, AGENT_SESSION_LAUNCH_ACCEPTANCE_SCHEMA,
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
use crate::{harness_engine::domain::HarnessVersionRef, identities::AssignedAgentIdentity};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
#[cfg(any(test, feature = "live-tests"))]
use std::path::Path;
use std::sync::Arc;

use crate::persistence::{ActiveDatabase, ManagedOperationError};

pub(crate) struct SqliteAgentSessionRepository {
    database: Arc<ActiveDatabase>,
}

impl SqliteAgentSessionRepository {
    pub(crate) fn from_database(database: Arc<ActiveDatabase>) -> Self {
        Self { database }
    }

    #[cfg(any(test, feature = "live-tests"))]
    pub(crate) fn new(connection: Connection) -> Result<Self, RepositoryError> {
        ActiveDatabase::from_connection(connection, initialize_agent_session_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .map_err(persistence_unavailable)
    }

    #[cfg(any(test, feature = "live-tests"))]

    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, RepositoryError> {
        ActiveDatabase::open(path, initialize_agent_session_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .map_err(persistence_unavailable)
    }

    fn load_session_history_snapshot(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSessionHistory>, RepositoryError> {
        self.read("load Agent Session history", |connection| {
            let transaction = connection
                .unchecked_transaction()
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
        })
    }

    fn list_session_summaries_snapshot(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSessionSummary>, RepositoryError> {
        self.read("list Agent Session summaries", |connection| {
        let transaction = connection
            .unchecked_transaction()
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
                        "SELECT status, submitted_text, id FROM agent_session_invocations WHERE session_id = ?1 ORDER BY created_at DESC, id DESC LIMIT 1",
                        params![session.id.as_str()],
                        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
                    )
                    .optional()
                    .map_err(sql_unavailable("load latest Agent Session invocation"))?;
                let (latest_invocation_status, latest_submitted_text, pending_request_count) = match latest {
                    Some((status, text, id)) => {
                        let status = parse_status(&status)?;
                        let count = if status.is_active() {
                            let id = AgentInvocationId::new(id).map_err(contract_error)?;
                            let invocation = get_invocation_from(&transaction, &id)?
                                .ok_or_else(|| RepositoryError::new(RepositoryErrorKind::Unavailable, "Latest invocation disappeared"))?;
                            super::interactions::pending_request_count(&[AgentInvocationHistory {
                                invocation,
                                launch_accepted_at: None,
                                events: list_events_from(&transaction, &id)?,
                            }])
                        } else { 0 };
                        (Some(status), Some(text), count)
                    }
                    None => (None, None, 0),
                };
                Ok(AgentSessionSummary {
                    pending_request_count,
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
    })
    }

    fn read<T>(
        &self,
        operation: &'static str,
        read: impl FnOnce(&Connection) -> Result<T, RepositoryError>,
    ) -> Result<T, RepositoryError> {
        self.database.read(operation, read).map_err(managed_error)
    }

    fn write<T>(
        &self,
        operation: &'static str,
        write: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, RepositoryError>,
    ) -> Result<T, RepositoryError> {
        self.database.write(operation, write).map_err(managed_error)
    }
}

impl AgentSessionRepository for SqliteAgentSessionRepository {
    fn create_session_with_placement(
        &self,
        session: AgentSession,
        placement: &super::organization::SessionPlacement,
    ) -> Result<AgentSession, RepositoryError> {
        SqliteAgentSessionRepository::create_session_with_placement(self, session, placement)
    }

    fn resolve_working_directory(
        &self,
        session_id: &AgentSessionId,
        path: &str,
        origin: &str,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        self.write("resolve Session working directory", |connection| {
        let changed = connection.execute("UPDATE agent_sessions SET working_directory=?2,workspace_origin=?3,updated_at=?4 WHERE id=?1 AND working_directory IS NULL", params![session_id.as_str(),path,origin,timestamp(updated_at)]).map_err(sql_write("resolve Session working context"))?;
        let session = get_session_from(&connection, session_id)?.ok_or_else(|| {
            RepositoryError::new(RepositoryErrorKind::NotFound, "Session not found")
        })?;
        if changed == 0 && session.working_directory.as_deref() != Some(path) {
            return Err(RepositoryError::new(
                RepositoryErrorKind::Conflict,
                "Session already has a different working directory",
            ));
        }
        Ok(session)
        })
    }
    fn create_session(&self, session: AgentSession) -> Result<AgentSession, RepositoryError> {
        validate_session(&session).map_err(contract_error)?;
        self.write("create Agent Session", |transaction| {
            insert_session(transaction, &session)?;
            Ok(session)
        })
    }

    fn get_session(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSession>, RepositoryError> {
        self.read("load Agent Session", |connection| {
            get_session_from(connection, session_id)
        })
    }

    fn list_sessions(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSession>, RepositoryError> {
        self.read("list Agent Sessions", |connection| {
            list_sessions_from(connection, query)
        })
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
        self.write("update Agent Session availability", |transaction| {
            let current = required_session(transaction, session_id)?;
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
            Ok(candidate)
        })
    }

    fn update_runtime_binding(
        &self,
        session_id: &AgentSessionId,
        binding: AgentRuntimeBinding,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        self.write("update Agent Session runtime binding", |transaction| {
        let current = required_session(transaction, session_id)?;
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
        Ok(candidate)
        })
    }

    fn update_harness_version(
        &self,
        session_id: &AgentSessionId,
        harness_version: Option<HarnessVersionRef>,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        self.write("update Agent Session Harness", |transaction| {
            let current = required_session(transaction, session_id)?;
            let mut candidate = current.clone();
            candidate.harness_version = harness_version;
            candidate.updated_at = updated_at;
            validate_session_update(&current, &candidate).map_err(contract_error)?;
            transaction
                .execute(
                    "UPDATE agent_sessions SET harness_version_ref_json = ?1, updated_at = ?2 WHERE id = ?3",
                    params![
                        candidate.harness_version.as_ref().map(to_json).transpose()?,
                        timestamp(updated_at),
                        session_id.as_str()
                    ],
                )
                .map_err(sql_unavailable("update session Harness"))?;
            Ok(candidate)
        })
    }

    fn update_assigned_identity(
        &self,
        session_id: &AgentSessionId,
        assigned_identity: Option<AssignedAgentIdentity>,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        self.write("update assigned Agent identity", |transaction| {
            let current = required_session(transaction, session_id)?;
            let mut candidate = current.clone();
            candidate.assigned_identity = assigned_identity;
            candidate.updated_at = updated_at;
            validate_session_update(&current, &candidate).map_err(contract_error)?;
            transaction
                .execute(
                    "UPDATE agent_sessions SET assigned_identity_json = ?1, updated_at = ?2 WHERE id = ?3",
                    params![
                        candidate.assigned_identity.as_ref().map(to_json).transpose()?,
                        timestamp(updated_at),
                        session_id.as_str()
                    ],
                )
                .map_err(sql_unavailable("update assigned Agent identity"))?;
            Ok(candidate)
        })
    }

    fn update_session_model_override(
        &self,
        session_id: &AgentSessionId,
        model: Option<String>,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        self.write("update Agent Session model override", |transaction| {
            let current = required_session(transaction, session_id)?;
            let mut candidate = current.clone();
            candidate.requested_options.model = model;
            candidate.updated_at = updated_at;
            validate_session_update(&current, &candidate).map_err(contract_error)?;
            transaction
                .execute(
                    "UPDATE agent_sessions SET requested_options_json = ?1, updated_at = ?2 WHERE id = ?3",
                    params![
                        to_json(&candidate.requested_options)?,
                        timestamp(updated_at),
                        session_id.as_str()
                    ],
                )
                .map_err(sql_unavailable("update Session model override"))?;
            Ok(candidate)
        })
    }

    fn create_pending_invocation(
        &self,
        invocation: AgentInvocation,
    ) -> Result<AgentInvocation, RepositoryError> {
        self.write("create pending Agent Session invocation", |transaction| {
            let session = required_session(transaction, &invocation.session_id)?;
            let active = active_invocation(transaction, &invocation.session_id)?;
            validate_new_invocation(&session, active.as_ref(), &invocation)
                .map_err(contract_error)?;
            insert_invocation(transaction, &invocation)?;
            transaction
                .execute(
                    "UPDATE agent_sessions SET updated_at = MAX(updated_at, ?1) WHERE id = ?2",
                    params![
                        timestamp(invocation.updated_at),
                        invocation.session_id.as_str()
                    ],
                )
                .map_err(sql_unavailable("touch Agent Session"))?;
            Ok(invocation)
        })
    }

    fn get_invocation(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<AgentInvocation>, RepositoryError> {
        self.read("load Agent Session invocation", |connection| {
            get_invocation_from(connection, invocation_id)
        })
    }

    fn list_invocations(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Vec<AgentInvocation>, RepositoryError> {
        self.read("list Agent Session invocations", |connection| {
            list_invocations_from(connection, session_id)
        })
    }

    fn mark_invocation_running(
        &self,
        invocation_id: &AgentInvocationId,
        started_at: DateTime<Utc>,
        effective_options: AgentRuntimeOptions,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        self.write("start Agent Session invocation", |transaction| {
            let current = required_invocation(transaction, invocation_id)?;
            let updated = current
                .mark_running(started_at, effective_options, updated_at)
                .map_err(contract_error)?;
            update_invocation(transaction, &updated)?;
            touch_session(transaction, &updated.session_id, updated_at)?;
            Ok(updated)
        })
    }

    fn record_invocation_launch_accepted(
        &self,
        invocation_id: &AgentInvocationId,
        accepted_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        self.write("record invocation launch acceptance", |transaction| {
            transaction
            .execute(
                "INSERT OR IGNORE INTO agent_session_invocation_launch_acceptances (invocation_id, accepted_at) VALUES (?1, ?2)",
                params![invocation_id.as_str(), timestamp(accepted_at)],
            )
            .map_err(sql_write("record invocation launch acceptance"))?;
        Ok(())
    })
    }

    fn invocation_launch_accepted_at(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        self.read("load invocation launch acceptance", |connection| {
            invocation_launch_accepted_at_from(connection, invocation_id)
        })
    }

    fn recover_pre_acceptance_interruption(
        &self,
        invocation_id: &AgentInvocationId,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        self.write("recover pre-acceptance invocation", |transaction| {
            if invocation_launch_accepted_at_from(transaction, invocation_id)?.is_some() {
                return Err(RepositoryError::new(
                    RepositoryErrorKind::Conflict,
                    "launch-accepted invocation cannot return to pre-acceptance state",
                ));
            }
            let current = required_invocation(transaction, invocation_id)?;
            let updated = current
                .recover_pre_acceptance_interruption(updated_at)
                .map_err(contract_error)?;
            update_invocation(transaction, &updated)?;
            touch_session(transaction, &updated.session_id, updated_at)?;
            Ok(updated)
        })
    }

    fn finish_invocation(
        &self,
        invocation_id: &AgentInvocationId,
        completion: InvocationCompletion,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        self.write("finish Agent Session invocation", |transaction| {
            let current = required_invocation(transaction, invocation_id)?;
            let requested_status = AgentInvocationStatus::from(completion.status);
            if current.status == requested_status
                && current.completed_at == Some(completion.completed_at)
                && current.exit_code == completion.exit_code
                && current.signal == completion.signal
                && current.runtime_error == completion.runtime_error
            {
                return Ok(current);
            }
            let updated = current
                .finish(completion, updated_at)
                .map_err(contract_error)?;
            update_invocation(transaction, &updated)?;
            touch_session(transaction, &updated.session_id, updated_at)?;
            Ok(updated)
        })
    }

    fn append_invocation_diagnostic(
        &self,
        invocation_id: &AgentInvocationId,
        diagnostic: AgentDiagnostic,
    ) -> Result<AgentInvocation, RepositoryError> {
        self.write("append Agent Session diagnostic", |transaction| {
        let mut invocation = required_invocation(transaction, invocation_id)?;
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
        Ok(invocation)
        })
    }

    fn append_event(&self, event: AgentRuntimeEvent) -> Result<AgentRuntimeEvent, RepositoryError> {
        self.write("append Agent Session runtime event", |transaction| {
        if get_invocation_from(transaction, &event.invocation_id)?.is_none() {
            return Err(not_found("invocation not found"));
        }
        let previous = last_event(transaction, &event.invocation_id)?;
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
        Ok(event)
        })
    }

    fn list_events(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Vec<AgentRuntimeEvent>, RepositoryError> {
        self.read("list Agent Session runtime events", |connection| {
            list_events_from(connection, invocation_id)
        })
    }
}

#[cfg(any(test, feature = "live-tests"))]
fn initialize_agent_session_storage(connection: &Connection) -> Result<(), String> {
    let exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='agent_sessions')",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(|error| format!("Unable to inspect Agent Session storage: {error}"))?;
    if !exists {
        connection
            .execute_batch(AGENT_SESSION_SCHEMA)
            .map_err(|error| format!("Unable to initialize Agent Session storage: {error}"))?;
    }
    ensure_agent_session_ownership_schema(connection)?;
    initialize_session_address_storage(connection)?;
    connection
        .execute_batch(SESSION_ORGANIZATION_SCHEMA)
        .map_err(|e| e.to_string())?;
    connection
        .execute_batch(AGENT_SESSION_LAUNCH_ACCEPTANCE_SCHEMA)
        .map_err(|error| format!("Unable to initialize Agent Session launch storage: {error}"))
}

fn persistence_unavailable(error: crate::persistence::PersistenceError) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::Unavailable, error.to_string())
}

fn managed_error(error: ManagedOperationError<RepositoryError>) -> RepositoryError {
    match error {
        ManagedOperationError::Infrastructure(error) => persistence_unavailable(error),
        ManagedOperationError::Domain(error) => error,
    }
}

pub(crate) fn initialize_session_address_storage(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(addressing::SCHEMA)
        .map_err(|error| format!("Unable to initialize Session addresses: {error}"))
}
