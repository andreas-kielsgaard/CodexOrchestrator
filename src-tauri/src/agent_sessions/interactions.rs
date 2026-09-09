//! Durable interaction read model derived from the Session event log.
use super::domain::{AgentInvocationId, AgentRuntimeEventSource};
use serde::Serialize;
use serde_json::Value;
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionInteraction {
    pub(crate) id: String,
    pub(crate) invocation_id: AgentInvocationId,
    pub(crate) sequence: u64,
    pub(crate) kind: String,
    pub(crate) state: String,
    pub(crate) content: Value,
    pub(crate) result: Option<String>,
}

pub(crate) fn project_interactions(
    history: &crate::agent_sessions::ports::AgentSessionHistory,
) -> Vec<SessionInteraction> {
    project_invocation_interactions(&history.invocations)
}

pub(crate) fn pending_request_count(
    invocations: &[crate::agent_sessions::ports::AgentInvocationHistory],
) -> usize {
    project_invocation_interactions(invocations)
        .iter()
        .filter(|interaction| {
            interaction.kind == "request"
                && matches!(interaction.state.as_str(), "pending" | "responding")
        })
        .count()
}

fn project_invocation_interactions(
    invocations: &[crate::agent_sessions::ports::AgentInvocationHistory],
) -> Vec<SessionInteraction> {
    let mut interactions: Vec<SessionInteraction> = Vec::new();
    for invocation in invocations {
        for event in &invocation.events {
            if event.source != AgentRuntimeEventSource::Runtime {
                continue;
            }
            let payload = &event.raw_payload;
            let kind = payload["kind"].as_str().unwrap_or("");
            match kind {
                "session_steering_pending"
                | "runtime_request_opened"
                | "runtime_request_unsupported" => {
                    let (id, content, interaction_kind, state) =
                        if kind == "session_steering_pending" {
                            (
                                payload["inputId"].as_str(),
                                payload.clone(),
                                "steering",
                                "pending",
                            )
                        } else {
                            (
                                payload["request"]["id"].as_str(),
                                payload["request"].clone(),
                                "request",
                                if kind == "runtime_request_unsupported" {
                                    "unsupported"
                                } else {
                                    "pending"
                                },
                            )
                        };
                    if let Some(id) = id {
                        interactions.push(SessionInteraction {
                            id: id.into(),
                            invocation_id: invocation.invocation.id.clone(),
                            sequence: event.sequence,
                            kind: interaction_kind.into(),
                            state: state.into(),
                            content,
                            result: None,
                        });
                    }
                }
                "session_steering_result" | "runtime_request_response" => {
                    if let Some(interaction) = interactions.iter_mut().rev().find(|i| {
                        i.invocation_id == invocation.invocation.id
                            && Some(i.id.as_str()) == payload["id"].as_str()
                    }) {
                        interaction.state = payload["state"].as_str().unwrap_or("uncertain").into();
                        interaction.result = payload["message"].as_str().map(str::to_string);
                    }
                }
                _ => {}
            }
        }
        if invocation.invocation.status.is_terminal() {
            for interaction in interactions.iter_mut().filter(|i| {
                i.invocation_id == invocation.invocation.id
                    && matches!(i.state.as_str(), "pending" | "responding")
            }) {
                interaction.state =
                    if interaction.kind == "steering" || interaction.state == "responding" {
                        "uncertain"
                    } else {
                        "expired"
                    }
                    .into();
            }
        }
    }
    interactions
}
