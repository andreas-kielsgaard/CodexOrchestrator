use crate::contracts::domain::{
    AgentInvocationId, AgentInvocationTerminalStatus, AgentRuntimeEventSource, AgentRuntimeFailure,
    AgentRuntimeOptions, AgentSessionId, ExternalRuntimeContextId, NormalizedRuntimeEvent,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt, sync::Arc};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInvocationRequest {
    pub session_id: AgentSessionId,
    pub invocation_id: AgentInvocationId,
    pub submitted_text: String,
    pub working_directory: Option<String>,
    pub options: AgentRuntimeOptions,
    /// Prepared native-home binding, capability additions and explicit domain launch selections.
    pub launch_extension: Option<RuntimeLaunchExtension>,
}

/// Launch data prepared by the application. Authority is established before crossing this port.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLaunchExtension {
    /// Product-owned connections added to native configuration by the provider adapter.
    pub managed_mcp_servers: Vec<RuntimeManagedMcpServer>,
    /// Semantic invocation choice. Only the provider adapter serializes its configuration.
    pub reasoning_mode: Option<String>,
    /// Opaque provider-native settings. Only the provider named by the envelope may decode them.
    pub provider_options: Option<crate::contracts::provider::ProviderNativeOptions>,
    /// Explicit existing Harness intent; never inherited by ordinary sessions.
    pub ignore_user_rules: bool,
    /// Pinned skills available to the session reader. Mentioned native skills may also become
    /// explicit turn inputs; the manifest itself does not invoke them.
    pub skill_inputs: Vec<RuntimeSkillInput>,
    /// Whether native Codex-configured MCP servers are exposed for this pinned session.
    pub native_mcp_enabled: Option<bool>,
    /// Codex KEY=TOML_VALUE overrides; this cannot carry process flags.
    pub config_overrides: Vec<String>,
    pub environment: Vec<(String, String)>,
    /// Neutral, application-provenance text delivered before the initial user prompt. The
    /// persisted invocation remains the user's submitted text and generic callers leave this absent.
    pub initial_prompt_prefix: Option<InitialPromptPrefix>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeManagedMcpServer {
    pub name: String,
    pub url: String,
}

/// One immutable skill selection from Orchid's session capability manifest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSkillInput {
    pub id: String,
    pub name: String,
    pub path: String,
    pub content_sha256: String,
    #[serde(default)]
    pub description: String,
}

/// Explicit, non-user provenance delivered before an initial user query.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitialPromptPrefix {
    pub source: String,
    pub version: u16,
    pub content: String,
}

impl InitialPromptPrefix {
    pub fn render_before_user_query(&self, user_query: &str) -> String {
        format!(
            "<application_context provenance=\"product_initial_prompt_prefix\" source=\"{}\" version=\"{}\">\n{}\n</application_context>\n\n<user_query>\n{}\n</user_query>",
            self.source, self.version, self.content, user_query
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeInvocationMode {
    Start,
    Resume,
}

/// Semantic support states exposed by agent access adapters.
///
/// `Unknown` covers both undiscovered support and a discovery result that could not establish a
/// reliable answer. Callers must never treat it as supported.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilitySupport {
    Supported,
    Unsupported,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationCapabilities {
    pub structured_events: CapabilitySupport,
    pub model_selection: CapabilitySupport,
    pub sandbox_selection: CapabilitySupport,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAccessCapabilities {
    pub start: InvocationCapabilities,
    pub resume: InvocationCapabilities,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityDiscoveryState {
    Observed,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityProvenance {
    /// Adapter-owned discovery mechanism, for example a CLI help probe.
    pub source: String,
    pub runtime_version: Option<String>,
}

/// Cacheable semantic capability evidence returned by an agent access adapter.
///
/// `valid_until` is adapter policy, not a promise that an external runtime cannot change sooner.
/// An unavailable discovery is represented by unknown capabilities plus `Unavailable`, retaining
/// provenance and a diagnostic message without promoting absence of evidence to unsupported.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentAccessCapabilitySnapshot {
    pub capabilities: AgentAccessCapabilities,
    pub discovery_state: CapabilityDiscoveryState,
    pub provenance: CapabilityProvenance,
    pub observed_at: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
    pub unavailable_reason: Option<String>,
}

impl AgentAccessCapabilitySnapshot {
    pub fn is_fresh_at(&self, now: DateTime<Utc>) -> bool {
        now < self.valid_until
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub enum CapabilityRefresh {
    #[default]
    UseFreshCache,
    Refresh,
}

/// Adapter-owned discovery supplies semantic evidence; infrastructure owns cache reuse.
pub trait AgentAccessCapabilityDiscovery: Send + Sync {
    fn discover_capabilities(&self, observed_at: DateTime<Utc>) -> AgentAccessCapabilitySnapshot;
}

/// Confirms which semantic options preflight determined will be applied at launch.
///
/// Absent values remain unknown and must not be filled with provider defaults by the caller.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInvocationPreflight {
    pub effective_options: AgentRuntimeOptions,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEventDraft {
    pub source: AgentRuntimeEventSource,
    pub raw_payload: Value,
    pub normalized: Option<NormalizedRuntimeEvent>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInvocationOutcome {
    pub status: AgentInvocationTerminalStatus,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    pub runtime_error: Option<AgentRuntimeFailure>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
pub enum RuntimeUpdate {
    Event(RuntimeEventDraft),
    Finished(RuntimeInvocationOutcome),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeUpdateDeliveryFailure {
    pub update: RuntimeUpdate,
    pub error: RuntimePortError,
}

/// Receives provider-neutral event drafts in runtime-observed order and one terminal outcome.
///
/// The application layer assigns durable event IDs, sequence numbers, and timestamps before
/// repository append. Returning an error reports failed delivery without reclassifying the
/// runtime's eventual terminal outcome.
pub trait AgentRuntimeUpdateSink: Send + Sync {
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInvocationReady {
    pub external_context_id: ExternalRuntimeContextId,
    pub working_directory: String,
}

pub trait AgentRuntime: Send + Sync {
    /// Resolve native continuation on a retained connection without delivering the prompt.
    fn prepare_invocation(
        &self,
        _request: RuntimeInvocationRequest,
        _external_context_id: Option<ExternalRuntimeContextId>,
        _sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<RuntimeInvocationReady, RuntimePortError> {
        Err(RuntimePortError::new(
            RuntimePortErrorKind::UnsupportedOptions,
            "Runtime preparation is unavailable",
        ))
    }

    fn deliver_prepared_invocation(&self, _id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        Err(RuntimePortError::new(
            RuntimePortErrorKind::NotActive,
            "No prepared invocation",
        ))
    }

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
        _response: crate::contracts::interactions::RuntimeInteractionResponse,
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
pub struct RuntimeTurnTarget {
    pub thread_id: String,
    pub turn_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimePortErrorKind {
    UnsupportedOptions,
    AlreadyActive,
    NotActive,
    LaunchFailed,
    EventDeliveryFailed,
    CancellationFailed,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePortError {
    pub kind: RuntimePortErrorKind,
    pub message: String,
    pub details: Option<Value>,
}

impl RuntimePortError {
    pub fn new(kind: RuntimePortErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
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
