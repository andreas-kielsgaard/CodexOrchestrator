use super::{
    EventDeliveryRecord, EventGroupRecord, ReferenceIdentity, SessionEventQueryApplication,
    SessionEventResult,
};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

pub(crate) struct SessionEventQueryTauriState {
    queries: Arc<SessionEventQueryApplication>,
}

impl SessionEventQueryTauriState {
    pub(crate) fn new(queries: Arc<SessionEventQueryApplication>) -> Self {
        Self { queries }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EventGroupQuery {
    event_group_id: ReferenceIdentity,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionEventDeliveryQuery {
    session: ReferenceIdentity,
}

#[tauri::command]
pub(crate) fn load_session_event_group(
    state: State<'_, SessionEventQueryTauriState>,
    query: EventGroupQuery,
) -> Result<Option<EventGroupRecord>, String> {
    state
        .queries
        .event_group(&query.event_group_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn load_recorded_session_event(
    state: State<'_, SessionEventQueryTauriState>,
    query: EventGroupQuery,
) -> Result<Option<SessionEventResult>, String> {
    state
        .queries
        .recorded_event(&query.event_group_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn list_session_event_deliveries_for_group(
    state: State<'_, SessionEventQueryTauriState>,
    query: EventGroupQuery,
) -> Result<Vec<EventDeliveryRecord>, String> {
    state
        .queries
        .deliveries_for_group(&query.event_group_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn list_session_event_deliveries_for_session(
    state: State<'_, SessionEventQueryTauriState>,
    query: SessionEventDeliveryQuery,
) -> Result<Vec<EventDeliveryRecord>, String> {
    state
        .queries
        .deliveries_for_session(&query.session)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_group_query_requires_complete_reference_identity() {
        assert!(
            serde_json::from_value::<EventGroupQuery>(serde_json::json!({
                "eventGroupId": {
                    "namespace": "workflow",
                    "kind": "event_group"
                }
            }))
            .is_err()
        );
    }
}
