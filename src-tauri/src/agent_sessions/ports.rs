use super::domain::{
    AgentDiagnostic, AgentInvocation, AgentInvocationId, AgentInvocationTerminalStatus,
    AgentRuntimeBinding, AgentRuntimeEvent, AgentRuntimeEventSource, AgentRuntimeFailure,
    AgentRuntimeOptions, AgentSession, AgentSessionAvailability, AgentSessionId,
    ExternalRuntimeContextId, InvocationCompletion, NormalizedRuntimeEvent,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt, sync::Arc};

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
    pub(crate) session: AgentSession,
    pub(crate) invocation_count: u64,
    pub(crate) latest_invocation_status: Option<super::domain::AgentInvocationStatus>,
    pub(crate) latest_submitted_text: Option<String>,
}

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
    /// Opt-in child-process configuration supplied by a role-specific application service.
    /// Ordinary Agent Session sends always leave this absent.
    pub(crate) launch_extension: Option<RuntimeLaunchExtension>,
}

/// Concrete-runtime launch data. This carries no session role, product identity, or authority;
/// an application service must explicitly opt into it for one invocation.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RuntimeLaunchExtension {
    pub(crate) additional_args: Vec<String>,
    pub(crate) environment: Vec<(String, String)>,
    /// Neutral, application-provenance text delivered before the initial user prompt. The
    /// persisted invocation remains the user's submitted text and generic callers leave this absent.
    pub(crate) initial_prompt_prefix: Option<InitialPromptPrefix>,
}

/// Explicit, non-user provenance delivered before an initial user query.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InitialPromptPrefix {
    pub(crate) source: String,
    pub(crate) version: u16,
    pub(crate) content: String,
}

impl InitialPromptPrefix {
    pub(crate) fn render_before_user_query(&self, user_query: &str) -> String {
        format!(
            "<application_context provenance=\"product_initial_prompt_prefix\" source=\"{}\" version=\"{}\">\n{}\n</application_context>\n\n<user_query>\n{}\n</user_query>",
            self.source, self.version, self.content, user_query
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeInvocationMode {
    Start,
    Resume,
}

/// Semantic support states exposed by agent access adapters.
///
/// `Unknown` covers both undiscovered support and a discovery result that could not establish a
/// reliable answer. Callers must never treat it as supported.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilitySupport {
    Supported,
    Unsupported,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InvocationCapabilities {
    pub(crate) structured_events: CapabilitySupport,
    pub(crate) model_selection: CapabilitySupport,
    pub(crate) sandbox_selection: CapabilitySupport,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentAccessCapabilities {
    pub(crate) start: InvocationCapabilities,
    pub(crate) resume: InvocationCapabilities,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityDiscoveryState {
    Observed,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CapabilityProvenance {
    /// Adapter-owned discovery mechanism, for example a CLI help probe.
    pub(crate) source: String,
    pub(crate) runtime_version: Option<String>,
}

/// Cacheable semantic capability evidence returned by an agent access adapter.
///
/// `valid_until` is adapter policy, not a promise that an external runtime cannot change sooner.
/// An unavailable discovery is represented by unknown capabilities plus `Unavailable`, retaining
/// provenance and a diagnostic message without promoting absence of evidence to unsupported.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentAccessCapabilitySnapshot {
    pub(crate) capabilities: AgentAccessCapabilities,
    pub(crate) discovery_state: CapabilityDiscoveryState,
    pub(crate) provenance: CapabilityProvenance,
    pub(crate) observed_at: DateTime<Utc>,
    pub(crate) valid_until: DateTime<Utc>,
    pub(crate) unavailable_reason: Option<String>,
}

impl AgentAccessCapabilitySnapshot {
    pub(crate) fn is_fresh_at(&self, now: DateTime<Utc>) -> bool {
        now < self.valid_until
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum CapabilityRefresh {
    #[default]
    UseFreshCache,
    Refresh,
}

/// Adapter-owned discovery supplies semantic evidence; infrastructure owns cache reuse.
pub(crate) trait AgentAccessCapabilityDiscovery: Send + Sync {
    fn discover_capabilities(&self, observed_at: DateTime<Utc>) -> AgentAccessCapabilitySnapshot;
}

/// Confirms which semantic options preflight determined will be applied at launch.
///
/// Absent values remain unknown and must not be filled with provider defaults by the caller.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeInvocationPreflight {
    pub(crate) effective_options: AgentRuntimeOptions,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeEventDraft {
    pub(crate) source: AgentRuntimeEventSource,
    pub(crate) raw_payload: Value,
    pub(crate) normalized: Option<NormalizedRuntimeEvent>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeInvocationOutcome {
    pub(crate) status: AgentInvocationTerminalStatus,
    pub(crate) exit_code: Option<i32>,
    pub(crate) signal: Option<String>,
    pub(crate) runtime_error: Option<AgentRuntimeFailure>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub(crate) enum RuntimeUpdate {
    Event(RuntimeEventDraft),
    Finished(RuntimeInvocationOutcome),
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeUpdateDeliveryFailure {
    pub(crate) update: RuntimeUpdate,
    pub(crate) error: RuntimePortError,
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

    /// Observes one failed `emit_update` attempt through a bounded, non-recursive fallback.
    ///
    /// Implementations must not retry the failed runtime update from this callback. The runtime
    /// calls it at most once per failed attempt and preserves the actual runtime terminal result.
    fn report_delivery_failure(
        &self,
        invocation_id: &AgentInvocationId,
        failure: RuntimeUpdateDeliveryFailure,
    );
}

/// Runtime operations proven necessary by the first Agent Session slice.
///
/// Callers must execute `preflight_invocation`, durably transition the invocation from pending to
/// running using the returned effective options, and only then call `start_invocation` or
/// `resume_invocation`. A child may emit updates before either launch method returns.
///
/// Start and resume are separate so callers cannot accidentally use the local session ID as the
/// external continuation identity. Implementations may return after launch and continue emitting
/// through the supplied sink. Process arguments, JSONL types, and child handles are adapter
/// concerns and do not cross this boundary. If either launch method returns an error after also
/// producing a terminal update, that terminal update must be delivered synchronously before the
/// error is returned, and no later updates may follow that error return. The application then
/// checks durable invocation state: an already-terminal invocation is left unchanged, while a
/// still-active invocation is durably failed from the returned launch error.
pub(crate) trait AgentRuntime: Send + Sync {
    fn preflight_invocation(
        &self,
        mode: RuntimeInvocationMode,
        requested_options: &AgentRuntimeOptions,
    ) -> Result<RuntimeInvocationPreflight, RuntimePortError>;

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

    /// Stops runtime-owned processes during application shutdown. Implementations without
    /// process ownership may keep the default no-op behavior.
    fn shutdown(&self) -> Result<(), RuntimePortError> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimePortErrorKind {
    UnsupportedOptions,
    AlreadyActive,
    NotActive,
    LaunchFailed,
    EventDeliveryFailed,
    CancellationFailed,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
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
