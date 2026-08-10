use super::domain::{
    PreparedWorkflowMcpHandoff, WorkflowCompletedTurnTrigger, WorkflowConnectionActivation,
    WorkflowConnectionActivationPreparation, WorkflowConnectionActivationRecord,
    WorkflowConnectionActivationStatus, WorkflowConnectionConfig, WorkflowDefinition,
    WorkflowElementRef, WorkflowHarnessConfig, WorkflowInstance, WorkflowInstanceRecord,
    WorkflowInstanceSession, WorkflowInstanceSummary, WorkflowInvocation,
    WorkflowLaunchPreparation, WorkflowMcpActivationContext, WorkflowMcpComponent,
    WorkflowMcpOutput, WorkflowNativeQuery, WorkflowNodeConfig, WorkflowRole,
    WorkflowSessionActivity, WorkflowTypeSummary,
};
use crate::agent_sessions::{
    application::{
        AgentSessionApplication, AgentSessionNotification, CreateAgentSessionCommand,
        CreateApplicationAgentSessionCommand, SendAgentSessionMessageCommand,
        SendIdempotentApplicationAgentSessionMessageCommand,
    },
    domain::{
        AgentInvocation, AgentInvocationId, AgentInvocationStatus, AgentRuntimeOptions,
        AgentSessionId, NormalizedRuntimeEventKind,
    },
    ports::{InitialPromptPrefix, RuntimeLaunchExtension},
};
use chrono::Utc;
use globset::Glob;
use regex::Regex;
use std::{
    cmp::Reverse,
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};
use uuid::Uuid;

const SUPPORTED_CODEX_MODELS: [&str; 2] = ["gpt-5.6-sol", "gpt-5.6-terra"];

pub(crate) trait WorkflowRepository: Send + Sync {
    fn list_workflow_types(&self) -> Result<Vec<WorkflowTypeSummary>, String>;
    fn list_roles(&self) -> Result<Vec<WorkflowRole>, String>;
    fn create_role(
        &self,
        name: &str,
        harness: WorkflowHarnessConfig,
    ) -> Result<WorkflowRole, String>;
    fn update_role(
        &self,
        role_id: &str,
        name: &str,
        harness: WorkflowHarnessConfig,
    ) -> Result<WorkflowRole, String>;
    fn create_workflow_type(&self, name: &str) -> Result<WorkflowDefinition, String>;
    fn load_workflow_type(&self, workflow_type_id: &str) -> Result<WorkflowDefinition, String>;
    fn update_workflow_type(
        &self,
        workflow_type_id: &str,
        name: &str,
    ) -> Result<WorkflowDefinition, String>;
    fn save_node_draft(
        &self,
        workflow_type_id: &str,
        node: WorkflowNodeConfig,
    ) -> Result<WorkflowDefinition, String>;
    fn delete_node_draft(
        &self,
        workflow_type_id: &str,
        node_id: &str,
    ) -> Result<WorkflowDefinition, String>;
    fn detach_node_role(
        &self,
        workflow_type_id: &str,
        node_id: &str,
    ) -> Result<WorkflowDefinition, String>;
    fn save_node_as_role(
        &self,
        workflow_type_id: &str,
        node_id: &str,
        role_name: &str,
    ) -> Result<WorkflowDefinition, String>;
    fn save_connection_draft(
        &self,
        workflow_type_id: &str,
        connection: WorkflowConnectionConfig,
    ) -> Result<WorkflowDefinition, String>;
    fn delete_connection_draft(
        &self,
        workflow_type_id: &str,
        connection_id: &str,
    ) -> Result<WorkflowDefinition, String>;
    fn activate_changes(
        &self,
        workflow_type_id: &str,
        elements: &[WorkflowElementRef],
    ) -> Result<WorkflowDefinition, String>;
    fn native_query(&self) -> Result<WorkflowNativeQuery, String>;
    fn create_instance_launch(
        &self,
        preparation: WorkflowLaunchPreparation,
    ) -> Result<WorkflowInstanceRecord, String>;
    fn associate_instance_session(
        &self,
        activation_id: &str,
        workflow_instance_id: &str,
        node_id: &str,
        session_id: &str,
        associated_at: &str,
    ) -> Result<WorkflowInstanceRecord, String>;
    fn mark_instance_launch_requested(
        &self,
        activation_id: &str,
        requested_at: &str,
    ) -> Result<(), String>;
    fn mark_instance_launch_accepted(
        &self,
        activation_id: &str,
        accepted_at: &str,
    ) -> Result<(), String>;
    fn mark_instance_launch_failed(
        &self,
        activation_id: &str,
        stage: &str,
        reason: &str,
        failed_at: &str,
    ) -> Result<(), String>;
    fn list_workflow_instances(&self) -> Result<Vec<WorkflowInstanceRecord>, String>;
    fn load_workflow_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstanceRecord, String>;
    fn load_completed_turn_trigger(
        &self,
        source_session_id: &str,
    ) -> Result<Option<WorkflowCompletedTurnTrigger>, String>;
    fn load_mcp_prepared_trigger(
        &self,
        workflow_instance_id: &str,
        recipe_id: &str,
        sender_node_id: &str,
        source_session_id: &str,
    ) -> Result<WorkflowCompletedTurnTrigger, String>;
    fn create_connection_activation(
        &self,
        preparation: WorkflowConnectionActivationPreparation,
    ) -> Result<(), String>;
    fn mark_connection_activation_resolved(
        &self,
        activation_id: &str,
        relative_file_path: &str,
        resolved_at: &str,
    ) -> Result<(), String>;
    fn mark_mcp_connection_activation_resolved(
        &self,
        activation_id: &str,
        resolved_output_json: &str,
        resolved_at: &str,
    ) -> Result<(), String>;
    fn reserve_connection_activation_target(
        &self,
        activation_id: &str,
        workflow_instance_id: &str,
        node_id: &str,
        session_id: &str,
        invocation_id: &str,
        session_mode: &str,
    ) -> Result<(), String>;
    fn associate_connection_activation_session(
        &self,
        activation_id: &str,
        workflow_instance_id: &str,
        node_id: &str,
        session_id: &str,
        associated_at: &str,
        create_association: bool,
    ) -> Result<(), String>;
    fn mark_connection_activation_launch_requested(
        &self,
        activation_id: &str,
        requested_at: &str,
    ) -> Result<(), String>;
    fn mark_connection_activation_launch_accepted(
        &self,
        activation_id: &str,
        accepted_at: &str,
    ) -> Result<(), String>;
    fn mark_connection_activation_failed(
        &self,
        activation_id: &str,
        stage: &str,
        reason: &str,
        failed_at: &str,
    ) -> Result<(), String>;
    fn list_connection_activations(
        &self,
        workflow_instance_id: &str,
    ) -> Result<Vec<WorkflowConnectionActivationRecord>, String>;
}

#[derive(Clone, Debug)]
pub(crate) struct BindWorkflowSessionHarness {
    pub(crate) session_id: AgentSessionId,
    pub(crate) runtime_instance_id: String,
    pub(crate) workflow_instance_id: String,
    pub(crate) recipe_id: String,
    pub(crate) node_id: String,
    pub(crate) harness: WorkflowHarnessConfig,
}

pub(crate) trait WorkflowSessionHarnessBinder: Send + Sync {
    fn bind_workflow_session(&self, request: BindWorkflowSessionHarness) -> Result<(), String>;
}

pub(crate) struct WorkflowApplication {
    repository: Arc<dyn WorkflowRepository>,
    sessions: Arc<AgentSessionApplication>,
    harnesses: Arc<dyn WorkflowSessionHarnessBinder>,
    instance_root: PathBuf,
    receiver_lanes: Mutex<HashMap<(String, String), Arc<Mutex<()>>>>,
}

impl WorkflowApplication {
    pub(crate) fn new(
        repository: Arc<dyn WorkflowRepository>,
        sessions: Arc<AgentSessionApplication>,
        harnesses: Arc<dyn WorkflowSessionHarnessBinder>,
        instance_root: PathBuf,
    ) -> Self {
        Self {
            repository,
            sessions,
            harnesses,
            instance_root,
            receiver_lanes: Mutex::new(HashMap::new()),
        }
    }

    pub(crate) fn list_workflow_types(&self) -> Result<Vec<WorkflowTypeSummary>, String> {
        self.repository.list_workflow_types()
    }

    pub(crate) fn list_roles(&self) -> Result<Vec<WorkflowRole>, String> {
        self.repository.list_roles()
    }

    pub(crate) fn list_mcp_components(&self) -> Vec<WorkflowMcpComponent> {
        vec![super::mcp::component()]
    }

    pub(crate) fn create_role(
        &self,
        name: &str,
        harness: WorkflowHarnessConfig,
    ) -> Result<WorkflowRole, String> {
        self.repository.create_role(name, harness)
    }

    pub(crate) fn update_role(
        &self,
        role_id: &str,
        name: &str,
        harness: WorkflowHarnessConfig,
    ) -> Result<WorkflowRole, String> {
        self.repository.update_role(role_id, name, harness)
    }

    pub(crate) fn create_workflow_type(&self, name: &str) -> Result<WorkflowDefinition, String> {
        self.repository.create_workflow_type(name)
    }

    pub(crate) fn load_workflow_type(
        &self,
        workflow_type_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository.load_workflow_type(workflow_type_id)
    }

    pub(crate) fn update_workflow_type(
        &self,
        workflow_type_id: &str,
        name: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository.update_workflow_type(workflow_type_id, name)
    }

    pub(crate) fn save_node_draft(
        &self,
        workflow_type_id: &str,
        node: WorkflowNodeConfig,
    ) -> Result<WorkflowDefinition, String> {
        self.repository.save_node_draft(workflow_type_id, node)
    }

    pub(crate) fn delete_node_draft(
        &self,
        workflow_type_id: &str,
        node_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository.delete_node_draft(workflow_type_id, node_id)
    }

    pub(crate) fn detach_node_role(
        &self,
        workflow_type_id: &str,
        node_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository.detach_node_role(workflow_type_id, node_id)
    }

    pub(crate) fn save_node_as_role(
        &self,
        workflow_type_id: &str,
        node_id: &str,
        role_name: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository
            .save_node_as_role(workflow_type_id, node_id, role_name)
    }

    pub(crate) fn save_connection_draft(
        &self,
        workflow_type_id: &str,
        connection: WorkflowConnectionConfig,
    ) -> Result<WorkflowDefinition, String> {
        self.repository
            .save_connection_draft(workflow_type_id, connection)
    }

    pub(crate) fn delete_connection_draft(
        &self,
        workflow_type_id: &str,
        connection_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        self.repository
            .delete_connection_draft(workflow_type_id, connection_id)
    }

    pub(crate) fn activate_changes(
        &self,
        workflow_type_id: &str,
        elements: &[WorkflowElementRef],
    ) -> Result<WorkflowDefinition, String> {
        self.repository.activate_changes(workflow_type_id, elements)
    }

    pub(crate) fn native_query(&self) -> Result<WorkflowNativeQuery, String> {
        self.repository.native_query()
    }

    pub(crate) fn launch_workflow_instance(
        &self,
        workflow_type_id: &str,
        name: Option<&str>,
        starting_prompt: &str,
    ) -> Result<WorkflowInstance, String> {
        if starting_prompt.trim().is_empty() {
            return Err("A starting prompt is required.".to_string());
        }
        let definition = self.repository.load_workflow_type(workflow_type_id)?;
        let recipe = definition
            .active_recipe
            .ok_or_else(|| "Activate the Workflow type before launching it.".to_string())?;
        let starts = recipe
            .nodes
            .iter()
            .filter(|node| node.is_starting_point)
            .collect::<Vec<_>>();
        if starts.len() != 1 {
            return Err(format!(
                "The active recipe must contain exactly one starting node; found {}.",
                starts.len()
            ));
        }
        let start = starts[0];
        let launch_extension = launch_extension(&start.harness)?;
        let requested_options = AgentRuntimeOptions {
            model: nonempty(start.harness.default_model()),
            sandbox: None,
        };
        let instance_id = format!("workflow-instance-{}", Uuid::new_v4());
        let session_id = format!("workflow-session-{}", Uuid::new_v4());
        let invocation_id = format!("workflow-invocation-{}", Uuid::new_v4());
        let activation_id = format!("workflow-activation-{}", Uuid::new_v4());
        let instance_name = name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| {
                crate::agent_sessions::application::title_from_message(starting_prompt)
            });
        let working_directory = self.instance_root.join(&instance_id);
        fs::create_dir_all(&working_directory)
            .map_err(|error| format!("Unable to create Workflow instance directory: {error}"))?;
        let working_directory = working_directory.to_string_lossy().into_owned();
        let requested_at = Utc::now().to_rfc3339();
        let preparation = WorkflowLaunchPreparation {
            instance_id: instance_id.clone(),
            workflow_type_id: workflow_type_id.to_string(),
            recipe_id: recipe.id.clone(),
            name: instance_name.clone(),
            starting_prompt: starting_prompt.to_string(),
            working_directory: working_directory.clone(),
            activation_id: activation_id.clone(),
            target_node_id: start.id.clone(),
            target_session_id: session_id.clone(),
            target_invocation_id: invocation_id.clone(),
            requested_at,
        };
        self.repository.create_instance_launch(preparation)?;

        let session = AgentSessionId::new(session_id.clone()).map_err(|error| error.to_string())?;
        if let Err(error) =
            self.sessions
                .create_application_session(CreateApplicationAgentSessionCommand {
                    session_id: session.clone(),
                    session: CreateAgentSessionCommand {
                        title: Some(instance_name),
                        working_directory: Some(working_directory.clone()),
                        requested_options: requested_options.clone(),
                    },
                })
        {
            self.record_failure(&activation_id, "session_creation", &error.to_string());
            return self.load_workflow_instance(&instance_id);
        }
        let associated_at = Utc::now().to_rfc3339();
        if let Err(error) = self.repository.associate_instance_session(
            &activation_id,
            &instance_id,
            &start.id,
            &session_id,
            &associated_at,
        ) {
            self.record_failure(&activation_id, "session_association", &error);
            return self.load_workflow_instance(&instance_id);
        }
        if let Err(error) = self
            .harnesses
            .bind_workflow_session(BindWorkflowSessionHarness {
                session_id: session.clone(),
                runtime_instance_id: invocation_id.clone(),
                workflow_instance_id: instance_id.clone(),
                recipe_id: recipe.id.clone(),
                node_id: start.id.clone(),
                harness: start.harness.clone(),
            })
        {
            self.record_failure(&activation_id, "harness_binding", &error);
            return self.load_workflow_instance(&instance_id);
        }
        if let Err(error) = self
            .repository
            .mark_instance_launch_requested(&activation_id, &Utc::now().to_rfc3339())
        {
            self.record_failure(&activation_id, "launch_request_recording", &error);
            return self.load_workflow_instance(&instance_id);
        }
        let invocation =
            AgentInvocationId::new(invocation_id).map_err(|error| error.to_string())?;
        let launch = self
            .sessions
            .send_idempotent_user_message_with_launch_observation(
                SendIdempotentApplicationAgentSessionMessageCommand {
                    invocation_id: invocation,
                    message: SendAgentSessionMessageCommand {
                        session_id: Some(session),
                        submitted_text: starting_prompt.to_string(),
                        title: None,
                        working_directory: Some(working_directory),
                        requested_options: Some(requested_options),
                    },
                },
                launch_extension,
            );
        match launch {
            Ok(result) if result.launch_accepted => self
                .repository
                .mark_instance_launch_accepted(&activation_id, &Utc::now().to_rfc3339())?,
            Ok(_) => {
                self.record_failure(
                    &activation_id,
                    "runtime_launch",
                    "The Agent Session launch was not accepted.",
                );
            }
            Err(error) => {
                self.record_failure(&activation_id, "runtime_launch", &error.to_string());
            }
        }
        self.load_workflow_instance(&instance_id)
    }

    pub(crate) fn on_agent_notification(
        &self,
        notification: &AgentSessionNotification,
    ) -> Result<usize, String> {
        let AgentSessionNotification::InvocationTerminal {
            session_id,
            invocation,
        } = notification
        else {
            return Ok(0);
        };
        if invocation.status != AgentInvocationStatus::Completed {
            return Ok(0);
        }
        self.execute_completed_turn_connections(session_id, invocation)
    }

    fn execute_completed_turn_connections(
        &self,
        source_session_id: &AgentSessionId,
        source_invocation: &AgentInvocation,
    ) -> Result<usize, String> {
        let Some(trigger) = self
            .repository
            .load_completed_turn_trigger(source_session_id.as_str())?
        else {
            return Ok(0);
        };
        let connections = trigger
            .recipe
            .connections
            .iter()
            .filter(|connection| connection.sender_node_id == trigger.sender_node_id)
            .filter(|connection| {
                matches!(
                    connection.mechanism,
                    Some(
                        super::domain::WorkflowConnectionMechanism::TurnFinishedExpectedFile { .. }
                    )
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        if connections.is_empty() {
            return Ok(0);
        }
        let connection_count = connections.len();
        let final_output = self.final_output_for(source_session_id, &source_invocation.id)?;
        std::thread::scope(|scope| {
            let mut executions = Vec::with_capacity(connections.len());
            for connection in connections {
                let trigger = trigger.clone();
                let source_session_id = source_session_id.as_str().to_string();
                let source_invocation_id = source_invocation.id.as_str().to_string();
                let final_output = final_output.clone();
                executions.push(scope.spawn(move || {
                    self.execute_connection(
                        trigger,
                        connection,
                        &source_session_id,
                        &source_invocation_id,
                        final_output.as_deref(),
                    )
                }));
            }
            for execution in executions {
                let _ = execution.join();
            }
        });
        Ok(connection_count)
    }

    fn final_output_for(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<String>, String> {
        let history = self
            .sessions
            .load_session(session_id)
            .map_err(|error| error.to_string())?;
        Ok(history
            .invocations
            .iter()
            .find(|entry| entry.invocation.id == *invocation_id)
            .and_then(|entry| {
                entry.events.iter().rev().find_map(|event| {
                    let normalized = event.normalized.as_ref()?;
                    (normalized.kind == NormalizedRuntimeEventKind::AgentMessage
                        && normalized
                            .details
                            .as_ref()
                            .and_then(|details| details.get("role"))
                            .and_then(|role| role.as_str())
                            == Some("final"))
                    .then(|| normalized.text.clone())
                    .flatten()
                })
            }))
    }

    fn execute_connection(
        &self,
        trigger: WorkflowCompletedTurnTrigger,
        connection: WorkflowConnectionConfig,
        source_session_id: &str,
        source_invocation_id: &str,
        final_output: Option<&str>,
    ) {
        let Some(receiver_node_id) = connection.receiver_node_id.clone() else {
            return;
        };
        let activation_id = format!("workflow-connection-activation-{}", Uuid::new_v4());
        let requested_at = Utc::now().to_rfc3339();
        if self
            .repository
            .create_connection_activation(WorkflowConnectionActivationPreparation {
                id: activation_id.clone(),
                workflow_instance_id: trigger.workflow_instance_id.clone(),
                recipe_id: trigger.recipe.id.clone(),
                connection_id: connection.id.clone(),
                sender_node_id: trigger.sender_node_id.clone(),
                receiver_node_id: receiver_node_id.clone(),
                source_session_id: source_session_id.to_string(),
                source_invocation_id: source_invocation_id.to_string(),
                requested_at,
            })
            .is_err()
        {
            return;
        }
        let outcome = self.execute_connection_after_request(
            &activation_id,
            &trigger,
            &connection,
            &receiver_node_id,
            final_output,
        );
        if let Err((stage, reason)) = outcome {
            let _ = self.repository.mark_connection_activation_failed(
                &activation_id,
                stage,
                &reason,
                &Utc::now().to_rfc3339(),
            );
        }
    }

    fn execute_connection_after_request(
        &self,
        activation_id: &str,
        trigger: &WorkflowCompletedTurnTrigger,
        connection: &WorkflowConnectionConfig,
        receiver_node_id: &str,
        final_output: Option<&str>,
    ) -> Result<(), (&'static str, String)> {
        let mechanism = connection.mechanism.as_ref().ok_or_else(|| {
            (
                "file_resolution",
                "The connection has no mechanism.".to_string(),
            )
        })?;
        let super::domain::WorkflowConnectionMechanism::TurnFinishedExpectedFile {
            file_selector,
            description_text,
            prompt_text,
            ..
        } = mechanism
        else {
            return Err((
                "mechanism",
                "The connection is not a turn-finished file connection.".to_string(),
            ));
        };
        let relative_file_path = resolve_expected_file(
            Path::new(&trigger.working_directory),
            file_selector,
            final_output,
        )
        .map_err(|reason| ("file_resolution", reason))?;
        self.repository
            .mark_connection_activation_resolved(
                activation_id,
                &relative_file_path,
                &Utc::now().to_rfc3339(),
            )
            .map_err(|reason| ("resolution_recording", reason))?;
        let prompt = format!("{relative_file_path}\n{description_text}\n{prompt_text}");
        self.deliver_connection_prompt(activation_id, trigger, receiver_node_id, prompt)
    }

    fn deliver_connection_prompt(
        &self,
        activation_id: &str,
        trigger: &WorkflowCompletedTurnTrigger,
        receiver_node_id: &str,
        prompt: String,
    ) -> Result<(), (&'static str, String)> {
        let receiver = trigger
            .recipe
            .nodes
            .iter()
            .find(|node| node.id == receiver_node_id)
            .ok_or_else(|| {
                (
                    "receiver_resolution",
                    "The receiver node is absent from the activated recipe.".to_string(),
                )
            })?;
        let requested_options = AgentRuntimeOptions {
            model: nonempty(receiver.harness.default_model()),
            sandbox: None,
        };
        let launch_extension =
            launch_extension(&receiver.harness).map_err(|reason| ("receiver_harness", reason))?;
        let receiver_lane = self
            .receiver_lane(&trigger.workflow_instance_id, receiver_node_id)
            .map_err(|reason| ("receiver_session_resolution", reason))?;
        let receiver_guard = receiver_lane.lock().map_err(|_| {
            (
                "receiver_session_resolution",
                "The receiver Session lane is unavailable.".to_string(),
            )
        })?;
        let existing = self
            .most_recent_receiver_session(trigger, receiver_node_id)
            .map_err(|reason| ("receiver_session_resolution", reason))?;
        let (session, invocation, mode, create_association) = if let Some(session) = existing {
            (
                session,
                self.sessions.allocate_application_invocation_id(),
                "continued",
                false,
            )
        } else {
            let session = AgentSessionId::new(format!("workflow-session-{}", Uuid::new_v4()))
                .map_err(|error| ("session_creation", error.to_string()))?;
            let invocation = self.sessions.allocate_application_invocation_id();
            (session, invocation, "fresh", true)
        };
        self.repository
            .reserve_connection_activation_target(
                activation_id,
                &trigger.workflow_instance_id,
                receiver_node_id,
                session.as_str(),
                invocation.as_str(),
                mode,
            )
            .map_err(|reason| ("target_reservation", reason))?;
        if create_association {
            self.sessions
                .create_application_session(CreateApplicationAgentSessionCommand {
                    session_id: session.clone(),
                    session: CreateAgentSessionCommand {
                        title: Some(receiver.harness.name().to_string()),
                        working_directory: Some(trigger.working_directory.clone()),
                        requested_options: requested_options.clone(),
                    },
                })
                .map_err(|error| ("session_creation", error.to_string()))?;
            self.harnesses
                .bind_workflow_session(BindWorkflowSessionHarness {
                    session_id: session.clone(),
                    runtime_instance_id: invocation.as_str().to_string(),
                    workflow_instance_id: trigger.workflow_instance_id.clone(),
                    recipe_id: trigger.recipe.id.clone(),
                    node_id: receiver_node_id.to_string(),
                    harness: receiver.harness.clone(),
                })
                .map_err(|reason| ("harness_binding", reason))?;
        }
        let associated_at = Utc::now().to_rfc3339();
        self.repository
            .associate_connection_activation_session(
                activation_id,
                &trigger.workflow_instance_id,
                receiver_node_id,
                session.as_str(),
                &associated_at,
                create_association,
            )
            .map_err(|reason| ("session_association", reason))?;
        // The Session association is now unique and visible to competing edges. Do not retain the
        // lane while entering the runtime because a synchronous terminal callback may cycle back
        // to this receiver; the Agent Session repository rejects a second active invocation.
        drop(receiver_guard);
        self.repository
            .mark_connection_activation_launch_requested(activation_id, &Utc::now().to_rfc3339())
            .map_err(|reason| ("launch_request_recording", reason))?;
        let launch = self
            .sessions
            .send_idempotent_application_message_with_launch_observation(
                SendIdempotentApplicationAgentSessionMessageCommand {
                    invocation_id: invocation,
                    message: SendAgentSessionMessageCommand {
                        session_id: Some(session),
                        submitted_text: prompt,
                        title: None,
                        working_directory: Some(trigger.working_directory.clone()),
                        requested_options: Some(requested_options),
                    },
                },
                launch_extension,
            )
            .map_err(|error| ("runtime_launch", error.to_string()))?;
        if !launch.launch_accepted {
            return Err((
                "runtime_launch",
                "The Agent Session launch was not accepted.".to_string(),
            ));
        }
        self.repository
            .mark_connection_activation_launch_accepted(activation_id, &Utc::now().to_rfc3339())
            .map_err(|reason| ("launch_acceptance_recording", reason))?;
        Ok(())
    }

    pub(crate) fn prepare_mcp_native_handoff(
        &self,
        workflow_instance_id: &str,
        sender_node_id: &str,
        source_session_id: &str,
        source_invocation_id: &str,
        server_name: &str,
        tool_name: &str,
    ) -> Result<Option<PreparedWorkflowMcpHandoff>, String> {
        let Some(trigger) = self
            .repository
            .load_completed_turn_trigger(source_session_id)?
        else {
            return Err("The calling Session is not associated with a Workflow node.".to_string());
        };
        if trigger.workflow_instance_id != workflow_instance_id
            || trigger.sender_node_id != sender_node_id
        {
            return Err(
                "The Harness binding does not match the calling Workflow Session.".to_string(),
            );
        }
        let matches = trigger
            .recipe
            .connections
            .iter()
            .filter(|connection| connection.sender_node_id == sender_node_id)
            .filter(|connection| {
                matches!(
                    &connection.mechanism,
                    Some(super::domain::WorkflowConnectionMechanism::McpNativePromptAgent {
                        server_name: configured_server,
                        tool_name: configured_tool,
                        ..
                    }) if configured_server == server_name && configured_tool == tool_name
                )
            })
            .collect::<Vec<_>>();
        if matches.is_empty() {
            return Ok(None);
        }
        if matches.len() != 1 {
            return Err("More than one Workflow connection matches this MCP call.".to_string());
        }
        let connection = matches[0];
        let activation_id = format!("workflow-connection-activation-{}", Uuid::new_v4());
        let warning_text = match &connection.mechanism {
            Some(super::domain::WorkflowConnectionMechanism::McpNativePromptAgent {
                warning_text,
                ..
            }) => warning_text.clone(),
            _ => None,
        };
        Ok(Some(PreparedWorkflowMcpHandoff {
            invocation: WorkflowInvocation {
                contract_version: "workflow-invocation/v1".to_string(),
                connection_activation_reference: activation_id,
                recipe_reference: trigger.recipe.id,
                connection_reference: connection.id.clone(),
                sender_node_reference: sender_node_id.to_string(),
                sender_activation_reference: source_invocation_id.to_string(),
            },
            warning_text,
        }))
    }

    pub(crate) fn settle_mcp_native_handoff(
        &self,
        workflow_instance_id: &str,
        source_session_id: &str,
        invocation: &WorkflowInvocation,
        output: Result<WorkflowMcpOutput, String>,
    ) -> Result<(), String> {
        if invocation.contract_version != "workflow-invocation/v1" {
            return Err("Unsupported Workflow invocation contract.".to_string());
        }
        let trigger = self.repository.load_mcp_prepared_trigger(
            workflow_instance_id,
            &invocation.recipe_reference,
            &invocation.sender_node_reference,
            source_session_id,
        )?;
        let connection = trigger
            .recipe
            .connections
            .iter()
            .find(|connection| connection.id == invocation.connection_reference)
            .cloned()
            .ok_or_else(|| {
                "The Workflow MCP connection is absent from its activated recipe.".to_string()
            })?;
        if connection.sender_node_id != invocation.sender_node_reference {
            return Err("Workflow invocation sender does not match its connection.".to_string());
        }
        if !matches!(
            &connection.mechanism,
            Some(super::domain::WorkflowConnectionMechanism::McpNativePromptAgent { .. })
        ) {
            return Err(
                "Workflow invocation does not reference a native MCP connection.".to_string(),
            );
        }
        let receiver_node_id = connection
            .receiver_node_id
            .clone()
            .ok_or_else(|| "The Workflow MCP connection is dangling.".to_string())?;
        self.repository
            .create_connection_activation(WorkflowConnectionActivationPreparation {
                id: invocation.connection_activation_reference.clone(),
                workflow_instance_id: workflow_instance_id.to_string(),
                recipe_id: invocation.recipe_reference.clone(),
                connection_id: invocation.connection_reference.clone(),
                sender_node_id: invocation.sender_node_reference.clone(),
                receiver_node_id: receiver_node_id.clone(),
                source_session_id: source_session_id.to_string(),
                source_invocation_id: invocation.sender_activation_reference.clone(),
                requested_at: Utc::now().to_rfc3339(),
            })?;
        let context = WorkflowMcpActivationContext {
            activation_id: invocation.connection_activation_reference.clone(),
            trigger,
        };
        let settlement = (|| -> Result<(), (String, String)> {
            let output = output.map_err(|reason| ("mcp_output".to_string(), reason))?;
            let mut prompt_parts = Vec::with_capacity(output.file_paths.len() + 1);
            for file_path in &output.file_paths {
                prompt_parts.push(
                    canonical_relative_literal(Path::new(file_path), "Workflow MCP file path")
                        .map_err(|reason| ("mcp_output".to_string(), reason))?,
                );
            }
            prompt_parts.push(output.prompt_text.clone());
            let resolved_output_json = serde_json::to_string(&output).map_err(|error| {
                (
                    "mcp_output".to_string(),
                    format!("Unable to record Workflow MCP output: {error}"),
                )
            })?;
            self.repository
                .mark_mcp_connection_activation_resolved(
                    &context.activation_id,
                    &resolved_output_json,
                    &Utc::now().to_rfc3339(),
                )
                .map_err(|reason| ("mcp_output_recording".to_string(), reason))?;
            self.deliver_connection_prompt(
                &context.activation_id,
                &context.trigger,
                &receiver_node_id,
                prompt_parts.join("\n"),
            )
            .map_err(|(stage, reason)| (stage.to_string(), reason))
        })();
        settlement.map_err(|(stage, reason)| {
            let _ = self.repository.mark_connection_activation_failed(
                &context.activation_id,
                &stage,
                &reason,
                &Utc::now().to_rfc3339(),
            );
            reason
        })
    }

    fn receiver_lane(
        &self,
        workflow_instance_id: &str,
        receiver_node_id: &str,
    ) -> Result<Arc<Mutex<()>>, String> {
        let mut lanes = self
            .receiver_lanes
            .lock()
            .map_err(|_| "Workflow receiver Session lanes are unavailable.".to_string())?;
        Ok(lanes
            .entry((
                workflow_instance_id.to_string(),
                receiver_node_id.to_string(),
            ))
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone())
    }

    fn most_recent_receiver_session(
        &self,
        trigger: &WorkflowCompletedTurnTrigger,
        receiver_node_id: &str,
    ) -> Result<Option<AgentSessionId>, String> {
        let instance = self
            .repository
            .load_workflow_instance(&trigger.workflow_instance_id)?;
        let mut candidates = Vec::new();
        for association in instance
            .session_associations
            .iter()
            .filter(|association| association.node_id == receiver_node_id)
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

    pub(crate) fn list_workflow_instances(&self) -> Result<Vec<WorkflowInstanceSummary>, String> {
        self.repository
            .list_workflow_instances()?
            .into_iter()
            .map(|record| {
                self.project_instance(record, Vec::new())
                    .map(|instance| instance.summary)
            })
            .collect()
    }

    pub(crate) fn load_workflow_instance(
        &self,
        workflow_instance_id: &str,
    ) -> Result<WorkflowInstance, String> {
        let record = self
            .repository
            .load_workflow_instance(workflow_instance_id)?;
        let mut activations = self
            .repository
            .list_connection_activations(workflow_instance_id)?;
        activations.sort_by(|left, right| {
            right
                .requested_at
                .cmp(&left.requested_at)
                .then_with(|| right.id.cmp(&left.id))
        });
        self.project_instance(record, activations)
    }

    fn project_instance(
        &self,
        record: WorkflowInstanceRecord,
        connection_activations: Vec<WorkflowConnectionActivationRecord>,
    ) -> Result<WorkflowInstance, String> {
        let mut sessions = Vec::with_capacity(record.session_associations.len());
        for association in &record.session_associations {
            let session_id = AgentSessionId::new(association.session_id.clone())
                .map_err(|error| error.to_string())?;
            let history = self
                .sessions
                .load_session(&session_id)
                .map_err(|error| error.to_string())?;
            let active = history
                .invocations
                .iter()
                .any(|entry| entry.invocation.status.is_active());
            sessions.push(WorkflowInstanceSession {
                node_id: association.node_id.clone(),
                session_id: association.session_id.clone(),
                title: history.session.title,
                activity: if active {
                    WorkflowSessionActivity::Active
                } else {
                    WorkflowSessionActivity::Idle
                },
                latest_turn_summary: history
                    .invocations
                    .last()
                    .map(|entry| summarize(&entry.invocation.submitted_text)),
                associated_at: association.associated_at.clone(),
            });
        }
        let active_session_count = sessions
            .iter()
            .filter(|session| session.activity == WorkflowSessionActivity::Active)
            .count() as u32;
        let summary = WorkflowInstanceSummary {
            id: record.id.clone(),
            workflow_type_id: record.workflow_type_id.clone(),
            workflow_type_name: record.workflow_type_name.clone(),
            recipe_id: record.recipe.id.clone(),
            name: record.name.clone(),
            session_count: sessions.len() as u32,
            active_session_count,
            idle_session_count: sessions.len() as u32 - active_session_count,
            launch_status: record.launch_activation.status,
            created_at: record.created_at.clone(),
        };
        Ok(WorkflowInstance {
            summary,
            starting_prompt: record.starting_prompt,
            working_directory: record.working_directory,
            recipe: record.recipe,
            sessions,
            launch_activation: record.launch_activation,
            connection_activations: connection_activations
                .into_iter()
                .map(project_connection_activation)
                .collect(),
        })
    }

    fn record_failure(&self, activation_id: &str, stage: &str, reason: &str) {
        let _ = self.repository.mark_instance_launch_failed(
            activation_id,
            stage,
            reason,
            &Utc::now().to_rfc3339(),
        );
    }
}

fn project_connection_activation(
    record: WorkflowConnectionActivationRecord,
) -> WorkflowConnectionActivation {
    let status = if record.failed_at.is_some() {
        WorkflowConnectionActivationStatus::Failed
    } else if record.launch_accepted_at.is_some() {
        WorkflowConnectionActivationStatus::LaunchAccepted
    } else if record.launch_requested_at.is_some() {
        WorkflowConnectionActivationStatus::LaunchRequested
    } else if record.associated_at.is_some() {
        WorkflowConnectionActivationStatus::Associated
    } else if record.resolved_at.is_some() {
        WorkflowConnectionActivationStatus::Resolved
    } else {
        WorkflowConnectionActivationStatus::Requested
    };
    WorkflowConnectionActivation {
        id: record.id,
        recipe_id: record.recipe_id,
        connection_id: record.connection_id,
        sender_node_id: record.sender_node_id,
        receiver_node_id: record.receiver_node_id,
        source_session_id: record.source_session_id,
        source_invocation_id: record.source_invocation_id,
        target_session_id: record.target_session_id,
        target_invocation_id: record.target_invocation_id,
        delivery_kind: record.delivery_kind,
        session_mode: record.session_mode,
        context_inheritance: record.context_inheritance,
        compression: record.compression,
        resolved_file_path: record.resolved_file_path,
        status,
        requested_at: record.requested_at,
        resolved_at: record.resolved_at,
        associated_at: record.associated_at,
        launch_requested_at: record.launch_requested_at,
        launch_accepted_at: record.launch_accepted_at,
        failed_at: record.failed_at,
    }
}

fn launch_extension(
    harness: &WorkflowHarnessConfig,
) -> Result<Option<RuntimeLaunchExtension>, String> {
    if !harness.skills().is_empty() {
        return Err("Workflow launch does not support Harness skills yet.".to_string());
    }
    if !harness.hooks().is_empty() {
        return Err("Workflow launch does not support Harness hooks yet.".to_string());
    }
    validate_supported_codex_model(harness.default_model())?;
    let reasoning = harness.default_reasoning().unwrap_or_default();
    if harness.prompt_prefix().contains('\0') || harness.prompt_prefix().len() > 65_536 {
        return Err(
            "Workflow Harness instructions are invalid for direct prompt delivery.".to_string(),
        );
    }
    let mut extension = RuntimeLaunchExtension::default();
    if !reasoning.is_empty() {
        extension.additional_args = vec![
            "-c".to_string(),
            format!("model_reasoning_effort=\"{reasoning}\""),
        ];
    }
    if let Some(instructions) = nonempty(harness.prompt_prefix()) {
        extension.initial_prompt_prefix = Some(InitialPromptPrefix {
            source: "workflow_recipe_node_instructions".to_string(),
            version: 1,
            content: instructions,
        });
    }
    Ok((extension != RuntimeLaunchExtension::default()).then_some(extension))
}

fn validate_runtime_literal(value: &str, label: &str) -> Result<(), String> {
    if value != value.trim()
        || value.len() > 128
        || value.chars().any(|character| character.is_control())
    {
        return Err(format!("Workflow Harness {label} is invalid."));
    }
    Ok(())
}

fn validate_supported_codex_model(value: &str) -> Result<(), String> {
    validate_runtime_literal(value, "model")?;
    let model = value.trim();
    if !model.is_empty() && !SUPPORTED_CODEX_MODELS.contains(&model) {
        return Err(format!(
            "Unsupported Codex model {model}; use {}.",
            SUPPORTED_CODEX_MODELS.join(" or ")
        ));
    }
    Ok(())
}

fn nonempty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_string())
}

fn summarize(value: &str) -> String {
    let value = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if value.chars().count() <= 120 {
        value
    } else {
        format!("{}...", value.chars().take(117).collect::<String>())
    }
}

fn resolve_expected_file(
    instance_root: &Path,
    selector: &super::domain::WorkflowExpectedFileSelector,
    final_output: Option<&str>,
) -> Result<String, String> {
    let root = instance_root
        .canonicalize()
        .map_err(|error| format!("Unable to resolve the Workflow instance folder: {error}"))?;
    match selector {
        super::domain::WorkflowExpectedFileSelector::FolderFilenamePattern {
            folder,
            filename_pattern,
        } => {
            let folder = safe_folder(&root, folder)?;
            let matcher = Glob::new(filename_pattern)
                .map_err(|error| format!("Expected filename pattern is invalid: {error}"))?
                .compile_matcher();
            let mut matches = fs::read_dir(&folder)
                .map_err(|error| format!("Unable to read the expected file folder: {error}"))?
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let file_type = entry.file_type().ok()?;
                    if !file_type.is_file() || !matcher.is_match(entry.file_name()) {
                        return None;
                    }
                    let path = entry.path().canonicalize().ok()?;
                    if !path.starts_with(&folder) {
                        return None;
                    }
                    let modified = entry
                        .metadata()
                        .and_then(|metadata| metadata.modified())
                        .unwrap_or(SystemTime::UNIX_EPOCH);
                    let relative = relative_literal(&root, &path).ok()?;
                    Some((modified, relative))
                })
                .collect::<Vec<_>>();
            matches.sort_by(|left, right| {
                Reverse(left.0)
                    .cmp(&Reverse(right.0))
                    .then_with(|| left.1.cmp(&right.1))
            });
            matches
                .into_iter()
                .next()
                .map(|(_, relative)| relative)
                .ok_or_else(|| "No file matched the expected filename pattern.".to_string())
        }
        super::domain::WorkflowExpectedFileSelector::FolderOutputRegex {
            folder,
            output_regex,
        } => {
            let folder = safe_folder(&root, folder)?;
            let output = final_output.ok_or_else(|| {
                "The completed turn has no persisted final agent output.".to_string()
            })?;
            let expression = Regex::new(output_regex)
                .map_err(|error| format!("Agent output regex is invalid: {error}"))?;
            let capture = expression
                .captures(output)
                .and_then(|captures| captures.get(1))
                .ok_or_else(|| {
                    "The final agent output did not provide capture group 1.".to_string()
                })?
                .as_str();
            let capture = Path::new(capture);
            validate_relative_path(capture, "Captured file path")?;
            let path = folder.join(capture).canonicalize().map_err(|error| {
                format!("Unable to resolve the file captured from agent output: {error}")
            })?;
            if !path.starts_with(&folder) || !path.is_file() {
                return Err(
                    "The captured file path is not a file beneath the configured folder."
                        .to_string(),
                );
            }
            relative_literal(&root, &path)
        }
    }
}

fn safe_folder(root: &Path, configured: &str) -> Result<PathBuf, String> {
    let configured = Path::new(configured);
    validate_relative_path(configured, "Expected file folder")?;
    let folder = root
        .join(configured)
        .canonicalize()
        .map_err(|error| format!("Unable to resolve the expected file folder: {error}"))?;
    if !folder.starts_with(root) || !folder.is_dir() {
        return Err(
            "Expected file folder must be a folder inside the Workflow instance.".to_string(),
        );
    }
    Ok(folder)
}

fn validate_relative_path(path: &Path, label: &str) -> Result<(), String> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_) | Component::CurDir))
    {
        return Err(format!(
            "{label} must be a relative path without parent traversal."
        ));
    }
    Ok(())
}

fn relative_literal(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "Resolved file is outside the Workflow instance.".to_string())?;
    Ok(relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/"))
}

fn canonical_relative_literal(path: &Path, label: &str) -> Result<String, String> {
    validate_relative_path(path, label)?;
    let literal = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    if literal.is_empty() {
        Err(format!("{label} must name a file."))
    } else {
        Ok(literal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent_sessions::{
            application::{
                AgentSessionNotification, AgentSessionNotifier, SystemAgentSessionProviders,
            },
            domain::{
                AgentInvocationId, AgentInvocationStatus, AgentInvocationTerminalStatus,
                AgentRuntimeEvent, AgentRuntimeEventId, AgentRuntimeEventSource,
                AgentRuntimeOptions, ExternalRuntimeContextId, InvocationCompletion,
                NormalizedRuntimeEvent,
            },
            ports::{
                AgentRuntime, AgentRuntimeUpdateSink, AgentSessionRepository,
                RuntimeInvocationMode, RuntimeInvocationPreflight, RuntimeInvocationRequest,
                RuntimePortError,
            },
            repository::SqliteAgentSessionRepository,
        },
        workflows::{
            domain::{
                WorkflowElementKind, WorkflowElementRef, WorkflowExpectedFileSelector,
                WorkflowInitialCheck, WorkflowLaunchStatus, WorkflowMatchSelection,
                WorkflowNodeHarness,
            },
            repository::SqliteWorkflowRepository,
        },
    };
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };

    #[derive(Default)]
    struct RecordingRuntime {
        launches: Mutex<Vec<RuntimeInvocationRequest>>,
        fail_preflight: AtomicBool,
    }

    impl AgentRuntime for RecordingRuntime {
        fn preflight_invocation(
            &self,
            _: RuntimeInvocationMode,
            requested_options: &AgentRuntimeOptions,
        ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
            if self.fail_preflight.load(Ordering::SeqCst) {
                return Err(RuntimePortError::new(
                    crate::agent_sessions::ports::RuntimePortErrorKind::UnsupportedOptions,
                    "unsupported requested options",
                ));
            }
            Ok(RuntimeInvocationPreflight {
                effective_options: requested_options.clone(),
            })
        }

        fn start_invocation(
            &self,
            request: RuntimeInvocationRequest,
            _: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.launches.lock().unwrap().push(request);
            Ok(())
        }

        fn resume_invocation(
            &self,
            request: RuntimeInvocationRequest,
            _: ExternalRuntimeContextId,
            _: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            self.launches.lock().unwrap().push(request);
            Ok(())
        }

        fn cancel_invocation(&self, _: &AgentInvocationId) -> Result<(), RuntimePortError> {
            Ok(())
        }
    }

    struct RecordingHarnessBinder;

    impl WorkflowSessionHarnessBinder for RecordingHarnessBinder {
        fn bind_workflow_session(&self, _: BindWorkflowSessionHarness) -> Result<(), String> {
            Ok(())
        }
    }

    struct FailingHarnessBinder;

    impl WorkflowSessionHarnessBinder for FailingHarnessBinder {
        fn bind_workflow_session(&self, _: BindWorkflowSessionHarness) -> Result<(), String> {
            Err("managed upstream is unavailable".to_string())
        }
    }

    struct ReceiverFailingHarnessBinder;

    impl WorkflowSessionHarnessBinder for ReceiverFailingHarnessBinder {
        fn bind_workflow_session(&self, request: BindWorkflowSessionHarness) -> Result<(), String> {
            if request.node_id == "sender" {
                Ok(())
            } else {
                Err("receiver Harness binding failed".to_string())
            }
        }
    }

    struct NoopNotifier;

    impl AgentSessionNotifier for NoopNotifier {
        fn notify(&self, _: AgentSessionNotification) -> Result<(), String> {
            Ok(())
        }
    }

    fn fixture() -> (
        tempfile::TempDir,
        Arc<SqliteWorkflowRepository>,
        Arc<RecordingRuntime>,
        WorkflowApplication,
    ) {
        fixture_with_binder(Arc::new(RecordingHarnessBinder))
    }

    fn fixture_with_binder(
        harnesses: Arc<dyn WorkflowSessionHarnessBinder>,
    ) -> (
        tempfile::TempDir,
        Arc<SqliteWorkflowRepository>,
        Arc<RecordingRuntime>,
        WorkflowApplication,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("workflow-launch.sqlite");
        let agent_connection = crate::storage::open_active_database(&database_path).unwrap();
        let agent_repository =
            Arc::new(SqliteAgentSessionRepository::new(agent_connection).unwrap());
        let runtime = Arc::new(RecordingRuntime::default());
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            agent_repository,
            runtime.clone(),
            Arc::new(NoopNotifier),
            providers.clone(),
            providers,
            Some("codex-test".to_string()),
        ));
        let workflows = Arc::new(SqliteWorkflowRepository::open(&database_path).unwrap());
        let application = WorkflowApplication::new(
            workflows.clone(),
            sessions,
            harnesses,
            directory.path().join("workflow-instances"),
        );
        (directory, workflows, runtime, application)
    }

    fn activate_start(
        repository: &SqliteWorkflowRepository,
        mut harness: WorkflowHarnessConfig,
    ) -> String {
        let definition = repository.create_workflow_type("Review loop").unwrap();
        let workflow_type_id = definition.workflow_type.id;
        if harness.name().is_empty() {
            harness.0.identity.name = "Architecture reviewer".to_string();
            harness.0.identity.machine_key = "architecture_reviewer".to_string();
        }
        repository
            .save_node_draft(
                &workflow_type_id,
                WorkflowNodeConfig {
                    id: "start".to_string(),
                    name: "Review architecture".to_string(),
                    harness_name: harness.name().to_string(),
                    role_name: None,
                    position_x: 80.0,
                    position_y: 100.0,
                    is_starting_point: true,
                    harness: Some(WorkflowNodeHarness::Standalone { config: harness }),
                },
            )
            .unwrap();
        repository
            .activate_changes(
                &workflow_type_id,
                &[WorkflowElementRef {
                    kind: WorkflowElementKind::Node,
                    id: "start".to_string(),
                }],
            )
            .unwrap();
        workflow_type_id
    }

    #[test]
    fn launch_materializes_one_human_prompt_without_claiming_provider_activity() {
        let (_directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig::test_definition(
                "Architecture reviewer",
                "Reviewer",
                "Review the architecture carefully.",
                "gpt-5.6-sol",
                "high",
            ),
        );
        let prompt = "Inspect the proposed architecture and identify the main risk.";

        let instance = application
            .launch_workflow_instance(&workflow_type_id, None, prompt)
            .unwrap();

        assert_eq!(
            instance.summary.launch_status,
            WorkflowLaunchStatus::LaunchAccepted
        );
        assert_eq!(instance.summary.session_count, 1);
        assert_eq!(instance.summary.active_session_count, 1);
        assert_eq!(instance.summary.name, prompt);
        assert_eq!(instance.starting_prompt, prompt);
        assert_eq!(instance.launch_activation.source_kind, "human");
        assert_eq!(
            instance.launch_activation.delivery_kind,
            "direct_prompt_runtime_v1"
        );
        assert_eq!(instance.launch_activation.session_mode, "fresh");
        assert_eq!(instance.launch_activation.context_inheritance, "none");
        assert_eq!(instance.launch_activation.compression, "none");
        assert!(instance.launch_activation.associated_at.is_some());
        assert!(instance.launch_activation.launch_requested_at.is_some());
        assert!(instance.launch_activation.launch_accepted_at.is_some());
        assert!(PathBuf::from(&instance.working_directory).is_dir());

        let launches = runtime.launches.lock().unwrap();
        assert_eq!(launches.len(), 1);
        assert!(launches[0].submitted_text.contains(prompt));
        assert!(launches[0]
            .submitted_text
            .contains("Review the architecture carefully."));
        assert_eq!(launches[0].options.model.as_deref(), Some("gpt-5.6-sol"));
        let extension = launches[0].launch_extension.as_ref().unwrap();
        assert_eq!(
            extension
                .initial_prompt_prefix
                .as_ref()
                .map(|prefix| prefix.content.as_str()),
            Some("Review the architecture carefully.")
        );
        assert_eq!(
            extension.additional_args,
            ["-c", "model_reasoning_effort=\"high\""]
        );
        drop(launches);

        let loaded = application
            .sessions
            .load_session(&AgentSessionId::new(instance.sessions[0].session_id.clone()).unwrap())
            .unwrap();
        assert_eq!(loaded.invocations.len(), 1);
        assert_eq!(loaded.invocations[0].invocation.submitted_text, prompt);
        assert_eq!(
            loaded.invocations[0].invocation.input_provenance,
            crate::agent_sessions::domain::AgentInvocationInputProvenance::User
        );
        assert!(loaded.invocations[0].invocation.status.is_active());

        let listed = application.list_workflow_instances().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, instance.summary.id);
    }

    #[test]
    fn workflow_instance_projection_reopens_with_agent_session_history() {
        let (directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig::test_definition(
                "Architecture reviewer",
                "",
                "Review the architecture carefully.",
                "gpt-5.6-terra",
                "medium",
            ),
        );
        let prompt = "Review this durable Workflow launch.";
        let launched = application
            .launch_workflow_instance(&workflow_type_id, Some("Durable review"), prompt)
            .unwrap();
        let instance_id = launched.summary.id.clone();
        let recipe_id = launched.recipe.id.clone();
        let session_id = launched.sessions[0].session_id.clone();

        drop(application);
        drop(repository);
        drop(runtime);

        let database_path = directory.path().join("workflow-launch.sqlite");
        let agent_connection = crate::storage::open_active_database(&database_path).unwrap();
        let agent_repository =
            Arc::new(SqliteAgentSessionRepository::new(agent_connection).unwrap());
        let reopened_runtime = Arc::new(RecordingRuntime::default());
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            agent_repository,
            reopened_runtime.clone(),
            Arc::new(NoopNotifier),
            providers.clone(),
            providers,
            Some("codex-test".to_string()),
        ));
        let workflows = Arc::new(SqliteWorkflowRepository::open(&database_path).unwrap());
        let reopened = WorkflowApplication::new(
            workflows,
            sessions,
            Arc::new(RecordingHarnessBinder),
            directory.path().join("workflow-instances"),
        );

        let loaded = reopened.load_workflow_instance(&instance_id).unwrap();
        assert_eq!(loaded.summary.name, "Durable review");
        assert_eq!(
            loaded.summary.launch_status,
            WorkflowLaunchStatus::LaunchAccepted
        );
        assert_eq!(loaded.recipe.id, recipe_id);
        assert_eq!(loaded.starting_prompt, prompt);
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].session_id, session_id);
        assert_eq!(
            loaded.sessions[0].latest_turn_summary.as_deref(),
            Some(prompt)
        );
        assert_eq!(reopened.list_workflow_instances().unwrap().len(), 1);
        assert!(reopened_runtime.launches.lock().unwrap().is_empty());
    }

    #[test]
    fn unsupported_harness_capabilities_fail_before_instance_or_runtime_creation() {
        let (_directory, repository, runtime, application) = fixture();
        let mut harness = WorkflowHarnessConfig::test_definition("Reviewer", "", "", "", "");
        harness.0.skills.items.push(
            crate::orchestration::conversation_harness_working_copy::HarnessSkillConfiguration {
                name: "security-review".to_string(),
                path: "security-review".to_string(),
                purpose: String::new(),
                use_when: String::new(),
                policy: crate::orchestration::conversation_harness_working_copy::HarnessSkillPolicy::Available,
            },
        );
        let workflow_type_id = activate_start(&repository, harness);

        let error = application
            .launch_workflow_instance(&workflow_type_id, None, "Review this.")
            .unwrap_err();

        assert!(error.contains("does not support Harness skills yet"));
        assert!(application.list_workflow_instances().unwrap().is_empty());
        assert!(runtime.launches.lock().unwrap().is_empty());
    }

    #[test]
    fn unsupported_model_fails_before_instance_or_runtime_creation() {
        let (_directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig::test_definition(
                "Reviewer",
                "",
                "",
                "unverified-model",
                "medium",
            ),
        );

        let error = application
            .launch_workflow_instance(&workflow_type_id, None, "Review this.")
            .unwrap_err();

        assert!(error.contains("Unsupported Codex model unverified-model"));
        assert!(application.list_workflow_instances().unwrap().is_empty());
        assert!(runtime.launches.lock().unwrap().is_empty());
    }

    #[test]
    fn runtime_rejection_retains_the_failed_instance_without_claiming_acceptance() {
        let (_directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig::test_definition("Reviewer", "", "", "gpt-5.6-sol", "medium"),
        );
        runtime.fail_preflight.store(true, Ordering::SeqCst);

        let instance = application
            .launch_workflow_instance(&workflow_type_id, Some("Rejected review"), "Review this.")
            .unwrap();

        assert_eq!(instance.summary.launch_status, WorkflowLaunchStatus::Failed);
        assert_eq!(instance.summary.name, "Rejected review");
        assert!(instance.launch_activation.launch_requested_at.is_some());
        assert!(instance.launch_activation.launch_accepted_at.is_none());
        assert!(instance.launch_activation.failed_at.is_some());
        assert_eq!(
            instance.launch_activation.failure_stage.as_deref(),
            Some("runtime_launch")
        );
        assert!(instance
            .launch_activation
            .failure_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("launch was not accepted")));
        assert!(runtime.launches.lock().unwrap().is_empty());
    }

    #[test]
    fn harness_binding_failure_is_durable_and_prevents_runtime_launch() {
        let (_directory, repository, runtime, application) =
            fixture_with_binder(Arc::new(FailingHarnessBinder));
        let mut harness = WorkflowHarnessConfig::test_definition("Reviewer", "", "", "", "");
        harness.0.tools.mcp_servers = vec![crate::workflows::domain::WorkflowMcpServerExposure {
            server_name: "plan_builder".to_string(),
            access: crate::workflows::domain::WorkflowMcpServerAccess::EntireServer,
        }];
        let workflow_type_id = activate_start(&repository, harness);

        let instance = application
            .launch_workflow_instance(&workflow_type_id, None, "Review this.")
            .unwrap();

        assert_eq!(instance.summary.launch_status, WorkflowLaunchStatus::Failed);
        assert_eq!(
            instance.launch_activation.failure_stage.as_deref(),
            Some("harness_binding")
        );
        assert!(instance
            .launch_activation
            .failure_reason
            .as_deref()
            .unwrap()
            .contains("managed upstream"));
        assert!(instance.launch_activation.launch_requested_at.is_none());
        assert!(runtime.launches.lock().unwrap().is_empty());
    }

    struct ExecutionFixture {
        _directory: tempfile::TempDir,
        agent_repository: Arc<SqliteAgentSessionRepository>,
        workflow_repository: Arc<SqliteWorkflowRepository>,
        runtime: Arc<RecordingRuntime>,
        application: WorkflowApplication,
        workflow_type_id: String,
    }

    fn expected_file_connection(
        id: &str,
        receiver: &str,
        selector: WorkflowExpectedFileSelector,
        prompt: &str,
    ) -> WorkflowConnectionConfig {
        WorkflowConnectionConfig {
            id: id.to_string(),
            name: id.to_string(),
            sender_node_id: "sender".to_string(),
            receiver_node_id: Some(receiver.to_string()),
            mechanism: Some(
                super::super::domain::WorkflowConnectionMechanism::TurnFinishedExpectedFile {
                    file_selector: selector,
                    description_text: "Expected handoff file".to_string(),
                    prompt_text: prompt.to_string(),
                    match_selection: WorkflowMatchSelection::Newest,
                    initial_check: WorkflowInitialCheck::OnceImmediately,
                },
            ),
        }
    }

    fn mcp_native_connection(id: &str, receiver: &str) -> WorkflowConnectionConfig {
        WorkflowConnectionConfig {
            id: id.to_string(),
            name: id.to_string(),
            sender_node_id: "sender".to_string(),
            receiver_node_id: Some(receiver.to_string()),
            mechanism: Some(
                super::super::domain::WorkflowConnectionMechanism::McpNativePromptAgent {
                    server_name: super::super::mcp::SERVER_NAME.to_string(),
                    tool_name: super::super::mcp::TOOL_NAME.to_string(),
                    warning_text: None,
                },
            ),
        }
    }

    fn execution_fixture(connections: Vec<WorkflowConnectionConfig>) -> ExecutionFixture {
        execution_fixture_with_binder(connections, Arc::new(RecordingHarnessBinder))
    }

    fn execution_fixture_with_binder(
        connections: Vec<WorkflowConnectionConfig>,
        harnesses: Arc<dyn WorkflowSessionHarnessBinder>,
    ) -> ExecutionFixture {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("workflow-execution.sqlite");
        let agent_connection = crate::storage::open_active_database(&database_path).unwrap();
        let agent_repository = Arc::new(
            SqliteAgentSessionRepository::new(agent_connection).expect("Agent Session repository"),
        );
        let runtime = Arc::new(RecordingRuntime::default());
        let providers = Arc::new(SystemAgentSessionProviders);
        let sessions = Arc::new(AgentSessionApplication::new(
            agent_repository.clone(),
            runtime.clone(),
            Arc::new(NoopNotifier),
            providers.clone(),
            providers,
            Some("codex-test".to_string()),
        ));
        let workflow_repository = Arc::new(SqliteWorkflowRepository::open(&database_path).unwrap());
        let definition = workflow_repository
            .create_workflow_type("Connection execution")
            .unwrap();
        let workflow_type_id = definition.workflow_type.id;
        let receiver_ids = connections
            .iter()
            .map(|connection| connection.receiver_node_id.clone().unwrap())
            .collect::<std::collections::BTreeSet<_>>();
        let harness = |name: &str| WorkflowHarnessConfig::test_definition(name, "", "", "", "");
        let mut sender_harness = harness("Sender harness");
        sender_harness.0.tools.mcp_servers =
            vec![super::super::domain::WorkflowMcpServerExposure {
                server_name: super::super::mcp::SERVER_NAME.to_string(),
                access: super::super::domain::WorkflowMcpServerAccess::SelectedTools {
                    tool_names: vec![super::super::mcp::TOOL_NAME.to_string()],
                },
            }];
        workflow_repository
            .save_node_draft(
                &workflow_type_id,
                WorkflowNodeConfig {
                    id: "sender".to_string(),
                    name: "Sender".to_string(),
                    harness_name: "Sender harness".to_string(),
                    role_name: None,
                    position_x: 0.0,
                    position_y: 0.0,
                    is_starting_point: true,
                    harness: Some(WorkflowNodeHarness::Standalone {
                        config: sender_harness,
                    }),
                },
            )
            .unwrap();
        for (index, receiver_id) in receiver_ids.iter().enumerate() {
            workflow_repository
                .save_node_draft(
                    &workflow_type_id,
                    WorkflowNodeConfig {
                        id: receiver_id.clone(),
                        name: receiver_id.clone(),
                        harness_name: format!("{receiver_id} harness"),
                        role_name: None,
                        position_x: 300.0,
                        position_y: index as f64 * 160.0,
                        is_starting_point: false,
                        harness: Some(WorkflowNodeHarness::Standalone {
                            config: harness(&format!("{receiver_id} harness")),
                        }),
                    },
                )
                .unwrap();
        }
        for connection in &connections {
            workflow_repository
                .save_connection_draft(&workflow_type_id, connection.clone())
                .unwrap();
        }
        let mut elements = vec![WorkflowElementRef {
            kind: WorkflowElementKind::Node,
            id: "sender".to_string(),
        }];
        elements.extend(receiver_ids.into_iter().map(|id| WorkflowElementRef {
            kind: WorkflowElementKind::Node,
            id,
        }));
        elements.extend(connections.iter().map(|connection| WorkflowElementRef {
            kind: WorkflowElementKind::Connection,
            id: connection.id.clone(),
        }));
        workflow_repository
            .activate_changes(&workflow_type_id, &elements)
            .unwrap();
        let application = WorkflowApplication::new(
            workflow_repository.clone(),
            sessions,
            harnesses,
            directory.path().join("workflow-instances"),
        );
        ExecutionFixture {
            _directory: directory,
            agent_repository,
            workflow_repository,
            runtime,
            application,
            workflow_type_id,
        }
    }

    fn launch_execution_instance(fixture: &ExecutionFixture) -> WorkflowInstance {
        fixture
            .application
            .launch_workflow_instance(
                &fixture.workflow_type_id,
                Some("Execution instance"),
                "Produce the handoff.",
            )
            .unwrap()
    }

    fn complete_source_turn(
        fixture: &ExecutionFixture,
        instance: &WorkflowInstance,
        final_output: &str,
    ) -> (AgentSessionId, AgentInvocation) {
        let session_id = AgentSessionId::new(instance.sessions[0].session_id.clone()).unwrap();
        let invocation_id =
            AgentInvocationId::new(instance.launch_activation.target_invocation_id.clone())
                .unwrap();
        let now = Utc::now();
        fixture
            .agent_repository
            .append_event(AgentRuntimeEvent {
                id: AgentRuntimeEventId::new(format!("event-{}", Uuid::new_v4())).unwrap(),
                invocation_id: invocation_id.clone(),
                sequence: 0,
                source: AgentRuntimeEventSource::Runtime,
                raw_payload: serde_json::json!({"type":"agent_message","text":final_output}),
                normalized: Some(NormalizedRuntimeEvent {
                    kind: NormalizedRuntimeEventKind::AgentMessage,
                    text: Some(final_output.to_string()),
                    external_context_id: None,
                    usage: None,
                    details: Some(serde_json::json!({"role":"final"})),
                    tool_activity: None,
                }),
                recorded_at: now,
            })
            .unwrap();
        let invocation = fixture
            .agent_repository
            .finish_invocation(
                &invocation_id,
                InvocationCompletion {
                    status: AgentInvocationTerminalStatus::Completed,
                    completed_at: now,
                    exit_code: Some(0),
                    signal: None,
                    runtime_error: None,
                },
                now,
            )
            .unwrap();
        (session_id, invocation)
    }

    fn completed_notification(
        session_id: AgentSessionId,
        invocation: AgentInvocation,
    ) -> AgentSessionNotification {
        AgentSessionNotification::InvocationTerminal {
            session_id,
            invocation,
        }
    }

    #[test]
    fn completed_turn_selects_newest_file_and_constructs_interface_order_prompt() {
        let fixture = execution_fixture(vec![expected_file_connection(
            "edge",
            "receiver",
            WorkflowExpectedFileSelector::FolderFilenamePattern {
                folder: "handoffs".to_string(),
                filename_pattern: "*.md".to_string(),
            },
            "Review this handoff.",
        )]);
        let instance = launch_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.working_directory).join("handoffs");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("older.md"), "old").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(folder.join("newer.md"), "new").unwrap();
        let (session_id, invocation) = complete_source_turn(&fixture, &instance, "Done.");

        assert_eq!(
            fixture
                .application
                .on_agent_notification(&completed_notification(session_id, invocation))
                .unwrap(),
            1
        );

        let activations = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap();
        assert_eq!(activations.len(), 1);
        assert_eq!(
            activations[0].resolved_file_path.as_deref(),
            Some("handoffs/newer.md")
        );
        assert_eq!(activations[0].delivery_kind, "direct_prompt_runtime_v1");
        assert_eq!(activations[0].context_inheritance, "none");
        assert_eq!(activations[0].compression, "none");
        assert!(activations[0].launch_accepted_at.is_some());
        let launches = fixture.runtime.launches.lock().unwrap();
        assert_eq!(launches.len(), 2);
        assert_eq!(
            launches[1].submitted_text,
            "handoffs/newer.md\nExpected handoff file\nReview this handoff."
        );
    }

    #[test]
    fn mcp_native_handoff_resolves_current_recipe_and_preserves_interface_prompt_order() {
        let fixture = execution_fixture(vec![mcp_native_connection("edge", "receiver")]);
        let instance = launch_execution_instance(&fixture);
        let source_session_id = instance.sessions[0].session_id.clone();
        let source_invocation_id = instance.launch_activation.target_invocation_id.clone();

        let prepared = fixture
            .application
            .prepare_mcp_native_handoff(
                &instance.summary.id,
                "sender",
                &source_session_id,
                &source_invocation_id,
                super::super::mcp::SERVER_NAME,
                super::super::mcp::TOOL_NAME,
            )
            .unwrap()
            .expect("matching handoff");
        assert_eq!(prepared.invocation.recipe_reference, instance.recipe.id);
        assert_eq!(
            prepared.invocation.sender_activation_reference,
            source_invocation_id
        );

        fixture
            .application
            .settle_mcp_native_handoff(
                &instance.summary.id,
                &source_session_id,
                &prepared.invocation,
                Ok(WorkflowMcpOutput {
                    file_paths: vec!["first/a.md".into(), "second/b.json".into()],
                    prompt_text: "Review both outputs.".into(),
                }),
            )
            .unwrap();

        let launches = fixture.runtime.launches.lock().unwrap();
        assert_eq!(launches.len(), 2);
        assert_eq!(
            launches[1].submitted_text,
            "first/a.md\nsecond/b.json\nReview both outputs."
        );
        drop(launches);
        let activation = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap()
            .remove(0);
        assert!(activation.launch_accepted_at.is_some());
        let resolved: serde_json::Value =
            serde_json::from_str(activation.resolved_output_json.as_deref().unwrap()).unwrap();
        assert_eq!(resolved["filePaths"][0], "first/a.md");
        assert_eq!(resolved["promptText"], "Review both outputs.");
    }

    #[test]
    fn mcp_native_handoff_zero_match_is_ordinary_and_invalid_output_fails_once() {
        let ordinary = execution_fixture(vec![expected_file_connection(
            "file-edge",
            "receiver",
            WorkflowExpectedFileSelector::FolderFilenamePattern {
                folder: "handoffs".into(),
                filename_pattern: "*.md".into(),
            },
            "Review.",
        )]);
        let ordinary_instance = launch_execution_instance(&ordinary);
        assert!(ordinary
            .application
            .prepare_mcp_native_handoff(
                &ordinary_instance.summary.id,
                "sender",
                &ordinary_instance.sessions[0].session_id,
                &ordinary_instance.launch_activation.target_invocation_id,
                super::super::mcp::SERVER_NAME,
                super::super::mcp::TOOL_NAME,
            )
            .unwrap()
            .is_none());
        assert!(ordinary
            .workflow_repository
            .list_connection_activations(&ordinary_instance.summary.id)
            .unwrap()
            .is_empty());

        let fixture = execution_fixture(vec![mcp_native_connection("edge", "receiver")]);
        let instance = launch_execution_instance(&fixture);
        let prepared = fixture
            .application
            .prepare_mcp_native_handoff(
                &instance.summary.id,
                "sender",
                &instance.sessions[0].session_id,
                &instance.launch_activation.target_invocation_id,
                super::super::mcp::SERVER_NAME,
                super::super::mcp::TOOL_NAME,
            )
            .unwrap()
            .unwrap();
        let error = fixture
            .application
            .settle_mcp_native_handoff(
                &instance.summary.id,
                &instance.sessions[0].session_id,
                &prepared.invocation,
                Ok(WorkflowMcpOutput {
                    file_paths: vec!["../escape.md".into()],
                    prompt_text: "Review.".into(),
                }),
            )
            .unwrap_err();
        assert!(error.contains("relative"));
        let activation = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap()
            .remove(0);
        assert_eq!(activation.failure_stage.as_deref(), Some("mcp_output"));
        assert!(activation.launch_requested_at.is_none());
        assert_eq!(fixture.runtime.launches.lock().unwrap().len(), 1);
        assert!(fixture
            .application
            .settle_mcp_native_handoff(
                &instance.summary.id,
                &instance.sessions[0].session_id,
                &prepared.invocation,
                Ok(WorkflowMcpOutput {
                    file_paths: vec!["valid.md".into()],
                    prompt_text: "No retry.".into(),
                }),
            )
            .is_err());
        assert_eq!(fixture.runtime.launches.lock().unwrap().len(), 1);
    }

    #[test]
    fn mcp_native_prepare_uses_recipe_activated_after_session_creation() {
        let connection = mcp_native_connection("edge", "receiver");
        let fixture = execution_fixture(vec![connection.clone()]);
        let instance = launch_execution_instance(&fixture);
        let mut updated_connection = connection;
        let Some(super::super::domain::WorkflowConnectionMechanism::McpNativePromptAgent {
            warning_text,
            ..
        }) = updated_connection.mechanism.as_mut()
        else {
            unreachable!()
        };
        *warning_text = Some("Updated warning".to_string());
        fixture
            .workflow_repository
            .save_connection_draft(&fixture.workflow_type_id, updated_connection)
            .unwrap();
        let updated_recipe = fixture
            .workflow_repository
            .activate_changes(
                &fixture.workflow_type_id,
                &[WorkflowElementRef {
                    kind: WorkflowElementKind::Connection,
                    id: "edge".to_string(),
                }],
            )
            .unwrap()
            .active_recipe
            .unwrap();
        assert_ne!(updated_recipe.id, instance.recipe.id);

        let prepared = fixture
            .application
            .prepare_mcp_native_handoff(
                &instance.summary.id,
                "sender",
                &instance.sessions[0].session_id,
                &instance.launch_activation.target_invocation_id,
                super::super::mcp::SERVER_NAME,
                super::super::mcp::TOOL_NAME,
            )
            .unwrap()
            .unwrap();
        assert_eq!(prepared.invocation.recipe_reference, updated_recipe.id);
        assert_eq!(prepared.warning_text.as_deref(), Some("Updated warning"));
    }

    #[test]
    fn regex_uses_only_the_exact_completed_turn_final_output_capture() {
        let fixture = execution_fixture(vec![expected_file_connection(
            "edge",
            "receiver",
            WorkflowExpectedFileSelector::FolderOutputRegex {
                folder: "handoffs".to_string(),
                output_regex: r"handoff: ([a-z-]+\.md)".to_string(),
            },
            "Continue.",
        )]);
        let instance = launch_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.working_directory).join("handoffs");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("chosen.md"), "chosen").unwrap();
        fs::write(folder.join("ignored.md"), "ignored").unwrap();
        let (session_id, invocation) =
            complete_source_turn(&fixture, &instance, "final handoff: chosen.md");

        fixture
            .application
            .on_agent_notification(&completed_notification(session_id, invocation))
            .unwrap();

        let activation = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap()
            .remove(0);
        assert_eq!(
            activation.resolved_file_path.as_deref(),
            Some("handoffs/chosen.md")
        );
    }

    #[test]
    fn only_completed_invocations_fire_and_missing_file_failure_preserves_sender_completion() {
        let fixture = execution_fixture(vec![expected_file_connection(
            "edge",
            "receiver",
            WorkflowExpectedFileSelector::FolderFilenamePattern {
                folder: "handoffs".to_string(),
                filename_pattern: "*.md".to_string(),
            },
            "Continue.",
        )]);
        let instance = launch_execution_instance(&fixture);
        fs::create_dir_all(PathBuf::from(&instance.working_directory).join("handoffs")).unwrap();
        let (session_id, invocation) = complete_source_turn(&fixture, &instance, "Done.");
        let mut failed = invocation.clone();
        failed.status = AgentInvocationStatus::Failed;

        assert_eq!(
            fixture
                .application
                .on_agent_notification(&completed_notification(session_id.clone(), failed))
                .unwrap(),
            0
        );
        assert!(fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap()
            .is_empty());

        fixture
            .application
            .on_agent_notification(&completed_notification(session_id.clone(), invocation))
            .unwrap();
        let activation = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap()
            .remove(0);
        assert_eq!(activation.failure_stage.as_deref(), Some("file_resolution"));
        assert!(activation.launch_accepted_at.is_none());
        let history = fixture
            .application
            .sessions
            .load_session(&session_id)
            .unwrap();
        assert_eq!(
            history.invocations[0].invocation.status,
            AgentInvocationStatus::Completed
        );
    }

    #[test]
    fn receiver_is_fresh_once_then_continues_its_most_recent_session() {
        let fixture = execution_fixture(vec![expected_file_connection(
            "edge",
            "receiver",
            WorkflowExpectedFileSelector::FolderFilenamePattern {
                folder: "handoffs".to_string(),
                filename_pattern: "handoff.md".to_string(),
            },
            "Continue.",
        )]);
        let instance = launch_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.working_directory).join("handoffs");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("handoff.md"), "handoff").unwrap();
        let (session_id, invocation) = complete_source_turn(&fixture, &instance, "Done.");
        let notification = completed_notification(session_id, invocation);

        fixture
            .application
            .on_agent_notification(&notification)
            .unwrap();
        let first = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap()
            .remove(0);
        let first_target_invocation =
            AgentInvocationId::new(first.target_invocation_id.clone().unwrap()).unwrap();
        let now = Utc::now();
        fixture
            .agent_repository
            .finish_invocation(
                &first_target_invocation,
                InvocationCompletion {
                    status: AgentInvocationTerminalStatus::Completed,
                    completed_at: now,
                    exit_code: Some(0),
                    signal: None,
                    runtime_error: None,
                },
                now,
            )
            .unwrap();

        fixture
            .application
            .on_agent_notification(&notification)
            .unwrap();

        let activations = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap();
        assert_eq!(activations.len(), 2);
        assert_eq!(activations[0].session_mode.as_deref(), Some("fresh"));
        assert_eq!(activations[1].session_mode.as_deref(), Some("continued"));
        assert_eq!(
            activations[0].target_session_id,
            activations[1].target_session_id
        );
        assert_ne!(
            activations[0].target_invocation_id,
            activations[1].target_invocation_id
        );
    }

    #[test]
    fn trigger_uses_updated_activated_recipe_not_instance_launch_recipe() {
        let mut connection = expected_file_connection(
            "edge",
            "receiver",
            WorkflowExpectedFileSelector::FolderFilenamePattern {
                folder: "handoffs".to_string(),
                filename_pattern: "handoff.md".to_string(),
            },
            "Old prompt.",
        );
        let fixture = execution_fixture(vec![connection.clone()]);
        let instance = launch_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.working_directory).join("handoffs");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("handoff.md"), "handoff").unwrap();
        let Some(super::super::domain::WorkflowConnectionMechanism::TurnFinishedExpectedFile {
            prompt_text,
            ..
        }) = connection.mechanism.as_mut()
        else {
            unreachable!()
        };
        *prompt_text = "New prompt.".to_string();
        fixture
            .workflow_repository
            .save_connection_draft(&fixture.workflow_type_id, connection)
            .unwrap();
        let updated = fixture
            .workflow_repository
            .activate_changes(
                &fixture.workflow_type_id,
                &[WorkflowElementRef {
                    kind: WorkflowElementKind::Connection,
                    id: "edge".to_string(),
                }],
            )
            .unwrap();
        let updated_recipe_id = updated.active_recipe.unwrap().id;
        assert_ne!(updated_recipe_id, instance.recipe.id);
        let (session_id, invocation) = complete_source_turn(&fixture, &instance, "Done.");

        fixture
            .application
            .on_agent_notification(&completed_notification(session_id, invocation))
            .unwrap();

        let activation = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap()
            .remove(0);
        assert_eq!(activation.recipe_id, updated_recipe_id);
        assert!(fixture.runtime.launches.lock().unwrap()[1]
            .submitted_text
            .ends_with("New prompt."));
    }

    #[test]
    fn multiple_outgoing_connections_use_one_recipe_and_fail_independently() {
        let fixture = execution_fixture(vec![
            expected_file_connection(
                "edge-ok",
                "receiver-a",
                WorkflowExpectedFileSelector::FolderFilenamePattern {
                    folder: "handoffs".to_string(),
                    filename_pattern: "present.md".to_string(),
                },
                "Continue A.",
            ),
            expected_file_connection(
                "edge-missing",
                "receiver-b",
                WorkflowExpectedFileSelector::FolderFilenamePattern {
                    folder: "handoffs".to_string(),
                    filename_pattern: "missing.md".to_string(),
                },
                "Continue B.",
            ),
        ]);
        let instance = launch_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.working_directory).join("handoffs");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("present.md"), "present").unwrap();
        let (session_id, invocation) = complete_source_turn(&fixture, &instance, "Done.");

        assert_eq!(
            fixture
                .application
                .on_agent_notification(&completed_notification(session_id, invocation))
                .unwrap(),
            2
        );

        let activations = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap();
        assert_eq!(activations.len(), 2);
        assert!(activations
            .iter()
            .all(|activation| activation.recipe_id == instance.recipe.id));
        let succeeded = activations
            .iter()
            .find(|activation| activation.connection_id == "edge-ok")
            .unwrap();
        let failed = activations
            .iter()
            .find(|activation| activation.connection_id == "edge-missing")
            .unwrap();
        assert!(succeeded.launch_accepted_at.is_some());
        assert_eq!(failed.failure_stage.as_deref(), Some("file_resolution"));
        assert_eq!(fixture.runtime.launches.lock().unwrap().len(), 2);
    }

    #[test]
    fn same_receiver_fan_out_reuses_one_receiver_session_without_splitting_history() {
        let fixture = execution_fixture(vec![
            expected_file_connection(
                "edge-a",
                "receiver",
                WorkflowExpectedFileSelector::FolderFilenamePattern {
                    folder: "handoffs".to_string(),
                    filename_pattern: "handoff.md".to_string(),
                },
                "Continue A.",
            ),
            expected_file_connection(
                "edge-b",
                "receiver",
                WorkflowExpectedFileSelector::FolderFilenamePattern {
                    folder: "handoffs".to_string(),
                    filename_pattern: "handoff.md".to_string(),
                },
                "Continue B.",
            ),
        ]);
        let instance = launch_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.working_directory).join("handoffs");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("handoff.md"), "handoff").unwrap();
        let (session_id, invocation) = complete_source_turn(&fixture, &instance, "Done.");

        fixture
            .application
            .on_agent_notification(&completed_notification(session_id, invocation))
            .unwrap();

        let activations = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap();
        assert_eq!(activations.len(), 2);
        assert_eq!(
            activations[0].target_session_id,
            activations[1].target_session_id
        );
        assert_eq!(
            activations
                .iter()
                .filter(|activation| activation.session_mode.as_deref() == Some("fresh"))
                .count(),
            1
        );
        assert_eq!(
            fixture
                .application
                .load_workflow_instance(&instance.summary.id)
                .unwrap()
                .sessions
                .len(),
            2
        );
    }

    #[test]
    fn fresh_target_identity_is_durable_before_receiver_harness_binding_failure() {
        let fixture = execution_fixture_with_binder(
            vec![expected_file_connection(
                "edge",
                "receiver",
                WorkflowExpectedFileSelector::FolderFilenamePattern {
                    folder: "handoffs".to_string(),
                    filename_pattern: "handoff.md".to_string(),
                },
                "Continue.",
            )],
            Arc::new(ReceiverFailingHarnessBinder),
        );
        let instance = launch_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.working_directory).join("handoffs");
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("handoff.md"), "handoff").unwrap();
        let (session_id, invocation) = complete_source_turn(&fixture, &instance, "Done.");

        fixture
            .application
            .on_agent_notification(&completed_notification(session_id, invocation))
            .unwrap();

        let activation = fixture
            .workflow_repository
            .list_connection_activations(&instance.summary.id)
            .unwrap()
            .remove(0);
        assert!(activation.target_session_id.is_some());
        assert!(activation.target_invocation_id.is_some());
        assert_eq!(activation.session_mode.as_deref(), Some("fresh"));
        assert_eq!(activation.failure_stage.as_deref(), Some("harness_binding"));
        assert!(activation.associated_at.is_none());
        assert_eq!(
            fixture
                .application
                .load_workflow_instance(&instance.summary.id)
                .unwrap()
                .sessions
                .len(),
            1
        );
    }

    #[test]
    fn instance_projects_connection_activations_newest_first_with_exact_endpoints() {
        let fixture = execution_fixture(vec![expected_file_connection(
            "edge",
            "receiver",
            WorkflowExpectedFileSelector::FolderFilenamePattern {
                folder: "handoffs".to_string(),
                filename_pattern: "handoff.md".to_string(),
            },
            "Continue.",
        )]);
        let instance = launch_execution_instance(&fixture);
        let preparation = |id: &str, source: &str, invocation: &str, requested_at: &str| {
            WorkflowConnectionActivationPreparation {
                id: id.to_string(),
                workflow_instance_id: instance.summary.id.clone(),
                recipe_id: instance.recipe.id.clone(),
                connection_id: "edge".to_string(),
                sender_node_id: "sender".to_string(),
                receiver_node_id: "receiver".to_string(),
                source_session_id: source.to_string(),
                source_invocation_id: invocation.to_string(),
                requested_at: requested_at.to_string(),
            }
        };
        fixture
            .workflow_repository
            .create_connection_activation(preparation(
                "activation-older",
                "source-session-older",
                "source-invocation-older",
                "2026-08-09T01:00:00Z",
            ))
            .unwrap();
        fixture
            .workflow_repository
            .mark_connection_activation_failed(
                "activation-older",
                "file_resolution",
                "missing",
                "2026-08-09T01:00:01Z",
            )
            .unwrap();
        fixture
            .workflow_repository
            .create_connection_activation(preparation(
                "activation-newer",
                "source-session-newer",
                "source-invocation-newer",
                "2026-08-09T02:00:00Z",
            ))
            .unwrap();
        fixture
            .workflow_repository
            .mark_connection_activation_resolved(
                "activation-newer",
                "handoffs/handoff.md",
                "2026-08-09T02:00:01Z",
            )
            .unwrap();
        fixture
            .workflow_repository
            .reserve_connection_activation_target(
                "activation-newer",
                &instance.summary.id,
                "receiver",
                "target-session-exact",
                "target-invocation-exact",
                "fresh",
            )
            .unwrap();

        let projected = fixture
            .application
            .load_workflow_instance(&instance.summary.id)
            .unwrap();

        assert_eq!(
            projected
                .connection_activations
                .iter()
                .map(|activation| activation.id.as_str())
                .collect::<Vec<_>>(),
            vec!["activation-newer", "activation-older"]
        );
        let newer = &projected.connection_activations[0];
        assert_eq!(newer.status, WorkflowConnectionActivationStatus::Resolved);
        assert_eq!(newer.source_session_id, "source-session-newer");
        assert_eq!(newer.source_invocation_id, "source-invocation-newer");
        assert_eq!(
            newer.target_session_id.as_deref(),
            Some("target-session-exact")
        );
        assert_eq!(
            newer.target_invocation_id.as_deref(),
            Some("target-invocation-exact")
        );
        let older = &projected.connection_activations[1];
        assert_eq!(older.status, WorkflowConnectionActivationStatus::Failed);
        assert!(older.target_session_id.is_none());
        assert!(older.target_invocation_id.is_none());
    }
}
