use super::{
    AgentSessionApplication, AgentSessionApplicationError, SessionInteraction,
    SteerAgentSessionCommand,
};
use crate::agent_sessions::{
    domain::{AgentInvocationId, AgentSessionId, NormalizedRuntimeEventKind, ToolActivityPhase},
    ports::{
        AgentRuntimeUpdateSink, RuntimePortError, RuntimeUpdate, RuntimeUpdateDeliveryFailure,
    },
};
use orchid_engine::contracts::RuntimeControlRecord;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(super) struct PendingSteering(Mutex<HashMap<AgentInvocationId, Vec<SteerAgentSessionCommand>>>);
impl PendingSteering {
    fn put(&self, command: SteerAgentSessionCommand) {
        let mut queues = self.0.lock().expect("steering queue");
        let items = queues.entry(command.invocation_id.clone()).or_default();
        items.retain(|c| c.input_id != command.input_id);
        items.push(command);
    }
    fn remove(&self, invocation: &AgentInvocationId, input: &str) {
        if let Some(items) = self.0.lock().expect("steering queue").get_mut(invocation) {
            items.retain(|c| c.input_id != input);
        }
    }
    fn take(&self, invocation: &AgentInvocationId) -> Vec<SteerAgentSessionCommand> {
        self.0
            .lock()
            .expect("steering queue")
            .remove(invocation)
            .unwrap_or_default()
    }
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditSteeringCommand {
    pub session_id: AgentSessionId,
    pub invocation_id: AgentInvocationId,
    pub input_id: String,
    pub action: SteeringEditAction,
    pub text: Option<String>,
}
#[derive(Clone, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SteeringEditAction {
    Begin,
    Save,
    Resume,
    Cancel,
}

impl AgentSessionApplication {
    pub fn steer_session(
        &self,
        command: SteerAgentSessionCommand,
    ) -> Result<SessionInteraction, AgentSessionApplicationError> {
        if let Some(result) = self
            .route_owned_command(
                &command.session_id,
                crate::agent_sessions::client::HostCommand {
                    method: "steer".into(),
                    input: serde_json::to_value(&command)
                        .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?,
                },
            )
            .map_err(AgentSessionApplicationError::invalid)?
        {
            return serde_json::from_value(result)
                .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()));
        }
        let lane = self.interaction_lanes.lane(&command.invocation_id)?;
        let _guard = lane
            .lock()
            .map_err(|_| AgentSessionApplicationError::conflict("Steering unavailable"))?;
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
        if command.input_id.trim().is_empty()
            || (command.text.trim().is_empty() && command.attachments.is_empty())
        {
            return Err(AgentSessionApplicationError::invalid(
                "Steering requires text or images",
            ));
        }
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
        self.queue_steering(command)
    }
    fn queue_steering(
        &self,
        command: SteerAgentSessionCommand,
    ) -> Result<SessionInteraction, AgentSessionApplicationError> {
        self.record_interaction(
            &command.invocation_id,
            RuntimeControlRecord::SteeringQueued {
                input_id: command.input_id.clone(),
                text: command.text.clone(),
                attachments: command.attachments.clone(),
            },
        )?;
        self.steering_queue.put(command.clone());
        self.session_interactions(&command.session_id)?
            .into_iter()
            .find(|i| i.id == command.input_id)
            .ok_or_else(|| AgentSessionApplicationError::not_found("Steering disappeared"))
    }
    pub fn edit_steering(
        &self,
        input: EditSteeringCommand,
    ) -> Result<SessionInteraction, AgentSessionApplicationError> {
        let lane = self.interaction_lanes.lane(&input.invocation_id)?;
        let _guard = lane
            .lock()
            .map_err(|_| AgentSessionApplicationError::conflict("Steering unavailable"))?;
        let existing = self
            .session_interactions(&input.session_id)?
            .into_iter()
            .find(|i| i.id == input.input_id && i.invocation_id == input.invocation_id)
            .ok_or_else(|| AgentSessionApplicationError::not_found("Steering not found"))?;
        if !matches!(existing.state.as_str(), "queued" | "editing" | "expired") {
            return Err(AgentSessionApplicationError::conflict(
                "Steering has already been handed to the agent",
            ));
        }
        match input.action {
            SteeringEditAction::Begin | SteeringEditAction::Cancel => {
                self.steering_queue
                    .remove(&input.invocation_id, &input.input_id);
                self.record_interaction(
                    &input.invocation_id,
                    RuntimeControlRecord::SteeringResult {
                        id: input.input_id.clone(),
                        state: if matches!(input.action, SteeringEditAction::Begin) {
                            "editing"
                        } else {
                            "canceled"
                        }
                        .into(),
                        message: None,
                    },
                )?;
            }
            SteeringEditAction::Save | SteeringEditAction::Resume => {
                self.require_active_interaction(&input.session_id, &input.invocation_id)?;
                let attachments: Vec<orchid_engine::contracts::runtime::RuntimeImageAttachment> =
                    serde_json::from_value(existing.content["attachments"].clone())
                        .unwrap_or_default();
                let text = input.text.unwrap_or_else(|| {
                    existing.content["text"].as_str().unwrap_or_default().into()
                });
                if text.trim().is_empty() && attachments.is_empty() {
                    return Err(AgentSessionApplicationError::invalid(
                        "Steering requires text or images",
                    ));
                }
                return self.queue_steering(SteerAgentSessionCommand {
                    session_id: input.session_id,
                    invocation_id: input.invocation_id,
                    input_id: input.input_id,
                    text,
                    attachments,
                });
            }
        }
        self.session_interactions(&input.session_id)?
            .into_iter()
            .find(|i| i.id == input.input_id)
            .ok_or_else(|| AgentSessionApplicationError::not_found("Steering disappeared"))
    }
    pub(super) fn steering_boundary(&self, invocation: &AgentInvocationId, terminal: bool) {
        let commands = self.steering_queue.take(invocation);
        if commands.is_empty() {
            return;
        }
        let app = self.clone();
        self.workers.spawn_blocking(move || {
            for command in commands {
                if terminal {
                    let _ = app.record_interaction(
                        &command.invocation_id,
                        RuntimeControlRecord::SteeringResult {
                            id: command.input_id,
                            state: "expired".into(),
                            message: Some(
                                "The turn finished before this input was delivered".into(),
                            ),
                        },
                    );
                } else {
                    if let Err(error) = app.deliver_steering(command.clone()) {
                        let _ = app.record_interaction(
                            &command.invocation_id,
                            RuntimeControlRecord::SteeringResult {
                                id: command.input_id,
                                state: "expired".into(),
                                message: Some(error.to_string()),
                            },
                        );
                    }
                }
            }
        });
    }
}

pub(super) struct SteeringRuntimeUpdateSink {
    inner: Arc<dyn AgentRuntimeUpdateSink>,
    app: AgentSessionApplication,
}
impl SteeringRuntimeUpdateSink {
    pub(super) fn new(
        inner: Arc<dyn AgentRuntimeUpdateSink>,
        app: AgentSessionApplication,
    ) -> Self {
        Self { inner, app }
    }
}
impl AgentRuntimeUpdateSink for SteeringRuntimeUpdateSink {
    fn emit_update(
        &self,
        invocation: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        let terminal = matches!(update, RuntimeUpdate::Finished(_));
        let boundary = match &update {
            RuntimeUpdate::Event(event) => event.normalized.as_ref().is_some_and(|n| {
                n.tool_activity
                    .as_ref()
                    .is_some_and(|t| t.phase == ToolActivityPhase::Completed)
                    || (n.kind == NormalizedRuntimeEventKind::AgentMessage
                        && n.agent_message_role() == Some("intermediate"))
            }),
            _ => false,
        };
        let result = self.inner.emit_update(invocation, update);
        if result.is_ok() && (boundary || terminal) {
            self.app.steering_boundary(invocation, terminal);
        }
        result
    }
    fn report_delivery_failure(
        &self,
        invocation: &AgentInvocationId,
        failure: RuntimeUpdateDeliveryFailure,
    ) {
        self.inner.report_delivery_failure(invocation, failure);
    }
}
