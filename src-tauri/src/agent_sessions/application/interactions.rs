//! Durable turn input and runtime request use cases. The event log is the single history store.
use super::{
    update_sink::PersistedRuntimeUpdateSink, AgentSessionApplication, AgentSessionApplicationError,
    AgentSessionNotification,
};
use crate::agent_sessions::{
    domain::{AgentInvocationId, AgentSessionId},
    ports::{AgentRuntimeUpdateSink, RuntimePortErrorKind, RuntimeUpdate},
};
use orchid_engine::contracts::{RuntimeControlRecord, RuntimeInteractionResponse};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, Weak},
};

#[derive(Default)]
pub(super) struct InteractionLanes(Mutex<HashMap<AgentInvocationId, Weak<Mutex<()>>>>);
impl InteractionLanes {
    fn lane(&self, id: &AgentInvocationId) -> Result<Arc<Mutex<()>>, AgentSessionApplicationError> {
        let mut lanes = self.0.lock().map_err(|_| {
            AgentSessionApplicationError::conflict("Interaction registry is unavailable")
        })?;
        lanes.retain(|_, lane| lane.strong_count() > 0);
        if let Some(lane) = lanes.get(id).and_then(Weak::upgrade) {
            return Ok(lane);
        }
        let lane = Arc::new(Mutex::new(()));
        lanes.insert(id.clone(), Arc::downgrade(&lane));
        Ok(lane)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SteerAgentSessionCommand {
    pub(crate) session_id: AgentSessionId,
    pub(crate) invocation_id: AgentInvocationId,
    pub(crate) input_id: String,
    pub(crate) text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RespondToRuntimeRequestCommand {
    pub(crate) session_id: AgentSessionId,
    pub(crate) invocation_id: AgentInvocationId,
    pub(crate) request_id: String,
    pub(crate) response: RuntimeInteractionResponse,
}

use crate::agent_sessions::interactions::{project_interactions, SessionInteraction};

impl AgentSessionApplication {
    pub(crate) fn session_interactions(
        &self,
        id: &AgentSessionId,
    ) -> Result<Vec<SessionInteraction>, AgentSessionApplicationError> {
        let history = self.load_session(id)?;
        Ok(project_interactions(&history))
    }

    pub(crate) fn steer_session(
        &self,
        command: SteerAgentSessionCommand,
    ) -> Result<SessionInteraction, AgentSessionApplicationError> {
        if command.text.trim().is_empty() || command.input_id.trim().is_empty() {
            return Err(AgentSessionApplicationError::invalid(
                "Steering requires text and an input identity",
            ));
        }
        let lane = self.interaction_lanes.lane(&command.invocation_id)?;
        let _guard = lane.lock().map_err(|_| {
            AgentSessionApplicationError::conflict("Interaction lane is unavailable")
        })?;
        if let Some(existing) = self
            .session_interactions(&command.session_id)?
            .into_iter()
            .find(|i| i.id == command.input_id)
        {
            if existing.invocation_id != command.invocation_id
                || existing.content["text"] != command.text
            {
                return Err(AgentSessionApplicationError::conflict(
                    "Input identity was already used for different content",
                ));
            }
            return Ok(existing);
        }
        self.require_active_interaction(&command.session_id, &command.invocation_id)?;
        if self
            .repository
            .preparation(&command.invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .is_some_and(|p| !p.delivery_started)
        {
            return Err(AgentSessionApplicationError::conflict(
                "The submitted prompt is still preparing",
            ));
        }

        let runtime = self.runtime_for_invocation(&command.invocation_id)?;
        let target = runtime
            .active_turn(&command.invocation_id)
            .map_err(AgentSessionApplicationError::runtime)?;
        self.record_interaction(
            &command.invocation_id,
            RuntimeControlRecord::SteeringPending {
                input_id: command.input_id.clone(),
                text: command.text.clone(),
                target: target.clone(),
            },
        )?;
        let result = runtime.steer(
            &command.invocation_id,
            &target,
            &command.input_id,
            &command.text,
        );
        let state = match &result {
            Ok(()) => "accepted",
            Err(e) if e.kind == RuntimePortErrorKind::Unavailable => "uncertain",
            Err(_) => "rejected",
        };
        self.record_interaction(
            &command.invocation_id,
            RuntimeControlRecord::SteeringResult {
                id: command.input_id.clone(),
                state: state.into(),
                message: result.as_ref().err().map(|e| e.message.clone()),
            },
        )?;
        if state == "accepted" {
            self.notify_or_record(AgentSessionNotification::SteeringAccepted {
                session_id: command.session_id.clone(),
                invocation_id: command.invocation_id.clone(),
                input_id: command.input_id.clone(),
            });
        }
        self.session_interactions(&command.session_id)?
            .into_iter()
            .find(|i| i.id == command.input_id)
            .ok_or_else(|| AgentSessionApplicationError::not_found("Steering record disappeared"))
    }

    pub(crate) fn respond_to_runtime_request(
        &self,
        command: RespondToRuntimeRequestCommand,
    ) -> Result<(), AgentSessionApplicationError> {
        let lane = self.interaction_lanes.lane(&command.invocation_id)?;
        let _guard = lane.lock().map_err(|_| {
            AgentSessionApplicationError::conflict("Interaction lane is unavailable")
        })?;
        self.require_active_interaction(&command.session_id, &command.invocation_id)?;
        if self
            .repository
            .preparation(&command.invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .is_some_and(|p| !p.delivery_started)
        {
            return Err(AgentSessionApplicationError::conflict(
                "The submitted prompt is still preparing",
            ));
        }

        let pending = self
            .session_interactions(&command.session_id)?
            .into_iter()
            .find(|i| {
                i.id == command.request_id
                    && i.invocation_id == command.invocation_id
                    && i.kind == "request"
            });
        if !pending.is_some_and(|i| i.state == "pending") {
            return Err(AgentSessionApplicationError::conflict(
                "Runtime request is no longer pending",
            ));
        }
        // Persist intent before writing, so restart/retry cannot silently duplicate a response.
        self.record_interaction(
            &command.invocation_id,
            RuntimeControlRecord::RequestResponse {
                id: command.request_id.clone(),
                state: "responding".into(),
                message: None,
            },
        )?;
        let result = self
            .runtime_for_invocation(&command.invocation_id)?
            .respond(
                &command.invocation_id,
                &command.request_id,
                command.response,
            );
        let state = match &result {
            Ok(()) => "answered",
            Err(e) if e.kind == RuntimePortErrorKind::UnsupportedOptions => "pending",
            Err(_) => "uncertain",
        };
        self.record_interaction(
            &command.invocation_id,
            RuntimeControlRecord::RequestResponse {
                id: command.request_id.clone(),
                state: state.into(),
                message: result.as_ref().err().map(|e| e.message.clone()),
            },
        )?;
        result.map_err(AgentSessionApplicationError::runtime)
    }

    fn require_active_interaction(
        &self,
        session: &AgentSessionId,
        id: &AgentInvocationId,
    ) -> Result<(), AgentSessionApplicationError> {
        let invocation = self
            .repository
            .get_invocation(id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Invocation not found"))?;
        if invocation.session_id != *session || !invocation.status.is_active() {
            return Err(AgentSessionApplicationError::conflict(
                "The targeted invocation is no longer active in this session",
            ));
        }
        Ok(())
    }

    fn record_interaction(
        &self,
        id: &AgentInvocationId,
        record: RuntimeControlRecord,
    ) -> Result<(), AgentSessionApplicationError> {
        PersistedRuntimeUpdateSink::new(
            self.repository.clone(),
            self.notifier.clone(),
            self.clock.clone(),
            self.ids.clone(),
            self.update_lanes.clone(),
        )
        .emit_update(id, RuntimeUpdate::Event(record.into_draft()))
        .map_err(AgentSessionApplicationError::runtime)
    }
}
