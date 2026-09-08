use super::{
    EventDeliveryRecord, EventGroupRecord, ReferenceIdentity, SessionEventResult,
    SessionEventStore, SessionEventStoreError,
};
use std::sync::Arc;

/// Read-only application facade over recorded Session Event runtime truth.
///
/// Definitions and occurrences remain inputs to materialization and dispatch. This facade returns
/// only records produced by an attempted dispatch.
pub(crate) struct SessionEventQueryApplication {
    store: Arc<dyn SessionEventStore>,
}

impl SessionEventQueryApplication {
    pub(crate) fn new(store: Arc<dyn SessionEventStore>) -> Self {
        Self { store }
    }

    pub(crate) fn event_group(
        &self,
        event_group_id: &ReferenceIdentity,
    ) -> Result<Option<EventGroupRecord>, SessionEventStoreError> {
        self.store.event_group(event_group_id)
    }

    pub(crate) fn recorded_event(
        &self,
        event_group_id: &ReferenceIdentity,
    ) -> Result<Option<SessionEventResult>, SessionEventStoreError> {
        let Some(group) = self.store.event_group(event_group_id)? else {
            return Ok(None);
        };
        let deliveries = self.store.deliveries_for_group(event_group_id)?;
        Ok(Some(SessionEventResult { group, deliveries }))
    }

    pub(crate) fn deliveries_for_group(
        &self,
        event_group_id: &ReferenceIdentity,
    ) -> Result<Vec<EventDeliveryRecord>, SessionEventStoreError> {
        self.store.deliveries_for_group(event_group_id)
    }

    pub(crate) fn deliveries_for_session(
        &self,
        session: &ReferenceIdentity,
    ) -> Result<Vec<EventDeliveryRecord>, SessionEventStoreError> {
        self.store.deliveries_for_session(session)
    }
}
