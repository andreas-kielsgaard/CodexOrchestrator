//! Durable interaction read model derived from the Session event log.
use super::domain::AgentInvocationId;
use orchid_engine::contracts::RuntimeControlRecord;
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInteraction {
    pub id: String,
    pub invocation_id: AgentInvocationId,
    pub sequence: u64,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub kind: String,
    pub state: String,
    pub content: Value,
    pub result: Option<String>,
}

pub fn project_interactions(
    history: &crate::agent_sessions::ports::AgentSessionHistory,
) -> Vec<SessionInteraction> {
    project_invocation_interactions(&history.invocations)
}

pub fn pending_request_count(
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
                RuntimeControlRecord::SteeringQueued { input_id, .. } => Some((
                    input_id.clone(),
                    serde_json::to_value(&record).expect("control records serialize"),
                    "steering",
                    "queued",
                )),
                RuntimeControlRecord::SteeringPending { input_id, .. } => Some((
                    input_id.clone(),
                    serde_json::to_value(&record).expect("control records serialize"),
                    "steering",
                    "pending",
                )),
                RuntimeControlRecord::RequestOpened { request } => Some((
                    request.id.clone(),
                    serde_json::to_value(request).expect("requests serialize"),
                    "request",
                    "pending",
                )),
                RuntimeControlRecord::RequestUnsupported { request, .. } => Some((
                    request.id.clone(),
                    serde_json::to_value(request).expect("requests serialize"),
                    "request",
                    "unsupported",
                )),
                _ => None,
            };
            if let Some((id, content, kind, state)) = opened {
                interactions
                    .retain(|i| !(i.invocation_id == invocation.invocation.id && i.id == id));
                interactions.push(SessionInteraction {
                    id,
                    invocation_id: invocation.invocation.id.clone(),
                    sequence: event.sequence,
                    recorded_at: event.recorded_at,
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
                    && matches!(i.state.as_str(), "pending" | "responding" | "queued")
            }) {
                interaction.state = if interaction.state == "queued" {
                    "expired"
                } else if interaction.kind == "steering" || interaction.state == "responding" {
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
