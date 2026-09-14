use crate::agent_sessions::{
    application::{
        project_invocation_observation, AgentInvocationObservation, AgentSessionNotification,
        CancelAgentInvocationCommand, CreateAgentSessionCommand, SendAgentSessionMessageCommand,
        SendAgentSessionMessageResult, UpdateAgentSessionHarnessCommand,
        UpdateAgentSessionIdentityCommand, UpdateAgentSessionModelOverrideCommand,
    },
    domain::{
        AgentInvocation, AgentInvocationId, AgentRuntimeEvent, AgentRuntimeOptions, AgentSession,
        AgentSessionAvailability, AgentSessionId,
    },
    ports::{AgentSessionHistory, AgentSessionSummary, ListAgentSessionsQuery},
};
use crate::{harness_engine::domain::HarnessVersionRef, identities::AssignedAgentIdentity};
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
    #[serde(default)]
    pub(crate) harness_version: Option<HarnessVersionRef>,
    #[serde(default)]
    pub(crate) assigned_identity: Option<AssignedAgentIdentity>,
}

impl CreateAgentSessionCommandDto {
    pub(crate) fn ownership(&self) -> crate::agent_sessions::application::AgentSessionOwnership {
        crate::agent_sessions::application::AgentSessionOwnership {
            execution_target: None,
            harness_version: self.harness_version.clone(),
            assigned_identity: self.assigned_identity.clone(),
            session_profile: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateAgentSessionHarnessCommandDto {
    pub(crate) session_id: AgentSessionId,
    pub(crate) harness_version: Option<HarnessVersionRef>,
}

impl From<UpdateAgentSessionHarnessCommandDto> for UpdateAgentSessionHarnessCommand {
    fn from(value: UpdateAgentSessionHarnessCommandDto) -> Self {
        Self {
            session_id: value.session_id,
            harness_version: value.harness_version,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateAgentSessionIdentityCommandDto {
    pub(crate) session_id: AgentSessionId,
    pub(crate) assigned_identity: Option<AssignedAgentIdentity>,
}

impl From<UpdateAgentSessionIdentityCommandDto> for UpdateAgentSessionIdentityCommand {
    fn from(value: UpdateAgentSessionIdentityCommandDto) -> Self {
        Self {
            session_id: value.session_id,
            assigned_identity: value.assigned_identity,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateAgentSessionModelOverrideCommandDto {
    pub(crate) session_id: AgentSessionId,
    pub(crate) model: Option<String>,
}

impl From<UpdateAgentSessionModelOverrideCommandDto> for UpdateAgentSessionModelOverrideCommand {
    fn from(value: UpdateAgentSessionModelOverrideCommandDto) -> Self {
        Self {
            session_id: value.session_id,
            model: value.model,
        }
    }
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
    pub(crate) observation: AgentInvocationObservation,
    pub(crate) events: Vec<AgentRuntimeEventDto>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentSessionDetailsDto {
    pub(crate) interactions: Vec<crate::agent_sessions::application::SessionInteraction>,
    pub(crate) session: AgentSessionDto,
    pub(crate) invocations: Vec<AgentInvocationDetailsDto>,
}

impl AgentSessionDetailsDto {
    pub(crate) fn from_history(history: AgentSessionHistory) -> Self {
        Self {
            interactions: crate::agent_sessions::application::project_interactions(&history),
            session: history.session,
            invocations: history
                .invocations
                .into_iter()
                .map(|history| AgentInvocationDetailsDto {
                    observation: project_invocation_observation(&history),
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
    pub(crate) pending_request_count: usize,
    pub(crate) id: AgentSessionId,
    pub(crate) title: String,
    pub(crate) availability: AgentSessionAvailability,
    pub(crate) has_active_invocation: bool,
    pub(crate) latest_invocation_status:
        Option<crate::agent_sessions::domain::AgentInvocationStatus>,
    pub(crate) created_at: chrono::DateTime<chrono::Utc>,
    pub(crate) updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<AgentSessionSummary> for AgentSessionSummaryDto {
    fn from(value: AgentSessionSummary) -> Self {
        Self {
            pending_request_count: value.pending_request_count,
            id: value.session.id,
            title: value.session.title,
            availability: value.session.availability,
            has_active_invocation: value
                .latest_invocation_status
                .is_some_and(|status| status.is_active()),
            latest_invocation_status: value.latest_invocation_status,
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
    SteeringAccepted {
        session_id: AgentSessionId,
        invocation_id: AgentInvocationId,
        input_id: String,
    },
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
            AgentSessionNotification::SteeringAccepted {
                session_id,
                invocation_id,
                input_id,
            } => Self::SteeringAccepted {
                session_id,
                invocation_id,
                input_id,
            },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_sessions::{
        domain::{
            AgentInvocationInputProvenance, AgentInvocationStatus, AgentRuntimeBinding,
            AgentSessionAvailability,
        },
        ports::{AgentInvocationHistory, AgentSessionHistory},
    };
    use chrono::{TimeZone, Utc};

    #[test]
    fn model_override_transport_accepts_an_opaque_model_or_explicit_clear() {
        let selected: UpdateAgentSessionModelOverrideCommand = serde_json::from_value::<
            UpdateAgentSessionModelOverrideCommandDto,
        >(serde_json::json!({
            "sessionId": "session",
            "model": "user-selected-model"
        }))
        .unwrap()
        .into();
        assert_eq!(selected.session_id.as_str(), "session");
        assert_eq!(selected.model.as_deref(), Some("user-selected-model"));

        let cleared: UpdateAgentSessionModelOverrideCommand = serde_json::from_value::<
            UpdateAgentSessionModelOverrideCommandDto,
        >(serde_json::json!({
            "sessionId": "session",
            "model": null
        }))
        .unwrap()
        .into();
        assert!(cleared.model.is_none());
    }

    #[test]
    fn serializes_observation_from_durable_history_without_application_acceptance() {
        let at = Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).unwrap();
        let session_id = AgentSessionId::new("session").unwrap();
        let invocation_id = AgentInvocationId::new("invocation").unwrap();
        let dto = AgentSessionDetailsDto::from_history(AgentSessionHistory {
            session: AgentSession {
                execution_target: None,
                workspace_origin: None,
                id: session_id.clone(),
                title: "Session".into(),
                availability: AgentSessionAvailability::Available,
                runtime_binding: AgentRuntimeBinding {
                    external_context_id: None,
                    runtime_version: None,
                },
                working_directory: None,
                requested_options: AgentRuntimeOptions::default(),
                session_profile: None,
                harness_version: None,
                assigned_identity: None,
                created_at: at,
                updated_at: at,
            },
            invocations: vec![AgentInvocationHistory {
                invocation: AgentInvocation {
                    id: invocation_id,
                    session_id,
                    submitted_text: "Work".into(),
                    input_provenance: AgentInvocationInputProvenance::User,
                    status: AgentInvocationStatus::Pending,
                    requested_options: AgentRuntimeOptions::default(),
                    effective_options: None,
                    started_at: None,
                    completed_at: None,
                    exit_code: None,
                    signal: None,
                    runtime_error: None,
                    diagnostics: vec![],
                    created_at: at,
                    updated_at: at,
                },
                launch_accepted_at: Some(at),
                events: vec![],
            }],
        });
        let value = serde_json::to_value(dto).unwrap();
        assert_eq!(
            value["invocations"][0]["observation"]["launchAcceptedAt"],
            "2026-08-02T12:00:00Z"
        );
        assert!(value["invocations"][0]["observation"]
            .get("applicationAcceptance")
            .is_none());
    }
}
