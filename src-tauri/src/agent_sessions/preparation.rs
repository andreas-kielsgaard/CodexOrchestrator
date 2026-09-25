//! Accepted ordinary submissions and durable setup facts.
use super::domain::{AgentInvocationId, AgentRuntimeBinding, AgentSessionId};
use crate::execution_configuration::{
    DirectUserInvocationResolution, SandboxMode, SessionCreationResolution,
};
use crate::execution_targets::domain::{SessionExecutionSelection, SessionExecutionTarget};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionPreparation {
    pub(crate) invocation_id: AgentInvocationId,
    pub(crate) session_id: AgentSessionId,
    /// Set when a changed execution identity created `session_id` as a destination instance.
    /// The source Session and its history are never modified by this preparation.
    #[serde(default)]
    pub(crate) source_session_id: Option<AgentSessionId>,
    pub(crate) phase: PreparationPhase,
    pub(crate) steps: Vec<PreparationStep>,
    pub(crate) error: Option<String>,
    pub(crate) can_retry: bool,
    pub(crate) selection: Option<SessionExecutionSelection>,
    pub(crate) source_target: Option<SessionExecutionTarget>,
    pub(crate) source_binding: AgentRuntimeBinding,
    #[serde(default)]
    pub(crate) prepared_binding: Option<AgentRuntimeBinding>,
    pub(crate) resolved_target: Option<SessionExecutionTarget>,
    pub(crate) resolved_working_directory: Option<String>,
    #[serde(default)]
    pub(crate) accepted_working_directory: Option<String>,
    pub(crate) current_resolution: Option<SessionCreationResolution>,
    pub(crate) resolution: Option<DirectUserInvocationResolution>,
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
