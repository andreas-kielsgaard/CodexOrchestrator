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
pub(crate) struct InvocationManagedMcpSnapshot {
    pub(crate) name: String,
    pub(crate) enabled_tools: Option<Vec<String>>,
    pub(crate) required: bool,
}

/// Immutable execution facts selected for one invocation. Provider credentials and bearer tokens
/// are deliberately excluded.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InvocationExecutionSnapshot {
    pub(crate) target: SessionExecutionTarget,
    pub(crate) working_directory: String,
    pub(crate) session_policy: SessionCreationResolution,
    pub(crate) invocation_resolution: DirectUserInvocationResolution,
    pub(crate) managed_mcp_servers: Vec<InvocationManagedMcpSnapshot>,
    pub(crate) invoked_skill_ids: Vec<String>,
    pub(crate) harness_version: Option<HarnessVersionRef>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionPreparation {
    pub(crate) invocation_id: AgentInvocationId,
    pub(crate) session_id: AgentSessionId,
    pub(crate) phase: PreparationPhase,
    pub(crate) steps: Vec<PreparationStep>,
    pub(crate) error: Option<String>,
    pub(crate) can_retry: bool,
    pub(crate) selection: Option<SessionExecutionSelection>,
    pub(crate) source_target: Option<SessionExecutionTarget>,
    pub(crate) source_binding: AgentRuntimeBinding,
    #[serde(default)]
    pub(crate) prepared_binding: Option<AgentRuntimeBinding>,
    /// The current provider session retained when this preparation selects another route.
    #[serde(default, alias = "parkedSource")]
    pub(crate) source_provider_binding: Option<super::domain::ProviderSessionBinding>,
    pub(crate) resolved_target: Option<SessionExecutionTarget>,
    pub(crate) resolved_working_directory: Option<String>,
    #[serde(default)]
    pub(crate) accepted_working_directory: Option<String>,
    #[serde(default)]
    pub(crate) context: Vec<InvocationContextPart>,
    pub(crate) current_resolution: Option<SessionCreationResolution>,
    pub(crate) resolution: Option<DirectUserInvocationResolution>,
    #[serde(default)]
    pub(crate) execution_snapshot: Option<InvocationExecutionSnapshot>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_mode: Option<String>,
    pub(crate) sandbox_mode: Option<SandboxMode>,
    pub(crate) delivery_started: bool,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PreparationPhase {
    Accepted,
    Preparing,
    Ready,
    Failed,
    Canceled,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparationStep {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) status: PreparationStepStatus,
    pub(crate) error: Option<String>,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PreparationStepStatus {
    Pending,
    Running,
    Completed,
    Failed,
}
