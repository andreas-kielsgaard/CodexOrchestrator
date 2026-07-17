use crate::agent_sessions::{
    application::{
        AgentSessionNotification, CancelAgentInvocationCommand, CreateAgentSessionCommand,
        SendAgentSessionMessageCommand, SendAgentSessionMessageResult,
    },
    domain::{
        AgentInvocation, AgentInvocationId, AgentRuntimeEvent, AgentRuntimeOptions, AgentSession,
        AgentSessionAvailability, AgentSessionId,
    },
    ports::{AgentSessionHistory, AgentSessionSummary, ListAgentSessionsQuery},
};
use serde::{Deserialize, Serialize};

pub(crate) type AgentSessionDto = AgentSession;
pub(crate) type AgentInvocationDto = AgentInvocation;
pub(crate) type AgentRuntimeEventDto = AgentRuntimeEvent;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateAgentSessionCommandDto {
    pub(crate) title: Option<String>,
    pub(crate) working_directory: Option<String>,
    #[serde(default)]
    pub(crate) requested_options: AgentRuntimeOptions,
}

impl From<CreateAgentSessionCommandDto> for CreateAgentSessionCommand {
    fn from(value: CreateAgentSessionCommandDto) -> Self {
        Self {
            title: value.title,
            working_directory: value.working_directory,
            requested_options: value.requested_options,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendAgentSessionMessageCommandDto {
    pub(crate) session_id: Option<AgentSessionId>,
    pub(crate) submitted_text: String,
    pub(crate) title: Option<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) requested_options: Option<AgentRuntimeOptions>,
}

impl From<SendAgentSessionMessageCommandDto> for SendAgentSessionMessageCommand {
    fn from(value: SendAgentSessionMessageCommandDto) -> Self {
        Self {
            session_id: value.session_id,
            submitted_text: value.submitted_text,
            title: value.title,
            working_directory: value.working_directory,
            requested_options: value.requested_options,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancelAgentInvocationCommandDto {
    pub(crate) invocation_id: AgentInvocationId,
}

impl From<CancelAgentInvocationCommandDto> for CancelAgentInvocationCommand {
    fn from(value: CancelAgentInvocationCommandDto) -> Self {
        Self {
            invocation_id: value.invocation_id,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ListAgentSessionsQueryDto {
    pub(crate) availability: Option<AgentSessionAvailability>,
    pub(crate) limit: Option<u32>,
}

impl From<ListAgentSessionsQueryDto> for ListAgentSessionsQuery {
    fn from(value: ListAgentSessionsQueryDto) -> Self {
        Self {
            availability: value.availability,
            limit: value.limit,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadAgentSessionQueryDto {
    pub(crate) session_id: AgentSessionId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendAgentSessionMessageResultDto {
    pub(crate) session_id: AgentSessionId,
    pub(crate) invocation_id: AgentInvocationId,
}

impl From<SendAgentSessionMessageResult> for SendAgentSessionMessageResultDto {
    fn from(value: SendAgentSessionMessageResult) -> Self {
        Self {
            session_id: value.session_id,
            invocation_id: value.invocation_id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentInvocationDetailsDto {
    pub(crate) invocation: AgentInvocationDto,
    pub(crate) events: Vec<AgentRuntimeEventDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentSessionDetailsDto {
    pub(crate) session: AgentSessionDto,
    pub(crate) invocations: Vec<AgentInvocationDetailsDto>,
}

impl AgentSessionDetailsDto {
    pub(crate) fn from_history(history: AgentSessionHistory) -> Self {
        Self {
            session: history.session,
            invocations: history
                .invocations
                .into_iter()
                .map(|history| AgentInvocationDetailsDto {
                    invocation: history.invocation,
                    events: history.events,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentSessionSummaryDto {
    pub(crate) id: AgentSessionId,
    pub(crate) title: String,
    pub(crate) availability: AgentSessionAvailability,
    pub(crate) has_active_invocation: bool,
    pub(crate) created_at: chrono::DateTime<chrono::Utc>,
    pub(crate) updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<AgentSessionSummary> for AgentSessionSummaryDto {
    fn from(value: AgentSessionSummary) -> Self {
        Self {
            id: value.session.id,
            title: value.session.title,
            availability: value.session.availability,
            has_active_invocation: value
                .latest_invocation_status
                .is_some_and(|status| status.is_active()),
            created_at: value.session.created_at,
            updated_at: value.session.updated_at,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum AgentSessionUpdateDto {
    EventPersisted {
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
        event: AgentRuntimeEventDto,
    },
    InvocationTerminal {
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
        invocation: AgentInvocationDto,
    },
    DiagnosticRecorded {
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
        invocation: AgentInvocationDto,
    },
}

impl From<AgentSessionNotification> for AgentSessionUpdateDto {
    fn from(value: AgentSessionNotification) -> Self {
        match value {
            AgentSessionNotification::EventPersisted { session_id, event } => {
                Self::EventPersisted {
                    session_id,
                    invocation_id: event.invocation_id.clone(),
                    event,
                }
            }
            AgentSessionNotification::InvocationTerminal {
                session_id,
                invocation,
            } => Self::InvocationTerminal {
                session_id,
                invocation_id: invocation.id.clone(),
                invocation,
            },
            AgentSessionNotification::DiagnosticRecorded {
                session_id,
                invocation,
            } => Self::DiagnosticRecorded {
                session_id,
                invocation_id: invocation.id.clone(),
                invocation,
            },
        }
    }
}
