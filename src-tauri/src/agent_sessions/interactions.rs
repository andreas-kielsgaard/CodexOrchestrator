//! Durable interaction read model derived from the Session event log.
use super::domain::AgentInvocationId;
use orchid_engine::contracts::RuntimeControlRecord;
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
            let Some(record) = RuntimeControlRecord::from_event(event.source, &event.raw_payload)
            else {
                continue;
            };
            let opened = match &record {
                RuntimeControlRecord::SteeringPending { input_id, .. } => Some((
                    input_id.clone(),
                    serde_json::to_value(&record).expect("control records serialize"),
                    "steering",
                    "pending",
                )),
                RuntimeControlRecord::RequestOpened { request } => request["id"]
                    .as_str()
                    .map(|id| (id.to_owned(), request.clone(), "request", "pending")),
                RuntimeControlRecord::RequestUnsupported { request, .. } => request["id"]
                    .as_str()
                    .map(|id| (id.to_owned(), request.clone(), "request", "unsupported")),
                _ => None,
            };
            if let Some((id, content, kind, state)) = opened {
                interactions.push(SessionInteraction {
                    id,
                    invocation_id: invocation.invocation.id.clone(),
                    sequence: event.sequence,
                    kind: kind.into(),
                    state: state.into(),
                    content,
                    result: None,
                });
                continue;
            }
            if let RuntimeControlRecord::SteeringResult { id, state, message }
            | RuntimeControlRecord::RequestResponse { id, state, message } = record
            {
                if let Some(interaction) = interactions
                    .iter_mut()
                    .rev()
                    .find(|i| i.invocation_id == invocation.invocation.id && i.id == id)
                {
                    interaction.state = state;
                    interaction.result = message;
                }
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
