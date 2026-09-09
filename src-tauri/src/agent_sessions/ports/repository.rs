use crate::agent_sessions::domain::{
    AgentDiagnostic, AgentInvocation, AgentInvocationId, AgentRuntimeBinding, AgentRuntimeEvent,
    AgentRuntimeOptions, AgentSession, AgentSessionAvailability, AgentSessionId,
    InvocationCompletion,
};
use crate::{harness_engine::domain::HarnessVersionRef, identities::AssignedAgentIdentity};
use chrono::{DateTime, Utc};
use std::{error::Error, fmt};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AgentSessionHistory {
    pub(crate) session: AgentSession,
    pub(crate) invocations: Vec<AgentInvocationHistory>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AgentInvocationHistory {
    pub(crate) invocation: AgentInvocation,
    /// The application/process launch acknowledgement, if durably recorded. This is not
    /// inferred from invocation lifecycle fields or provider events.
    pub(crate) launch_accepted_at: Option<DateTime<Utc>>,
    pub(crate) events: Vec<AgentRuntimeEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AgentSessionSummary {
    pub(crate) pending_request_count: usize,
    pub(crate) session: AgentSession,
    pub(crate) invocation_count: u64,
    pub(crate) latest_invocation_status:
        Option<crate::agent_sessions::domain::AgentInvocationStatus>,
    pub(crate) latest_submitted_text: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ListAgentSessionsQuery {
    pub(crate) availability: Option<AgentSessionAvailability>,
    pub(crate) limit: Option<u32>,
}

/// Atomic persistence of a session together with its opaque Session Events address.
pub(crate) trait AgentSessionAddressStore: Send + Sync {
    fn create_addressed_session(
        &self,
        session: AgentSession,
        address: &crate::session_events::SessionLogicalAddress,
        event: &crate::session_events::ReferenceIdentity,
        creator: Option<&crate::session_events::ReferenceIdentity>,
    ) -> Result<(AgentSession, u64), RepositoryError>;
    fn find_session_entry(
        &self,
        id: &AgentSessionId,
    ) -> Result<
        Option<crate::session_events::SessionDirectoryEntry>,
        crate::session_events::SessionDirectoryError,
    >;
}

/// Durable Agent Session storage operations.
///
/// Implementations must make `create_pending_invocation` atomically reject a second active
/// invocation for the same session and must make identical terminal completion requests
/// idempotent. Session lists are ordered by update time descending with ID as a stable tie-breaker;
/// invocation lists are ordered by creation time ascending with ID as a tie-breaker; event lists
/// are ordered by sequence ascending. Event append implementations enforce invocation ownership
/// and increasing sequence.
pub(crate) trait AgentSessionRepository: Send + Sync {
    fn resolve_working_directory(
        &self,
        _session_id: &AgentSessionId,
        _path: &str,
        _origin: &str,
        _updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        Err(RepositoryError::new(
            RepositoryErrorKind::Unavailable,
            "Working context persistence is unavailable",
        ))
    }
    fn create_session(&self, session: AgentSession) -> Result<AgentSession, RepositoryError>;

    fn get_session(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSession>, RepositoryError>;

    fn list_sessions(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSession>, RepositoryError>;

    /// Loads one complete, consistently ordered session snapshot.
    fn load_session_history(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSessionHistory>, RepositoryError>;

    /// Lists consistently read session summaries using the repository's snapshot boundary.
    fn list_session_summaries(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSessionSummary>, RepositoryError>;

    fn set_session_availability(
        &self,
        session_id: &AgentSessionId,
        availability: AgentSessionAvailability,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError>;

    fn update_runtime_binding(
        &self,
        session_id: &AgentSessionId,
        binding: AgentRuntimeBinding,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError>;

    /// Replaces the exact Harness reference after explicit assignment or Harness-owned migration.
    fn update_harness_version(
        &self,
        session_id: &AgentSessionId,
        harness_version: Option<HarnessVersionRef>,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError>;

    /// Assigns a Session-owned identity snapshot or clears the current assignment.
    fn update_assigned_identity(
        &self,
        session_id: &AgentSessionId,
        assigned_identity: Option<AssignedAgentIdentity>,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError>;

    /// Updates the Session-owned model preference without changing its sandbox selection.
    fn update_session_model_override(
        &self,
        session_id: &AgentSessionId,
        model: Option<String>,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError>;

    fn create_pending_invocation(
        &self,
        invocation: AgentInvocation,
    ) -> Result<AgentInvocation, RepositoryError>;

    fn get_invocation(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<AgentInvocation>, RepositoryError>;

    fn list_invocations(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Vec<AgentInvocation>, RepositoryError>;

    fn mark_invocation_running(
        &self,
        invocation_id: &AgentInvocationId,
        started_at: DateTime<Utc>,
        effective_options: AgentRuntimeOptions,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError>;

    /// Records the durable application fact that the runtime accepted this exact invocation.
    fn record_invocation_launch_accepted(
        &self,
        invocation_id: &AgentInvocationId,
        accepted_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;

    fn invocation_launch_accepted_at(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError>;

    /// Returns only a classified restart interruption to pending. This never reattaches a
    /// process and never accepts a launch marker by inference.
    fn recover_pre_acceptance_interruption(
        &self,
        invocation_id: &AgentInvocationId,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError>;

    fn finish_invocation(
        &self,
        invocation_id: &AgentInvocationId,
        completion: InvocationCompletion,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError>;

    fn append_invocation_diagnostic(
        &self,
        invocation_id: &AgentInvocationId,
        diagnostic: AgentDiagnostic,
    ) -> Result<AgentInvocation, RepositoryError>;

    fn append_event(&self, event: AgentRuntimeEvent) -> Result<AgentRuntimeEvent, RepositoryError>;

    fn list_events(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Vec<AgentRuntimeEvent>, RepositoryError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RepositoryErrorKind {
    NotFound,
    Conflict,
    InvalidState,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryError {
    pub(crate) kind: RepositoryErrorKind,
    pub(crate) message: String,
}

impl RepositoryError {
    pub(crate) fn new(kind: RepositoryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RepositoryError {}
