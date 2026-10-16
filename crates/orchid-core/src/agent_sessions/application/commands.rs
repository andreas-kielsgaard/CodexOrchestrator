//! Session commands, receipts and application errors.

use crate::agent_sessions::domain::{AgentInvocationId, AgentRuntimeOptions, AgentSessionId};
use crate::agent_sessions::ports::{AgentSessionSummary, RepositoryError, RuntimePortError};
use crate::execution_configuration::SessionCreationResolution;
use crate::harness_engine::domain::HarnessVersionRef;
use crate::identities::AssignedAgentIdentity;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug)]
pub struct CreateAgentSessionCommand {
    pub title: Option<String>,
    pub working_directory: Option<String>,
    pub requested_options: AgentRuntimeOptions,
}

#[derive(Clone, Debug)]
pub struct CreateApplicationAgentSessionCommand {
    pub session_id: AgentSessionId,
    pub session: CreateAgentSessionCommand,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AgentSessionOwnership {
    pub execution_target: Option<crate::execution_targets::domain::SessionExecutionTarget>,
    pub harness_version: Option<HarnessVersionRef>,
    pub assigned_identity: Option<AssignedAgentIdentity>,
    pub session_profile: Option<SessionCreationResolution>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateAgentSessionHarnessCommand {
    pub session_id: AgentSessionId,
    pub harness_version: Option<HarnessVersionRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateAgentSessionIdentityCommand {
    pub session_id: AgentSessionId,
    pub assigned_identity: Option<AssignedAgentIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RenameAgentSessionCommand {
    pub session_id: AgentSessionId,
    pub title: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateAgentSessionModelOverrideCommand {
    pub session_id: AgentSessionId,
    pub model: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SendAgentSessionMessageCommand {
    pub session_id: Option<AgentSessionId>,
    pub submitted_text: String,
    pub attachments: Vec<crate::agent_sessions::domain::RuntimeImageAttachment>,
    pub title: Option<String>,
    pub working_directory: Option<String>,
    pub requested_options: Option<AgentRuntimeOptions>,
}

#[derive(Clone, Debug)]
pub struct SendIdempotentApplicationAgentSessionMessageCommand {
    pub invocation_id: AgentInvocationId,
    pub message: SendAgentSessionMessageCommand,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAgentSessionMessageResult {
    pub session_id: AgentSessionId,
    pub invocation_id: AgentInvocationId,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendAgentSessionMessageLaunchResult {
    pub acknowledgement: SendAgentSessionMessageResult,
    pub launch_accepted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApplicationInvocationLaunchEvidence {
    NeverPersisted,
    PersistedNotAccepted,
    LaunchAccepted,
}

#[derive(Clone, Debug)]
pub struct CancelAgentInvocationCommand {
    pub invocation_id: AgentInvocationId,
}

pub type ListAgentSessionsResult = Vec<AgentSessionSummary>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentSessionApplicationErrorKind {
    InvalidInput,
    NotFound,
    Conflict,
    Repository,
    Runtime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentSessionApplicationError {
    pub kind: AgentSessionApplicationErrorKind,
    pub message: String,
}

impl AgentSessionApplicationError {
    pub(super) fn invalid(message: impl Into<String>) -> Self {
        Self::new(AgentSessionApplicationErrorKind::InvalidInput, message)
    }

    pub(in crate::agent_sessions) fn not_found(message: impl Into<String>) -> Self {
        Self::new(AgentSessionApplicationErrorKind::NotFound, message)
    }

    pub(super) fn conflict(message: impl Into<String>) -> Self {
        Self::new(AgentSessionApplicationErrorKind::Conflict, message)
    }

    pub(in crate::agent_sessions) fn repository(error: RepositoryError) -> Self {
        Self::new(AgentSessionApplicationErrorKind::Repository, error.message)
    }

    pub(super) fn runtime(error: RuntimePortError) -> Self {
        Self::new(AgentSessionApplicationErrorKind::Runtime, error.message)
    }

    pub(super) fn new(kind: AgentSessionApplicationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for AgentSessionApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AgentSessionApplicationError {}
