use super::{
    application::{
        launch_extension, BindWorkflowSessionHarness, WorkflowRepository,
        WorkflowSessionHarnessBinder,
    },
    domain::{
        EffectiveWorkflowNodeConfig, WorkflowCompletedTurnTrigger, WorkflowReceiverSessionPolicy,
    },
};
use crate::agent_sessions::{
    application::{
        AgentSessionApplication, CreateAgentSessionCommand, CreateApplicationAgentSessionCommand,
        SendAgentSessionMessageCommand, SendAgentSessionMessageResult,
        SendIdempotentApplicationAgentSessionMessageCommand,
    },
    domain::{AgentRuntimeOptions, AgentSessionId},
};
use chrono::Utc;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

pub(crate) struct WorkflowHumanMessage {
    pub(crate) submitted_text: String,
    pub(crate) title: Option<String>,
}

pub(crate) struct WorkflowNodeSessions {
    repository: Arc<dyn WorkflowRepository>,
    sessions: Arc<AgentSessionApplication>,
    harnesses: Arc<dyn WorkflowSessionHarnessBinder>,
    receiver_lanes: Mutex<HashMap<(String, String), Arc<Mutex<()>>>>,
}

impl WorkflowNodeSessions {
    pub(crate) fn new(
        repository: Arc<dyn WorkflowRepository>,
        sessions: Arc<AgentSessionApplication>,
        harnesses: Arc<dyn WorkflowSessionHarnessBinder>,
    ) -> Self {
        Self {
            repository,
            sessions,
            harnesses,
            receiver_lanes: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn send_human_message(
        &self,
        workflow_instance_id: &str,
        node_id: &str,
        message: WorkflowHumanMessage,
    ) -> Result<SendAgentSessionMessageResult, String> {
        let instance = self
            .repository
            .load_workflow_instance(workflow_instance_id)?;
        let node = instance
            .recipe
            .nodes
            .iter()
            .find(|candidate| candidate.id == node_id)
            .cloned()
            .ok_or_else(|| "The Workflow node is absent from the instance recipe.".to_string())?;
        self.deliver(Delivery {
            workflow_instance_id,
            recipe_id: &instance.recipe.id,
            node: &node,
            worktree_root: &instance.target.worktree.path,
            prompt: message.submitted_text,
            title: message.title,
            connection_activation_id: None,
            receiver_session_policy: None,
        })
        .map_err(|(_, reason)| reason)
    }

    pub(crate) fn deliver_connection_message(
        &self,
        activation_id: &str,
        trigger: &WorkflowCompletedTurnTrigger,
        receiver_node_id: &str,
        receiver_session_policy: WorkflowReceiverSessionPolicy,
        prompt: String,
    ) -> Result<(), (&'static str, String)> {
        let node = trigger
            .recipe
            .nodes
            .iter()
            .find(|candidate| candidate.id == receiver_node_id)
            .ok_or_else(|| {
                (
                    "receiver_resolution",
                    "The receiver node is absent from the activated recipe.".to_string(),
                )
            })?;
        self.deliver(Delivery {
            workflow_instance_id: &trigger.workflow_instance_id,
            recipe_id: &trigger.recipe.id,
            node,
            worktree_root: &trigger.worktree_root,
            prompt,
            title: None,
            connection_activation_id: Some(activation_id),
            receiver_session_policy: Some(receiver_session_policy),
        })?;
        Ok(())
    }

    fn deliver(
        &self,
        request: Delivery<'_>,
    ) -> Result<SendAgentSessionMessageResult, (&'static str, String)> {
        let requested_options = AgentRuntimeOptions {
            model: nonempty(request.node.harness.default_model()),
            sandbox: None,
        };
        let extension = launch_extension(&request.node.harness)
            .map_err(|reason| ("receiver_harness", reason))?;
        let lane = self
            .receiver_lane(request.workflow_instance_id, &request.node.id)
            .map_err(|reason| ("receiver_session_resolution", reason))?;
        let guard = lane.lock().map_err(|_| {
            (
                "receiver_session_resolution",
                "The receiver Session lane is unavailable.".to_string(),
            )
        })?;
        let existing = match request.receiver_session_policy {
            Some(WorkflowReceiverSessionPolicy::ContinueLatest) => self
                .most_recent_session(request.workflow_instance_id, &request.node.id)
                .map_err(|reason| ("receiver_session_resolution", reason))?,
            Some(WorkflowReceiverSessionPolicy::Fresh) | None => None,
        };
        let create_association = existing.is_none();
        let session = match existing {
            Some(session) => session,
            None => AgentSessionId::new(format!("workflow-session-{}", Uuid::new_v4()))
                .map_err(|error| ("session_creation", error.to_string()))?,
        };
        let invocation = self.sessions.allocate_application_invocation_id();
        if let Some(activation_id) = request.connection_activation_id {
            self.repository
                .reserve_connection_activation_target(
                    activation_id,
                    request.workflow_instance_id,
                    &request.node.id,
                    session.as_str(),
                    invocation.as_str(),
                    if create_association {
                        "fresh"
                    } else {
                        "continued"
                    },
                )
                .map_err(|reason| ("target_reservation", reason))?;
        }
        if create_association {
            self.sessions
                .create_application_session(CreateApplicationAgentSessionCommand {
                    session_id: session.clone(),
                    session: CreateAgentSessionCommand {
                        title: Some(
                            request
                                .title
                                .filter(|value| !value.trim().is_empty())
                                .unwrap_or_else(|| request.node.harness.name().to_string()),
                        ),
                        working_directory: Some(request.worktree_root.to_string()),
                        requested_options: requested_options.clone(),
                    },
                })
                .map_err(|error| ("session_creation", error.to_string()))?;
            self.harnesses
                .bind_workflow_session(BindWorkflowSessionHarness {
                    session_id: session.clone(),
                    runtime_instance_id: invocation.as_str().to_string(),
                    workflow_instance_id: request.workflow_instance_id.to_string(),
                    recipe_id: request.recipe_id.to_string(),
                    node_id: request.node.id.clone(),
                    harness: request.node.harness.clone(),
                })
                .map_err(|reason| ("harness_binding", reason))?;
        }
        let associated_at = Utc::now().to_rfc3339();
        if let Some(activation_id) = request.connection_activation_id {
            self.repository
                .associate_connection_activation_session(
                    activation_id,
                    request.workflow_instance_id,
                    &request.node.id,
                    session.as_str(),
                    &associated_at,
                    create_association,
                )
                .map_err(|reason| ("session_association", reason))?;
            self.repository
                .mark_connection_activation_launch_requested(
                    activation_id,
                    &Utc::now().to_rfc3339(),
                )
                .map_err(|reason| ("launch_request_recording", reason))?;
        } else {
            self.repository
                .associate_instance_session(
                    request.workflow_instance_id,
                    &request.node.id,
                    session.as_str(),
                    &associated_at,
                )
                .map_err(|reason| ("session_association", reason))?;
        }
        // A fake/runtime may synchronously report a terminal turn that routes back to this node.
        drop(guard);
        let command = SendIdempotentApplicationAgentSessionMessageCommand {
            invocation_id: invocation,
            message: SendAgentSessionMessageCommand {
                session_id: Some(session),
                submitted_text: request.prompt,
                title: None,
                working_directory: Some(request.worktree_root.to_string()),
                requested_options: Some(requested_options),
            },
        };
        let launch = if request.connection_activation_id.is_some() {
            self.sessions
                .send_idempotent_application_message_with_launch_observation(command, extension)
        } else {
            self.sessions
                .send_idempotent_user_message_with_launch_observation(command, extension)
        }
        .map_err(|error| ("runtime_launch", error.to_string()))?;
        if let Some(activation_id) = request.connection_activation_id {
            if !launch.launch_accepted {
                return Err((
                    "runtime_launch",
                    "The Agent Session launch was not accepted.".to_string(),
                ));
            }
            self.repository
                .mark_connection_activation_launch_accepted(activation_id, &Utc::now().to_rfc3339())
                .map_err(|reason| ("launch_acceptance_recording", reason))?;
        }
        Ok(launch.acknowledgement)
    }

    fn receiver_lane(
        &self,
        workflow_instance_id: &str,
        node_id: &str,
    ) -> Result<Arc<Mutex<()>>, String> {
        let mut lanes = self
            .receiver_lanes
            .lock()
            .map_err(|_| "Workflow receiver Session lanes are unavailable.".to_string())?;
        Ok(lanes
            .entry((workflow_instance_id.to_string(), node_id.to_string()))
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone())
    }

    fn most_recent_session(
        &self,
        workflow_instance_id: &str,
        node_id: &str,
    ) -> Result<Option<AgentSessionId>, String> {
        let instance = self
            .repository
            .load_workflow_instance(workflow_instance_id)?;
        let mut candidates = Vec::new();
        for association in instance
            .session_associations
            .iter()
            .filter(|association| association.node_id == node_id)
        {
            let session_id = AgentSessionId::new(association.session_id.clone())
                .map_err(|error| error.to_string())?;
            let history = self
                .sessions
                .load_session(&session_id)
                .map_err(|error| error.to_string())?;
            let last_active_at = history
                .invocations
                .iter()
                .map(|entry| entry.invocation.updated_at)
                .max()
                .unwrap_or(history.session.updated_at);
            candidates.push((
                last_active_at,
                association.associated_at.clone(),
                session_id,
            ));
        }
        candidates.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| left.2.as_str().cmp(right.2.as_str()))
        });
        Ok(candidates.into_iter().next().map(|(_, _, session)| session))
    }
}

struct Delivery<'a> {
    workflow_instance_id: &'a str,
    recipe_id: &'a str,
    node: &'a EffectiveWorkflowNodeConfig,
    worktree_root: &'a str,
    prompt: String,
    title: Option<String>,
    connection_activation_id: Option<&'a str>,
    receiver_session_policy: Option<WorkflowReceiverSessionPolicy>,
}

fn nonempty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_string())
}
