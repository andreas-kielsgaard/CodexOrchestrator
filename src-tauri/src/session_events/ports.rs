use super::domain::{
    DirectUserInvocationOptions, EventDeliveryRecord, EventGroupRecord, ReferenceIdentity,
    SessionCreationConfiguration, SessionDirectoryEntry, SessionEventSource, SessionLogicalAddress,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionCreationSpec {
    pub(crate) logical_address: SessionLogicalAddress,
    pub(crate) event_group_id: ReferenceIdentity,
    pub(crate) created_by_event: ReferenceIdentity,
    pub(crate) created_by_session: Option<ReferenceIdentity>,
    pub(crate) configuration: SessionCreationConfiguration,
}

pub(crate) trait SessionDirectory: Send + Sync {
    fn find_exact(
        &self,
        session: &ReferenceIdentity,
    ) -> Result<Option<SessionDirectoryEntry>, SessionDirectoryError>;

    fn list_at_address(
        &self,
        address: &SessionLogicalAddress,
    ) -> Result<Vec<SessionDirectoryEntry>, SessionDirectoryError>;

    fn create_session(
        &self,
        request: SessionCreationSpec,
    ) -> Result<SessionDirectoryEntry, SessionDirectoryError>;

    fn mark_addressed(
        &self,
        session: &ReferenceIdentity,
        event_group_id: &ReferenceIdentity,
    ) -> Result<u64, SessionDirectoryError>;
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionInvocationRequest {
    pub(crate) event_group_id: ReferenceIdentity,
    pub(crate) delivery_id: ReferenceIdentity,
    pub(crate) target_session: ReferenceIdentity,
    pub(crate) source: SessionEventSource,
    pub(crate) prompt: String,
    pub(crate) initial_prompt: Option<String>,
    pub(crate) direct_user_options: Option<DirectUserInvocationOptions>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionInvocationReceipt {
    pub(crate) invocation: ReferenceIdentity,
}

pub(crate) trait SessionInvocationDispatcher: Send + Sync {
    fn dispatch(
        &self,
        request: SessionInvocationRequest,
    ) -> Result<SessionInvocationReceipt, SessionInvocationError>;
}

pub(crate) trait SessionEventStore: Send + Sync {
    /// Persists one event group and all of its delivery outcomes as one store operation.
    fn record(
        &self,
        group: EventGroupRecord,
        deliveries: Vec<EventDeliveryRecord>,
    ) -> Result<(), SessionEventStoreError>;

    fn event_group(
        &self,
        event_group_id: &ReferenceIdentity,
    ) -> Result<Option<EventGroupRecord>, SessionEventStoreError>;

    /// Returns deliveries in their recorded ordinal order, or an empty list for an unknown group.
    fn deliveries_for_group(
        &self,
        event_group_id: &ReferenceIdentity,
    ) -> Result<Vec<EventDeliveryRecord>, SessionEventStoreError>;

    /// Returns recorded deliveries targeting one Session in persistence order.
    fn deliveries_for_session(
        &self,
        session: &ReferenceIdentity,
    ) -> Result<Vec<EventDeliveryRecord>, SessionEventStoreError>;
}

macro_rules! port_error {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub(crate) struct $name {
            message: String,
        }

        impl $name {
            pub(crate) fn new(message: impl Into<String>) -> Self {
                Self {
                    message: message.into(),
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.message)
            }
        }

        impl Error for $name {}
    };
}

port_error!(SessionDirectoryError);
port_error!(SessionInvocationError);
port_error!(SessionEventStoreError);
