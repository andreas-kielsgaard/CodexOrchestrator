use crate::{
    execution_configuration::SessionCreationResolution, harness_engine::domain::HarnessVersionRef,
    identities::AssignedAgentIdentity,
};
use chrono::{DateTime, Utc};
pub use orchid_engine::contracts::domain::*;
pub use orchid_engine::contracts::RuntimeImageAttachment;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSessionAvailability {
    Available,
    Archived,
}

/// The native conversation of the provider the Session currently runs on.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRuntimeBinding {
    pub external_context_id: Option<ExternalRuntimeContextId>,
    pub runtime_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderBindingOwner {
    pub execution: crate::execution_targets::domain::ExecutionBinding,
    pub workspace: Option<ProviderWorkspaceIdentity>,
}
impl From<&crate::execution_targets::domain::SessionExecutionTarget> for ProviderBindingOwner {
    fn from(target: &crate::execution_targets::domain::SessionExecutionTarget) -> Self {
        Self {
            execution: target.execution.clone(),
            workspace: Some(ProviderWorkspaceIdentity::from(target)),
        }
    }
}

/// One provider-owned conversation cached at its exact execution route. Orchid may resume this
/// instance later, but never moves its native handle to another provider, configuration, or device.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSessionBinding {
    pub external_context_id: ExternalRuntimeContextId,
    pub runtime_version: Option<String>,
    /// The immutable route on which the provider owns this conversation.
    pub location: crate::execution_targets::domain::ExecutionBinding,
    /// The physical workspace in which the provider conversation was created. Legacy bindings
    /// without this identity are never used as an exact destination cache.
    #[serde(default)]
    pub workspace: Option<ProviderWorkspaceIdentity>,
    /// The last invocation the conversation took part in.
    pub last_invocation_id: Option<AgentInvocationId>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderWorkspaceIdentity {
    pub repository_id: String,
    pub worktree_id: String,
    pub path: String,
}

impl From<&crate::execution_targets::domain::SessionExecutionTarget> for ProviderWorkspaceIdentity {
    fn from(target: &crate::execution_targets::domain::SessionExecutionTarget) -> Self {
        Self {
            repository_id: target.repository_id.clone(),
            worktree_id: target.worktree_id.clone(),
            path: target.path.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSession {
    #[serde(default)]
    pub execution_target: Option<crate::execution_targets::domain::SessionExecutionTarget>,
    #[serde(default)]
    pub workspace_origin: Option<String>,
    pub id: AgentSessionId,
    pub title: String,
    pub availability: AgentSessionAvailability,
    pub runtime_binding: AgentRuntimeBinding,
    pub working_directory: Option<String>,
    pub requested_options: AgentRuntimeOptions,
    /// Immutable execution configuration pinned by the replacement Session Event path at
    /// creation. Legacy Sessions may remain unprofiled until their concepts are retired.
    pub session_profile: Option<SessionCreationResolution>,
    /// Exact reusable or Session-specific Harness version currently owned by this Session.
    pub harness_version: Option<HarnessVersionRef>,
    /// Session-owned snapshot; it is independent of later Harness or identity-definition edits.
    pub assigned_identity: Option<AssignedAgentIdentity>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentInvocationInputProvenance {
    User,
    Application,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentDiagnosticSource {
    Repository,
    Runtime,
    Transport,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDiagnostic {
    pub source: AgentDiagnosticSource,
    pub severity: AgentDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInvocation {
    pub id: AgentInvocationId,
    pub session_id: AgentSessionId,
    pub submitted_text: String,
    #[serde(default)]
    pub attachments: Vec<RuntimeImageAttachment>,
    pub input_provenance: AgentInvocationInputProvenance,
    pub status: AgentInvocationStatus,
    pub requested_options: AgentRuntimeOptions,
    pub effective_options: Option<AgentRuntimeOptions>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    pub runtime_error: Option<AgentRuntimeFailure>,
    pub diagnostics: Vec<AgentDiagnostic>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvocationCompletion {
    pub status: AgentInvocationTerminalStatus,
    pub completed_at: DateTime<Utc>,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    pub runtime_error: Option<AgentRuntimeFailure>,
}

impl AgentInvocation {
    pub fn mark_running(
        &self,
        started_at: DateTime<Utc>,
        effective_options: AgentRuntimeOptions,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, ContractViolation> {
        validate_invocation_status_transition(self.status, AgentInvocationStatus::Running)?;

        let mut next = self.clone();
        next.status = AgentInvocationStatus::Running;
        next.effective_options = Some(effective_options);
        next.started_at = Some(started_at);
        next.updated_at = updated_at;
        validate_invocation(&next)?;
        Ok(next)
    }

    pub fn finish(
        &self,
        completion: InvocationCompletion,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, ContractViolation> {
        let next_status = AgentInvocationStatus::from(completion.status);
        validate_invocation_status_transition(self.status, next_status)?;

        let mut next = self.clone();
        next.status = next_status;
        next.completed_at = Some(completion.completed_at);
        next.exit_code = completion.exit_code;
        next.signal = completion.signal;
        next.runtime_error = completion.runtime_error;
        next.updated_at = updated_at;
        validate_invocation(&next)?;
        Ok(next)
    }

    /// A restart may prove that no in-process runtime owner survived, while the durable launch
    /// acceptance marker is still absent. Only that classified interruption can return to the
    /// pre-launch state for an application-owned recovery of this exact invocation.
    pub fn recover_pre_acceptance_interruption(
        &self,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, ContractViolation> {
        let recoverable = self.status == AgentInvocationStatus::Interrupted
            && self
                .runtime_error
                .as_ref()
                .is_some_and(|error| error.code == "runtime_startup_without_launch_acceptance");
        if !recoverable {
            return Err(ContractViolation::InvalidInvocationTransition {
                from: self.status,
                to: AgentInvocationStatus::Pending,
            });
        }
        let mut next = self.clone();
        next.status = AgentInvocationStatus::Pending;
        next.effective_options = None;
        next.started_at = None;
        next.completed_at = None;
        next.exit_code = None;
        next.signal = None;
        next.runtime_error = None;
        next.updated_at = updated_at;
        validate_invocation(&next)?;
        Ok(next)
    }
}

/// Provider-neutral semantic detail for a tool item. The enclosing runtime event retains the
/// provider payload for audit; consumers use these fields without parsing it.

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRuntimeEvent {
    pub id: AgentRuntimeEventId,
    pub invocation_id: AgentInvocationId,
    pub sequence: u64,
    pub source: AgentRuntimeEventSource,
    pub raw_payload: Value,
    pub normalized: Option<NormalizedRuntimeEvent>,
    pub recorded_at: DateTime<Utc>,
}

pub fn validate_session_update(
    current: &AgentSession,
    candidate: &AgentSession,
) -> Result<(), ContractViolation> {
    validate_session(candidate)?;
    if current.id != candidate.id {
        return Err(ContractViolation::SessionIdentityChanged);
    }
    if current.session_profile != candidate.session_profile {
        return Err(ContractViolation::SessionProfileChanged);
    }
    validate_runtime_binding_update(&current.runtime_binding, &candidate.runtime_binding)
}

pub fn validate_session(session: &AgentSession) -> Result<(), ContractViolation> {
    if session.title.trim().is_empty() {
        return Err(ContractViolation::InvalidSessionRecord {
            reason: "session title cannot be blank",
        });
    }
    if session
        .working_directory
        .as_ref()
        .is_some_and(|working_directory| working_directory.trim().is_empty())
    {
        return Err(ContractViolation::InvalidSessionRecord {
            reason: "session working directory cannot be blank when present",
        });
    }
    if session.updated_at < session.created_at {
        return Err(ContractViolation::InvalidSessionRecord {
            reason: "session update time cannot precede creation time",
        });
    }
    if session
        .assigned_identity
        .as_ref()
        .is_some_and(|identity| identity.validate().is_err())
    {
        return Err(ContractViolation::InvalidSessionRecord {
            reason: "assigned Agent identity is invalid",
        });
    }
    if let Some(session_profile) = &session.session_profile {
        if session_profile.verify_digest().is_err() {
            return Err(ContractViolation::InvalidSessionRecord {
                reason: "pinned Session Profile failed integrity validation",
            });
        }
    }
    Ok(())
}

pub fn validate_runtime_binding_update(
    current: &AgentRuntimeBinding,
    candidate: &AgentRuntimeBinding,
) -> Result<(), ContractViolation> {
    if let Some(current_external_id) = current.external_context_id.as_ref() {
        if candidate.external_context_id.as_ref() != Some(current_external_id) {
            return Err(ContractViolation::ExternalRuntimeContextChanged);
        }
    }

    Ok(())
}

pub fn validate_new_invocation(
    session: &AgentSession,
    active_invocation: Option<&AgentInvocation>,
    candidate: &AgentInvocation,
) -> Result<(), ContractViolation> {
    validate_session(session)?;
    if candidate.session_id != session.id {
        return Err(ContractViolation::InvocationSessionMismatch);
    }
    if session.availability == AgentSessionAvailability::Archived {
        return Err(ContractViolation::ArchivedSessionCannotStartInvocation {
            session_id: session.id.clone(),
        });
    }
    if candidate.status != AgentInvocationStatus::Pending {
        return Err(ContractViolation::InvocationMustStartPending);
    }
    if let Some(active) =
        active_invocation.filter(|invocation| invocation.status == AgentInvocationStatus::Running)
    {
        return Err(ContractViolation::ActiveInvocationExists {
            invocation_id: active.id.clone(),
        });
    }
    validate_invocation(candidate)
}

pub fn validate_invocation(invocation: &AgentInvocation) -> Result<(), ContractViolation> {
    if invocation.updated_at < invocation.created_at {
        return Err(ContractViolation::InvalidInvocationRecord {
            reason: "invocation update time cannot precede creation time",
        });
    }

    if invocation.submitted_text.trim().is_empty() && invocation.attachments.is_empty() {
        return Err(ContractViolation::InvalidInvocationRecord {
            reason: "an invocation requires text or an attachment",
        });
    }
    if invocation.attachments.len() > 4
        || invocation.attachments.iter().any(|image| {
            image.id.len() != 64
                || !image.id.bytes().all(|value| value.is_ascii_hexdigit())
                || image.media_type.is_empty()
                || image.media_type.len() > 128
                || image.name.trim().is_empty()
                || image.data_base64.len() > 14 * 1024 * 1024
        })
    {
        return Err(ContractViolation::InvalidInvocationRecord {
            reason: "image attachments are invalid",
        });
    }

    match invocation.status {
        AgentInvocationStatus::Pending => {
            if invocation.started_at.is_some()
                || invocation.completed_at.is_some()
                || has_terminal_metadata(invocation)
            {
                return Err(ContractViolation::InvalidInvocationRecord {
                    reason: "a pending invocation cannot have lifecycle or terminal outcome data",
                });
            }
        }
        AgentInvocationStatus::Running => {
            if invocation.started_at.is_none()
                || invocation.completed_at.is_some()
                || invocation.effective_options.is_none()
                || has_terminal_metadata(invocation)
            {
                return Err(ContractViolation::InvalidInvocationRecord {
                    reason: "a running invocation requires effective options and a start time without terminal outcome data",
                });
            }
        }
        status if status.is_terminal() => {
            if invocation.completed_at.is_none() {
                return Err(ContractViolation::InvalidInvocationRecord {
                    reason: "a terminal invocation requires a completion time",
                });
            }
            if status == AgentInvocationStatus::Completed && invocation.runtime_error.is_some() {
                return Err(ContractViolation::InvalidInvocationRecord {
                    reason: "a completed invocation cannot contain a runtime error",
                });
            }
            if status == AgentInvocationStatus::Completed && invocation.started_at.is_none() {
                return Err(ContractViolation::InvalidInvocationRecord {
                    reason: "a completed invocation requires a start time",
                });
            }
        }
        _ => unreachable!("all invocation statuses are covered"),
    }

    if let Some(started_at) = invocation.started_at {
        if started_at < invocation.created_at || started_at > invocation.updated_at {
            return Err(ContractViolation::InvalidInvocationRecord {
                reason: "invocation start time is outside its record lifetime",
            });
        }
    }

    if let Some(completed_at) = invocation.completed_at {
        if completed_at < invocation.created_at || completed_at > invocation.updated_at {
            return Err(ContractViolation::InvalidInvocationRecord {
                reason: "invocation completion time is outside its record lifetime",
            });
        }
        if invocation
            .started_at
            .is_some_and(|started_at| completed_at < started_at)
        {
            return Err(ContractViolation::InvalidInvocationRecord {
                reason: "invocation completion time cannot precede its start time",
            });
        }
    }

    Ok(())
}

fn has_terminal_metadata(invocation: &AgentInvocation) -> bool {
    invocation.exit_code.is_some()
        || invocation.signal.is_some()
        || invocation.runtime_error.is_some()
}

pub fn validate_invocation_status_transition(
    current: AgentInvocationStatus,
    candidate: AgentInvocationStatus,
) -> Result<(), ContractViolation> {
    let allowed = matches!(
        (current, candidate),
        (
            AgentInvocationStatus::Pending,
            AgentInvocationStatus::Running
        ) | (
            AgentInvocationStatus::Pending,
            AgentInvocationStatus::Failed
        ) | (
            AgentInvocationStatus::Pending,
            AgentInvocationStatus::Canceled
        ) | (
            AgentInvocationStatus::Pending,
            AgentInvocationStatus::Interrupted
        ) | (
            AgentInvocationStatus::Running,
            AgentInvocationStatus::Completed
        ) | (
            AgentInvocationStatus::Running,
            AgentInvocationStatus::Failed
        ) | (
            AgentInvocationStatus::Running,
            AgentInvocationStatus::Canceled
        ) | (
            AgentInvocationStatus::Running,
            AgentInvocationStatus::Interrupted
        )
    );

    if allowed {
        Ok(())
    } else {
        Err(ContractViolation::InvalidInvocationTransition {
            from: current,
            to: candidate,
        })
    }
}

pub fn validate_next_event(
    invocation_id: &AgentInvocationId,
    previous: Option<&AgentRuntimeEvent>,
    candidate: &AgentRuntimeEvent,
) -> Result<(), ContractViolation> {
    if &candidate.invocation_id != invocation_id
        || previous.is_some_and(|event| &event.invocation_id != invocation_id)
    {
        return Err(ContractViolation::EventInvocationMismatch);
    }

    if let Some(previous) = previous {
        if candidate.sequence <= previous.sequence {
            return Err(ContractViolation::EventSequenceNotIncreasing {
                previous: previous.sequence,
                candidate: candidate.sequence,
            });
        }
    }

    Ok(())
}
