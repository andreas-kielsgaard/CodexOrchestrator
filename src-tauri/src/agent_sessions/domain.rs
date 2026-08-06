use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt};

macro_rules! id_type {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn new(value: impl Into<String>) -> Result<Self, ContractViolation> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(ContractViolation::EmptyIdentifier { kind: $label });
                }
                Ok(Self(value))
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

id_type!(AgentSessionId, "agent session");
id_type!(AgentInvocationId, "agent invocation");
id_type!(AgentRuntimeEventId, "agent runtime event");
id_type!(ExternalRuntimeContextId, "external runtime context");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentSessionAvailability {
    Available,
    Archived,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RuntimeSandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentRuntimeOptions {
    pub(crate) model: Option<String>,
    pub(crate) sandbox: Option<RuntimeSandboxMode>,
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
    pub(crate) id: AgentSessionId,
    pub(crate) title: String,
    pub(crate) availability: AgentSessionAvailability,
    pub(crate) runtime_binding: AgentRuntimeBinding,
    pub(crate) working_directory: Option<String>,
    pub(crate) requested_options: AgentRuntimeOptions,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentInvocationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Canceled,
    Interrupted,
}

impl AgentInvocationStatus {
    pub(crate) fn is_active(self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Canceled | Self::Interrupted
        )
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentInvocationTerminalStatus {
    Completed,
    Failed,
    Canceled,
    Interrupted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentInvocationInputProvenance {
    User,
    Application,
}

impl From<AgentInvocationTerminalStatus> for AgentInvocationStatus {
    fn from(status: AgentInvocationTerminalStatus) -> Self {
        match status {
            AgentInvocationTerminalStatus::Completed => Self::Completed,
            AgentInvocationTerminalStatus::Failed => Self::Failed,
            AgentInvocationTerminalStatus::Canceled => Self::Canceled,
            AgentInvocationTerminalStatus::Interrupted => Self::Interrupted,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentRuntimeFailure {
    pub(crate) code: String,
    pub(crate) message: String,
    pub(crate) details: Option<Value>,
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
            && self.runtime_error.as_ref().is_some_and(|error| {
                error.code == "runtime_startup_without_launch_acceptance"
            });
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentRuntimeEventSource {
    Stdout,
    Stderr,
    Runtime,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum NormalizedRuntimeEventKind {
    RuntimeContextEstablished,
    ProcessingStarted,
    ProcessingUpdate,
    ToolActivity,
    AgentMessage,
    Usage,
    InvocationCompleted,
    RuntimeError,
    Unknown,
}

/// Provider-neutral semantic detail for a tool item. The enclosing runtime event retains the
/// provider payload for audit; consumers use these fields without parsing it.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ToolActivityPhase {
    Started,
    Completed,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ToolResultClassification {
    Succeeded,
    Failed,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NormalizedToolActivity {
    pub(crate) phase: ToolActivityPhase,
    pub(crate) item_id: Option<String>,
    pub(crate) server: Option<String>,
    pub(crate) tool: Option<String>,
    pub(crate) status: Option<String>,
    pub(crate) result_classification: ToolResultClassification,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentRuntimeUsage {
    pub(crate) input_tokens: Option<u64>,
    pub(crate) cached_input_tokens: Option<u64>,
    pub(crate) output_tokens: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NormalizedRuntimeEvent {
    pub(crate) kind: NormalizedRuntimeEventKind,
    pub(crate) text: Option<String>,
    pub(crate) external_context_id: Option<ExternalRuntimeContextId>,
    pub(crate) usage: Option<AgentRuntimeUsage>,
    pub(crate) details: Option<Value>,
    /// Older durable event records predate this field. Missing data remains absent rather than
    /// being reconstructed from raw provider payloads.
    #[serde(default)]
    pub(crate) tool_activity: Option<NormalizedToolActivity>,
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ContractViolation {
    EmptyIdentifier {
        kind: &'static str,
    },
    SessionIdentityChanged,
    ExternalRuntimeContextChanged,
    InvalidSessionRecord {
        reason: &'static str,
    },
    ArchivedSessionCannotStartInvocation {
        session_id: AgentSessionId,
    },
    InvocationSessionMismatch,
    InvocationMustStartPending,
    ActiveInvocationExists {
        invocation_id: AgentInvocationId,
    },
    InvalidInvocationTransition {
        from: AgentInvocationStatus,
        to: AgentInvocationStatus,
    },
    InvalidInvocationRecord {
        reason: &'static str,
    },
    EventInvocationMismatch,
    EventSequenceNotIncreasing {
        previous: u64,
        candidate: u64,
    },
}

impl fmt::Display for ContractViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier { kind } => write!(formatter, "{kind} ID cannot be empty"),
            Self::SessionIdentityChanged => formatter.write_str("local session ID cannot change"),
            Self::ExternalRuntimeContextChanged => formatter
                .write_str("an established external runtime context cannot be cleared or replaced"),
            Self::InvalidSessionRecord { reason } => formatter.write_str(reason),
            Self::ArchivedSessionCannotStartInvocation { session_id } => write!(
                formatter,
                "archived session {session_id} cannot start a new invocation"
            ),
            Self::InvocationSessionMismatch => {
                formatter.write_str("invocation does not belong to the target session")
            }
            Self::InvocationMustStartPending => {
                formatter.write_str("a new invocation must have pending status")
            }
            Self::ActiveInvocationExists { invocation_id } => write!(
                formatter,
                "session already has active invocation {invocation_id}"
            ),
            Self::InvalidInvocationTransition { from, to } => {
                write!(
                    formatter,
                    "invalid invocation transition from {from:?} to {to:?}"
                )
            }
            Self::InvalidInvocationRecord { reason } => formatter.write_str(reason),
            Self::EventInvocationMismatch => {
                formatter.write_str("event does not belong to the target invocation")
            }
            Self::EventSequenceNotIncreasing {
                previous,
                candidate,
            } => write!(
                formatter,
                "event sequence {candidate} must be greater than previous sequence {previous}"
            ),
        }
    }
}

impl Error for ContractViolation {}

pub(crate) fn validate_session_update(
    current: &AgentSession,
    candidate: &AgentSession,
) -> Result<(), ContractViolation> {
    validate_session(candidate)?;
    if current.id != candidate.id {
        return Err(ContractViolation::SessionIdentityChanged);
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
