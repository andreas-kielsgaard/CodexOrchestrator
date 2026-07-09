use super::domain::{
    AgentDiagnostic, AgentInvocation, AgentInvocationId, AgentInvocationTerminalStatus,
    AgentRuntimeBinding, AgentRuntimeEvent, AgentRuntimeEventSource, AgentRuntimeFailure,
    AgentRuntimeOptions, AgentSession, AgentSessionAvailability, AgentSessionId,
    ExternalRuntimeContextId, InvocationCompletion, NormalizedRuntimeEvent,
};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::{error::Error, fmt, sync::Arc};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ListAgentSessionsQuery {
    pub(crate) availability: Option<AgentSessionAvailability>,
    pub(crate) limit: Option<u32>,
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
    fn create_session(&self, session: AgentSession) -> Result<AgentSession, RepositoryError>;

    fn get_session(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSession>, RepositoryError>;

    fn list_sessions(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSession>, RepositoryError>;

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeInvocationRequest {
    pub(crate) session_id: AgentSessionId,
    pub(crate) invocation_id: AgentInvocationId,
    pub(crate) submitted_text: String,
    pub(crate) working_directory: Option<String>,
    pub(crate) options: AgentRuntimeOptions,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RuntimeEventDraft {
    pub(crate) source: AgentRuntimeEventSource,
    pub(crate) raw_payload: Value,
    pub(crate) normalized: Option<NormalizedRuntimeEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RuntimeInvocationOutcome {
    pub(crate) status: AgentInvocationTerminalStatus,
    pub(crate) exit_code: Option<i32>,
    pub(crate) signal: Option<String>,
    pub(crate) runtime_error: Option<AgentRuntimeFailure>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum RuntimeUpdate {
    Event(RuntimeEventDraft),
    Finished(RuntimeInvocationOutcome),
}

/// Receives provider-neutral event drafts in runtime-observed order and one terminal outcome.
///
/// The application layer assigns durable event IDs, sequence numbers, and timestamps before
/// repository append. Returning an error reports failed delivery without reclassifying the
/// runtime's eventual terminal outcome.
pub(crate) trait AgentRuntimeUpdateSink: Send + Sync {
    fn emit_update(
        &self,
        invocation_id: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError>;
}

/// Runtime operations proven necessary by the first Agent Session slice.
///
/// Start and resume are separate so callers cannot accidentally use the local session ID as the
/// external continuation identity. Implementations may return after launch and continue emitting
/// through the supplied sink. Process arguments, JSONL types, and child handles are adapter
/// concerns and do not cross this boundary.
pub(crate) trait AgentRuntime: Send + Sync {
    fn start_invocation(
        &self,
        request: RuntimeInvocationRequest,
        update_sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError>;

    fn resume_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external_context_id: ExternalRuntimeContextId,
        update_sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError>;

    fn cancel_invocation(&self, invocation_id: &AgentInvocationId) -> Result<(), RuntimePortError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimePortErrorKind {
    UnsupportedOptions,
    AlreadyActive,
    NotActive,
    LaunchFailed,
    EventDeliveryFailed,
    CancellationFailed,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RuntimePortError {
    pub(crate) kind: RuntimePortErrorKind,
    pub(crate) message: String,
    pub(crate) details: Option<Value>,
}

impl RuntimePortError {
    pub(crate) fn new(kind: RuntimePortErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
        }
    }

    pub(crate) fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl fmt::Display for RuntimePortError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for RuntimePortError {}
