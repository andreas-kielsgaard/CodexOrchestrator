//! One application-owned entry point for prompt delivery. Callers describe semantic input; route
//! planning, continuity, Harness exposure, provider preparation and release remain internal.
pub(super) mod context;
mod continuity;
mod contracts;

pub(crate) use contracts::InvocationDeliveryIntent;

use super::{
    preparation::PreparedMessageInput, AgentSessionApplication, AgentSessionApplicationError,
    SendAgentSessionMessageLaunchResult, SendAgentSessionMessageResult,
};

pub(super) struct InvocationDeliveryService<'a> {
    application: &'a AgentSessionApplication,
}

impl<'a> InvocationDeliveryService<'a> {
    pub(super) fn new(application: &'a AgentSessionApplication) -> Self {
        Self { application }
    }

    pub(super) fn accept_prepared(
        &self,
        input: PreparedMessageInput,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        self.application.accept_prepared_delivery(input)
    }

    pub(super) fn deliver(
        &self,
        intent: InvocationDeliveryIntent,
    ) -> Result<SendAgentSessionMessageLaunchResult, AgentSessionApplicationError> {
        self.application.execute_delivery_intent(intent)
    }
}

impl AgentSessionApplication {
    pub(super) fn delivery(&self) -> InvocationDeliveryService<'_> {
        InvocationDeliveryService::new(self)
    }
}
