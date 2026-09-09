use crate::agent_sessions::domain::{
    AgentInvocationId, AgentInvocationTerminalStatus, AgentRuntimeEventSource, AgentRuntimeFailure,
    AgentRuntimeOptions, AgentSessionId, ExternalRuntimeContextId, NormalizedRuntimeEvent,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt, sync::Arc};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeInvocationRequest {
    pub(crate) session_id: AgentSessionId,
    pub(crate) invocation_id: AgentInvocationId,
    pub(crate) submitted_text: String,
    pub(crate) working_directory: Option<String>,
    pub(crate) options: AgentRuntimeOptions,
    /// Prepared native-home binding, capability additions and explicit domain launch selections.
    pub(crate) launch_extension: Option<RuntimeLaunchExtension>,
}

/// Launch data prepared by the application. Authority is established before crossing this port.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RuntimeLaunchExtension {
    /// Product-owned connections added to native configuration by the provider adapter.
    pub(crate) managed_mcp_servers: Vec<RuntimeManagedMcpServer>,
    /// Semantic invocation choice. Only the provider adapter serializes its configuration.
    pub(crate) reasoning_mode: Option<String>,
    /// Explicit existing Harness intent; never inherited by ordinary sessions.
    pub(crate) ignore_user_rules: bool,
    /// Additional native discovery roots for this invocation only.
    pub(crate) skill_roots: Vec<String>,
    /// Codex KEY=TOML_VALUE overrides; this cannot carry process flags.
    pub(crate) config_overrides: Vec<String>,
    pub(crate) environment: Vec<(String, String)>,
    /// Neutral, application-provenance text delivered before the initial user prompt. The
    /// persisted invocation remains the user's submitted text and generic callers leave this absent.
    pub(crate) initial_prompt_prefix: Option<InitialPromptPrefix>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeManagedMcpServer {
    pub(crate) name: String,
    pub(crate) url: String,
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
    fn active_turn(
        &self,
        _invocation_id: &AgentInvocationId,
    ) -> Result<RuntimeTurnTarget, RuntimePortError> {
        Err(RuntimePortError::new(
            RuntimePortErrorKind::NotActive,
            "Runtime has no active steerable turn",
        ))
    }

    fn steer(
        &self,
        _invocation_id: &AgentInvocationId,
        _target: &RuntimeTurnTarget,
        _input_id: &str,
        _text: &str,
    ) -> Result<(), RuntimePortError> {
        Err(RuntimePortError::new(
            RuntimePortErrorKind::UnsupportedOptions,
            "Turn steering is unavailable for this runtime",
        ))
    }

    fn respond(
        &self,
        _invocation_id: &AgentInvocationId,
        _request_id: &str,
        _response: Value,
    ) -> Result<(), RuntimePortError> {
        Err(RuntimePortError::new(
            RuntimePortErrorKind::UnsupportedOptions,
            "Runtime request responses are unavailable",
        ))
    }
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeTurnTarget {
    pub(crate) thread_id: String,
    pub(crate) turn_id: String,
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
