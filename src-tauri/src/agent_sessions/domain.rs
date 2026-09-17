use crate::{
    execution_configuration::SessionCreationResolution, harness_engine::domain::HarnessVersionRef,
    identities::AssignedAgentIdentity,
};
use chrono::{DateTime, Utc};
pub(crate) use orchid_engine::contracts::domain::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentSessionAvailability {
    Available,
    Archived,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentRuntimeBinding {
    pub(crate) external_context_id: Option<ExternalRuntimeContextId>,
    pub(crate) runtime_version: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentSession {
    #[serde(default)]
    pub(crate) execution_target: Option<crate::execution_targets::domain::SessionExecutionTarget>,
    #[serde(default)]
    pub(crate) workspace_origin: Option<String>,
    pub(crate) id: AgentSessionId,
    pub(crate) title: String,
    pub(crate) availability: AgentSessionAvailability,
    pub(crate) runtime_binding: AgentRuntimeBinding,
    pub(crate) working_directory: Option<String>,
    pub(crate) requested_options: AgentRuntimeOptions,
    /// Immutable execution configuration pinned by the replacement Session Event path at
    /// creation. Legacy Sessions may remain unprofiled until their concepts are retired.
    pub(crate) session_profile: Option<SessionCreationResolution>,
    /// Exact reusable or Session-specific Harness version currently owned by this Session.
    pub(crate) harness_version: Option<HarnessVersionRef>,
    /// Session-owned snapshot; it is independent of later Harness or identity-definition edits.
    pub(crate) assigned_identity: Option<AssignedAgentIdentity>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentInvocationInputProvenance {
    User,
    Application,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentDiagnosticSource {
    Repository,
    Runtime,
    Transport,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentDiagnostic {
    pub(crate) source: AgentDiagnosticSource,
    pub(crate) severity: AgentDiagnosticSeverity,
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) details: Option<Value>,
    pub(crate) recorded_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentInvocation {
    pub(crate) id: AgentInvocationId,
    pub(crate) session_id: AgentSessionId,
    pub(crate) submitted_text: String,
    pub(crate) input_provenance: AgentInvocationInputProvenance,
    pub(crate) status: AgentInvocationStatus,
    pub(crate) requested_options: AgentRuntimeOptions,
    pub(crate) effective_options: Option<AgentRuntimeOptions>,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) signal: Option<String>,
    pub(crate) runtime_error: Option<AgentRuntimeFailure>,
    pub(crate) diagnostics: Vec<AgentDiagnostic>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct InvocationCompletion {
    pub(crate) status: AgentInvocationTerminalStatus,
    pub(crate) completed_at: DateTime<Utc>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) signal: Option<String>,
    pub(crate) runtime_error: Option<AgentRuntimeFailure>,
}

impl AgentInvocation {
    pub(crate) fn mark_running(
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

    pub(crate) fn finish(
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
    pub(crate) fn recover_pre_acceptance_interruption(
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
pub(crate) struct AgentRuntimeEvent {
    pub(crate) id: AgentRuntimeEventId,
    pub(crate) invocation_id: AgentInvocationId,
    pub(crate) sequence: u64,
    pub(crate) source: AgentRuntimeEventSource,
    pub(crate) raw_payload: Value,
    pub(crate) normalized: Option<NormalizedRuntimeEvent>,
    pub(crate) recorded_at: DateTime<Utc>,
}

pub(crate) fn validate_session_update(
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

pub(crate) fn validate_session(session: &AgentSession) -> Result<(), ContractViolation> {
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

pub(crate) fn validate_runtime_binding_update(
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

pub(crate) fn validate_new_invocation(
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
    if let Some(active) = active_invocation.filter(|invocation| invocation.status.is_active()) {
        return Err(ContractViolation::ActiveInvocationExists {
            invocation_id: active.id.clone(),
        });
    }
    validate_invocation(candidate)
}

pub(crate) fn validate_invocation(invocation: &AgentInvocation) -> Result<(), ContractViolation> {
    if invocation.updated_at < invocation.created_at {
        return Err(ContractViolation::InvalidInvocationRecord {
            reason: "invocation update time cannot precede creation time",
        });
    }

    if invocation.submitted_text.trim().is_empty() {
        return Err(ContractViolation::InvalidInvocationRecord {
            reason: "submitted text cannot be empty",
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

pub(crate) fn validate_invocation_status_transition(
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

pub(crate) fn validate_next_event(
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
