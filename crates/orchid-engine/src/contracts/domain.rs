use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{error::Error, fmt};
macro_rules! id_type {
    ($name:ident, $label:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, ContractViolation> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(ContractViolation::EmptyIdentifier { kind: $label });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
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
pub enum RuntimeSandboxMode {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRuntimeOptions {
    pub model: Option<String>,
    pub sandbox: Option<RuntimeSandboxMode>,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentInvocationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Canceled,
    Interrupted,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentInvocationTerminalStatus {
    Completed,
    Failed,
    Canceled,
    Interrupted,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRuntimeFailure {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRuntimeEventSource {
    Stdout,
    Stderr,
    Runtime,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NormalizedRuntimeEventKind {
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
/// Provider-neutral semantic detail for a tool item. Raw provider evidence stays on the event.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolActivityPhase {
    Started,
    Completed,
    Unknown,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolResultClassification {
    Succeeded,
    Failed,
    Unknown,
}
/// What a tool item did, independent of the provider's own item vocabulary.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolActivityKind {
    Command,
    FileChange,
    WebSearch,
    Plan,
    /// Records written before this field existed default here: MCP tool calls were then the
    /// only items that carried tool activity.
    #[default]
    McpTool,
    Other,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedToolActivity {
    #[serde(default)]
    pub kind: ToolActivityKind,
    pub phase: ToolActivityPhase,
    pub item_id: Option<String>,
    pub server: Option<String>,
    pub tool: Option<String>,
    pub status: Option<String>,
    pub result_classification: ToolResultClassification,
}
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRuntimeUsage {
    pub input_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedRuntimeEvent {
    pub kind: NormalizedRuntimeEventKind,
    pub text: Option<String>,
    pub external_context_id: Option<ExternalRuntimeContextId>,
    pub usage: Option<AgentRuntimeUsage>,
    pub details: Option<Value>,
    /// Older durable event records predate this field. Missing data remains absent rather than
    /// being reconstructed from raw provider payloads.
    #[serde(default)]
    pub tool_activity: Option<NormalizedToolActivity>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractViolation {
    EmptyIdentifier {
        kind: &'static str,
    },
    SessionIdentityChanged,
    SessionProfileChanged,
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
impl AgentInvocationStatus {
    pub fn is_active(self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Canceled | Self::Interrupted
        )
    }
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
impl fmt::Display for ContractViolation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier { kind } => write!(formatter, "{kind} ID cannot be empty"),
            Self::SessionIdentityChanged => formatter.write_str("local session ID cannot change"),
            Self::SessionProfileChanged => {
                formatter.write_str("a pinned Session Profile cannot change")
            }
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
