//! Resolve display membership for order validation without changing execution ownership.
use super::application::SessionNavigationData;
use crate::agent_sessions::{domain::AgentSessionId, organization::SessionPlacement};

pub(super) fn folder_id(
    data: &SessionNavigationData,
    id: &AgentSessionId,
    placement: Option<&SessionPlacement>,
) -> String {
    let placement = placement.or_else(|| {
        data.organization
            .iter()
            .find(|m| &m.session_id == id)
            .map(|m| &m.placement)
    });
    let instance = match placement {
        Some(SessionPlacement::WorkflowInstance { instance_id }) => Some(instance_id.as_str()),
        None | Some(SessionPlacement::Default) => data
            .owners
            .iter()
            .find(|o| &o.session_id == id)
            .map(|o| o.instance_id.as_str()),
        _ => None,
    };
    if let Some(instance_id) = instance {
        if data.instances.iter().any(|i| {
            i.id == instance_id && data.repositories.iter().any(|r| r.id == i.repository_id)
        }) {
            return format!("instance:{instance_id}");
        }
    }
    if let Some(SessionPlacement::Repository { repository_id }) = placement {
        if data.repositories.iter().any(|r| &r.id == repository_id) {
            return format!("repo:{repository_id}:sessions");
        }
    }
    "unfiled".into()
}

pub(super) fn session_ids(data: &SessionNavigationData, folder: &str) -> Vec<String> {
    data.summaries
        .iter()
        .filter(|s| folder_id(data, &s.session.id, None) == folder)
        .map(|s| s.session.id.as_str().to_owned())
        .collect()
}
