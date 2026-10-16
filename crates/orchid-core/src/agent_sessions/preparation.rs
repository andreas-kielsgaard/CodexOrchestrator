//! Accepted ordinary submissions and durable setup facts.
use super::domain::{AgentInvocationId, AgentRuntimeBinding, AgentSessionId};
use crate::execution_configuration::{
    DirectUserInvocationResolution, SandboxMode, SessionCreationResolution,
};
use crate::execution_targets::domain::{SessionExecutionSelection, SessionExecutionTarget};
use crate::harness_engine::domain::HarnessVersionRef;
use orchid_engine::contracts::InvocationContextPart;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationManagedMcpSnapshot {
    pub name: String,
    pub enabled_tools: Option<Vec<String>>,
    pub required: bool,
}

/// Immutable execution facts selected for one invocation. Provider credentials and bearer tokens
/// are deliberately excluded.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationExecutionSnapshot {
    pub target: SessionExecutionTarget,
    pub working_directory: String,
    pub session_policy: SessionCreationResolution,
    pub invocation_resolution: DirectUserInvocationResolution,
    pub managed_mcp_servers: Vec<InvocationManagedMcpSnapshot>,
    pub invoked_skill_ids: Vec<String>,
    pub harness_version: Option<HarnessVersionRef>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedCapabilityProfile {
    pub id: String,
    pub revision: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPreparation {
    #[serde(default)]
    pub submission_fingerprint: Option<String>,
    /// Product delivery retains the Session's pinned policy and existing provider route.
    #[serde(default)]
    pub pinned_delivery: bool,
    pub invocation_id: AgentInvocationId,
    pub session_id: AgentSessionId,
    pub phase: PreparationPhase,
    pub steps: Vec<PreparationStep>,
    pub error: Option<String>,
    pub can_retry: bool,
    pub selection: Option<SessionExecutionSelection>,
    #[serde(default)]
    pub requested_capability_profile: Option<QueuedCapabilityProfile>,
    pub source_target: Option<SessionExecutionTarget>,
    pub source_binding: AgentRuntimeBinding,
    #[serde(default)]
    pub prepared_binding: Option<AgentRuntimeBinding>,
    /// The current provider session retained when this preparation selects another route.
    #[serde(default, alias = "parkedSource")]
    pub source_provider_binding: Option<super::domain::ProviderSessionBinding>,
    pub resolved_target: Option<SessionExecutionTarget>,
    pub resolved_working_directory: Option<String>,
    #[serde(default)]
    pub accepted_working_directory: Option<String>,
    #[serde(default)]
    pub context: Vec<InvocationContextPart>,
    pub current_resolution: Option<SessionCreationResolution>,
    pub resolution: Option<DirectUserInvocationResolution>,
    #[serde(default)]
    pub execution_snapshot: Option<InvocationExecutionSnapshot>,
    pub model: Option<String>,
    pub reasoning_mode: Option<String>,
    pub sandbox_mode: Option<SandboxMode>,
    pub delivery_started: bool,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparationPhase {
    Accepted,
    Preparing,
    Ready,
    Failed,
    Canceled,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreparationStep {
    pub id: String,
    pub label: String,
    pub status: PreparationStepStatus,
    pub error: Option<String>,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PreparationStepStatus {
    Pending,
    Running,
    Completed,
    Failed,
}
