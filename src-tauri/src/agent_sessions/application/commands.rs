//! Session commands, receipts and application errors.

use crate::agent_sessions::domain::{AgentInvocationId, AgentRuntimeOptions, AgentSessionId};
use crate::agent_sessions::ports::{AgentSessionSummary, RepositoryError, RuntimePortError};
use crate::execution_configuration::SessionCreationResolution;
use crate::harness_engine::domain::HarnessVersionRef;
use crate::identities::AssignedAgentIdentity;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug)]
pub(crate) struct CreateAgentSessionCommand {
    pub(crate) title: Option<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) requested_options: AgentRuntimeOptions,
}

#[derive(Clone, Debug)]
pub(crate) struct CreateApplicationAgentSessionCommand {
    pub(crate) session_id: AgentSessionId,
    pub(crate) session: CreateAgentSessionCommand,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct AgentSessionOwnership {
    pub(crate) execution_target: Option<crate::execution_targets::domain::SessionExecutionTarget>,
    pub(crate) harness_version: Option<HarnessVersionRef>,
    pub(crate) assigned_identity: Option<AssignedAgentIdentity>,
    pub(crate) session_profile: Option<SessionCreationResolution>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UpdateAgentSessionHarnessCommand {
    pub(crate) session_id: AgentSessionId,
    pub(crate) harness_version: Option<HarnessVersionRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UpdateAgentSessionIdentityCommand {
    pub(crate) session_id: AgentSessionId,
    pub(crate) assigned_identity: Option<AssignedAgentIdentity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UpdateAgentSessionModelOverrideCommand {
    pub(crate) session_id: AgentSessionId,
    pub(crate) model: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct SendAgentSessionMessageCommand {
    pub(crate) session_id: Option<AgentSessionId>,
    pub(crate) submitted_text: String,
    pub(crate) title: Option<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) requested_options: Option<AgentRuntimeOptions>,
}

#[derive(Clone, Debug)]
pub(crate) struct SendIdempotentApplicationAgentSessionMessageCommand {
    pub(crate) invocation_id: AgentInvocationId,
    pub(crate) message: SendAgentSessionMessageCommand,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SendAgentSessionMessageResult {
    pub(crate) session_id: AgentSessionId,
    pub(crate) invocation_id: AgentInvocationId,
}

pub(crate) struct SendAgentSessionMessageLaunchResult {
    pub(crate) acknowledgement: SendAgentSessionMessageResult,
    pub(crate) launch_accepted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApplicationInvocationLaunchEvidence {
    NeverPersisted,
    PersistedNotAccepted,
    LaunchAccepted,
}

#[derive(Clone, Debug)]
pub(crate) struct CancelAgentInvocationCommand {
    pub(crate) invocation_id: AgentInvocationId,
}

pub(crate) type ListAgentSessionsResult = Vec<AgentSessionSummary>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AgentSessionApplicationErrorKind {
    InvalidInput,
    NotFound,
    Conflict,
    Repository,
    Runtime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AgentSessionApplicationError {
    pub(crate) kind: AgentSessionApplicationErrorKind,
    pub(crate) message: String,
}

impl AgentSessionApplicationError {
    pub(super) fn invalid(message: impl Into<String>) -> Self {
        Self::new(AgentSessionApplicationErrorKind::InvalidInput, message)
    }

    pub(super) fn not_found(message: impl Into<String>) -> Self {
        Self::new(AgentSessionApplicationErrorKind::NotFound, message)
    }

    pub(super) fn conflict(message: impl Into<String>) -> Self {
        Self::new(AgentSessionApplicationErrorKind::Conflict, message)
    }

    pub(super) fn repository(error: RepositoryError) -> Self {
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
