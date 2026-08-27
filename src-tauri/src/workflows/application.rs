use super::domain::{
    PreparedWorkflowMcpHandoff, WorkflowCompletedTurnTrigger, WorkflowConnectionActivation,
    WorkflowConnectionActivationPreparation, WorkflowConnectionActivationRecord,
    WorkflowConnectionActivationStatus, WorkflowConnectionConfig, WorkflowDefinition,
    WorkflowElementRef, WorkflowHarnessConfig, WorkflowInvocation, WorkflowMcpActivationContext,
    WorkflowMcpComponent, WorkflowMcpOutput, WorkflowNativeQuery, WorkflowNodeConfig, WorkflowRole,
    WorkflowTypeSummary,
};
use super::instance_domain::{
    CreateWorkflowInstancePreparation, ResolvedRepoBranchWorktreeTarget, WorkflowInstance,
    WorkflowInstanceRecord, WorkflowInstanceSession, WorkflowInstanceSummary,
    WorkflowSessionActivity,
};
use super::legacy_node_configuration::WorkflowNodeConfigurationSource;
use super::node_sessions::{WorkflowHumanMessage, WorkflowNodeSessions};
use crate::agent_sessions::{
    application::{
        AgentSessionApplication, AgentSessionNotification, SendAgentSessionMessageResult,
    },
    domain::{
        AgentInvocation, AgentInvocationId, AgentInvocationStatus, AgentSessionId,
        NormalizedRuntimeEventKind,
    },
};
use chrono::Utc;
use globset::Glob;
use regex::Regex;
use std::{
    cmp::Reverse,
    fs,
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};
use uuid::Uuid;

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
    fn create_instance(
        &self,
        preparation: CreateWorkflowInstancePreparation,
    ) -> Result<WorkflowInstanceRecord, String>;
    fn associate_instance_session(
        &self,
        workflow_instance_id: &str,
        node_id: &str,
        session_id: &str,
        associated_at: &str,
    ) -> Result<WorkflowInstanceRecord, String>;
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
    pub(crate) workflow_type_id: String,
    pub(crate) recipe_id: String,
    pub(crate) node_id: String,
    pub(crate) harness: WorkflowHarnessConfig,
}

pub(crate) trait WorkflowSessionHarnessBinder: Send + Sync {
    fn bind_workflow_session(
        &self,
        request: BindWorkflowSessionHarness,
    ) -> Result<crate::harness_engine::domain::HarnessVersionRef, String>;
}

pub(crate) struct WorkflowApplication {
    repository: Arc<dyn WorkflowRepository>,
    sessions: Arc<AgentSessionApplication>,
    node_sessions: WorkflowNodeSessions,
}

impl WorkflowApplication {
    pub(crate) fn new(
        repository: Arc<dyn WorkflowRepository>,
        sessions: Arc<AgentSessionApplication>,
        harnesses: Arc<dyn WorkflowSessionHarnessBinder>,
        node_configuration: Arc<dyn WorkflowNodeConfigurationSource>,
    ) -> Self {
        let node_sessions = WorkflowNodeSessions::new(
            repository.clone(),
            sessions.clone(),
            harnesses,
            node_configuration,
        );
        Self {
            repository,
            sessions,
            node_sessions,
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

    pub(crate) fn create_workflow_instance(
        &self,
        workflow_type_id: &str,
        name: &str,
        target: ResolvedRepoBranchWorktreeTarget,
    ) -> Result<WorkflowInstance, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("A Workflow instance name is required.".to_string());
        }
        validate_target(&target)?;
        let definition = self.repository.load_workflow_type(workflow_type_id)?;
        let recipe = definition
            .active_recipe
            .ok_or_else(|| "Activate the Workflow type before creating an instance.".to_string())?;
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
        let instance_id = format!("workflow-instance-{}", Uuid::new_v4());
        let preparation = CreateWorkflowInstancePreparation {
            instance_id: instance_id.clone(),
            workflow_type_id: workflow_type_id.to_string(),
            recipe_id: recipe.id.clone(),
            name: name.to_string(),
            target,
            created_at: Utc::now().to_rfc3339(),
        };
        self.repository.create_instance(preparation)?;
        self.load_workflow_instance(&instance_id)
    }

    pub(crate) fn send_workflow_node_message(
        &self,
        workflow_instance_id: &str,
        node_id: &str,
        submitted_text: String,
        title: Option<String>,
    ) -> Result<SendAgentSessionMessageResult, String> {
        self.node_sessions.send_human_message(
            workflow_instance_id,
            node_id,
            WorkflowHumanMessage {
                submitted_text,
                title,
            },
        )
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
            Path::new(&trigger.worktree_root),
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
        self.node_sessions.deliver_connection_message(
            activation_id,
            trigger,
            receiver_node_id,
            prompt,
        )
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
            created_at: record.created_at.clone(),
        };
        Ok(WorkflowInstance {
            summary,
            target: record.target,
            recipe: record.recipe,
            sessions,
            connection_activations: connection_activations
                .into_iter()
                .map(project_connection_activation)
                .collect(),
        })
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

fn validate_target(target: &ResolvedRepoBranchWorktreeTarget) -> Result<(), String> {
    for (value, label) in [
        (&target.repository.id, "repository ID"),
        (&target.repository.name, "repository name"),
        (
            &target.repository.git_common_directory,
            "repository Git identity",
        ),
        (&target.branch.id, "branch ID"),
        (&target.branch.name, "branch name"),
        (&target.worktree.id, "worktree ID"),
        (&target.worktree.path, "worktree path"),
    ] {
        if value.trim().is_empty() {
            return Err(format!("A Workflow instance target {label} is required."));
        }
    }
    Ok(())
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
                WorkflowInitialCheck, WorkflowMatchSelection, WorkflowNodeHarness,
            },
            instance_domain::{
                ResolvedRepoBranchWorktreeTarget, WorkflowBranchTarget, WorkflowRepositoryTarget,
                WorkflowWorktreeTarget,
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
        fn bind_workflow_session(
            &self,
            _: BindWorkflowSessionHarness,
        ) -> Result<crate::harness_engine::domain::HarnessVersionRef, String> {
            Ok(recording_harness_reference())
        }
    }

    struct ReceiverFailingHarnessBinder;

    impl WorkflowSessionHarnessBinder for ReceiverFailingHarnessBinder {
        fn bind_workflow_session(
            &self,
            request: BindWorkflowSessionHarness,
        ) -> Result<crate::harness_engine::domain::HarnessVersionRef, String> {
            if request.node_id == "sender" {
                Ok(recording_harness_reference())
            } else {
                Err("receiver Harness binding failed".to_string())
            }
        }
    }

    fn recording_harness_reference() -> crate::harness_engine::domain::HarnessVersionRef {
        crate::harness_engine::domain::HarnessVersionRef::new(
            crate::harness_engine::domain::HarnessId::new("harness-workflow-test").unwrap(),
            crate::harness_engine::domain::HarnessVersionNumber::new(1).unwrap(),
        )
    }

    struct RecordingHarnessResolver;

    impl crate::agent_sessions::application::SessionHarnessVersionResolver
        for RecordingHarnessResolver
    {
        fn resolve_session_harness_version(
            &self,
            _: &AgentSessionId,
            requested: &crate::harness_engine::domain::HarnessVersionRef,
        ) -> Result<crate::harness_engine::domain::HarnessVersionRef, String> {
            Ok(requested.clone())
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
        let sessions = Arc::new(
            AgentSessionApplication::new(
                agent_repository,
                runtime.clone(),
                Arc::new(NoopNotifier),
                providers.clone(),
                providers,
                Some("codex-test".to_string()),
            )
            .with_session_harness_version_resolver(Arc::new(RecordingHarnessResolver)),
        );
        let workflows = Arc::new(SqliteWorkflowRepository::open(&database_path).unwrap());
        let application = WorkflowApplication::new(
            workflows.clone(),
            sessions,
            harnesses,
            Arc::new(
                super::super::legacy_node_configuration::LegacyWorkflowNodeConfigurationSource,
            ),
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

    fn target(path: &Path) -> ResolvedRepoBranchWorktreeTarget {
        ResolvedRepoBranchWorktreeTarget {
            repository: WorkflowRepositoryTarget {
                id: "repo-1".to_string(),
                name: "Codex Orchestrator".to_string(),
                git_common_directory: path.to_string_lossy().into_owned(),
            },
            branch: WorkflowBranchTarget {
                id: "branch-1".to_string(),
                name: "codex/workflow-engine-v1".to_string(),
            },
            worktree: WorkflowWorktreeTarget {
                id: "worktree-1".to_string(),
                path: path.to_string_lossy().into_owned(),
            },
        }
    }

    #[test]
    fn instance_creation_registers_target_without_session_or_runtime_launch() {
        let (directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig::test_definition("Reviewer", "", "", "gpt-5.6-sol", "high"),
        );
        let selected_worktree = directory.path().join("selected-worktree");

        let instance = application
            .create_workflow_instance(
                &workflow_type_id,
                "Architecture review",
                target(&selected_worktree),
            )
            .unwrap();

        assert_eq!(instance.summary.name, "Architecture review");
        assert_eq!(instance.summary.session_count, 0);
        assert!(instance.sessions.is_empty());
        assert_eq!(
            instance.target.worktree.path,
            selected_worktree.to_string_lossy()
        );
        assert!(!selected_worktree.exists());
        assert!(runtime.launches.lock().unwrap().is_empty());
    }

    #[test]
    fn first_human_message_creates_bound_session_and_user_invocation() {
        let (directory, repository, runtime, application) = fixture();
        let workflow_type_id = activate_start(
            &repository,
            WorkflowHarnessConfig::test_definition(
                "Architecture reviewer",
                "",
                "Review carefully.",
                "gpt-5.6-sol",
                "high",
            ),
        );
        let instance = application
            .create_workflow_instance(
                &workflow_type_id,
                "Architecture review",
                target(directory.path()),
            )
            .unwrap();
        let prompt = "Identify the main architectural risk.";

        let acknowledgement = application
            .send_workflow_node_message(
                &instance.summary.id,
                "start",
                prompt.to_string(),
                Some("Architecture reviewer".to_string()),
            )
            .unwrap();
        let loaded = application
            .load_workflow_instance(&instance.summary.id)
            .unwrap();

        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(
            loaded.sessions[0].session_id,
            acknowledgement.session_id.as_str()
        );
        let history = application
            .sessions
            .load_session(&acknowledgement.session_id)
            .unwrap();
        assert_eq!(history.invocations.len(), 1);
        assert_eq!(
            history.invocations[0].invocation.id,
            acknowledgement.invocation_id
        );
        assert_eq!(history.invocations[0].invocation.submitted_text, prompt);
        assert_eq!(
            history.session.working_directory.as_deref(),
            Some(directory.path().to_string_lossy().as_ref())
        );
        assert_eq!(
            history.session.harness_version,
            Some(recording_harness_reference())
        );
        assert_eq!(runtime.launches.lock().unwrap().len(), 1);
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
        let sessions = Arc::new(
            AgentSessionApplication::new(
                agent_repository.clone(),
                runtime.clone(),
                Arc::new(NoopNotifier),
                providers.clone(),
                providers,
                Some("codex-test".to_string()),
            )
            .with_session_harness_version_resolver(Arc::new(RecordingHarnessResolver)),
        );
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
            Arc::new(
                super::super::legacy_node_configuration::LegacyWorkflowNodeConfigurationSource,
            ),
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

    fn create_started_execution_instance(fixture: &ExecutionFixture) -> WorkflowInstance {
        let instance = fixture
            .application
            .create_workflow_instance(
                &fixture.workflow_type_id,
                "Execution instance",
                target(fixture._directory.path()),
            )
            .unwrap();
        let acknowledgement = fixture
            .application
            .send_workflow_node_message(
                &instance.summary.id,
                "sender",
                "Produce the handoff.".to_string(),
                Some("Execution instance".to_string()),
            )
            .unwrap();
        let loaded = fixture
            .application
            .load_workflow_instance(&instance.summary.id)
            .unwrap();
        assert_eq!(
            loaded.sessions[0].session_id,
            acknowledgement.session_id.as_str()
        );
        loaded
    }

    #[test]
    fn human_node_message_accepts_a_valid_non_start_node_at_the_application_boundary() {
        let fixture = execution_fixture(vec![mcp_native_connection("edge", "receiver")]);
        let instance = fixture
            .application
            .create_workflow_instance(
                &fixture.workflow_type_id,
                "Non-start delivery",
                target(fixture._directory.path()),
            )
            .unwrap();

        let acknowledgement = fixture
            .application
            .send_workflow_node_message(
                &instance.summary.id,
                "receiver",
                "Begin directly here.".to_string(),
                None,
            )
            .unwrap();
        let loaded = fixture
            .application
            .load_workflow_instance(&instance.summary.id)
            .unwrap();

        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].node_id, "receiver");
        assert_eq!(
            loaded.sessions[0].session_id,
            acknowledgement.session_id.as_str()
        );
    }

    fn initial_invocation_id(
        fixture: &ExecutionFixture,
        instance: &WorkflowInstance,
    ) -> AgentInvocationId {
        let session_id = AgentSessionId::new(instance.sessions[0].session_id.clone()).unwrap();
        fixture
            .application
            .sessions
            .load_session(&session_id)
            .unwrap()
            .invocations[0]
            .invocation
            .id
            .clone()
    }

    fn complete_source_turn(
        fixture: &ExecutionFixture,
        instance: &WorkflowInstance,
        final_output: &str,
    ) -> (AgentSessionId, AgentInvocation) {
        let session_id = AgentSessionId::new(instance.sessions[0].session_id.clone()).unwrap();
        let invocation_id = initial_invocation_id(fixture, instance);
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
        let instance = create_started_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.target.worktree.path).join("handoffs");
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
        let instance = create_started_execution_instance(&fixture);
        let source_session_id = instance.sessions[0].session_id.clone();
        let source_invocation_id = initial_invocation_id(&fixture, &instance).to_string();

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
        let ordinary_instance = create_started_execution_instance(&ordinary);
        let ordinary_invocation_id = initial_invocation_id(&ordinary, &ordinary_instance);
        assert!(ordinary
            .application
            .prepare_mcp_native_handoff(
                &ordinary_instance.summary.id,
                "sender",
                &ordinary_instance.sessions[0].session_id,
                ordinary_invocation_id.as_str(),
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
        let instance = create_started_execution_instance(&fixture);
        let source_invocation_id = initial_invocation_id(&fixture, &instance);
        let prepared = fixture
            .application
            .prepare_mcp_native_handoff(
                &instance.summary.id,
                "sender",
                &instance.sessions[0].session_id,
                source_invocation_id.as_str(),
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
        let instance = create_started_execution_instance(&fixture);
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

        let source_invocation_id = initial_invocation_id(&fixture, &instance);
        let prepared = fixture
            .application
            .prepare_mcp_native_handoff(
                &instance.summary.id,
                "sender",
                &instance.sessions[0].session_id,
                source_invocation_id.as_str(),
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
        let instance = create_started_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.target.worktree.path).join("handoffs");
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
        let instance = create_started_execution_instance(&fixture);
        fs::create_dir_all(PathBuf::from(&instance.target.worktree.path).join("handoffs")).unwrap();
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
        let instance = create_started_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.target.worktree.path).join("handoffs");
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
    fn trigger_uses_updated_activated_recipe_not_instance_creation_recipe() {
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
        let instance = create_started_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.target.worktree.path).join("handoffs");
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
        let instance = create_started_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.target.worktree.path).join("handoffs");
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
        let instance = create_started_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.target.worktree.path).join("handoffs");
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
        let instance = create_started_execution_instance(&fixture);
        let folder = PathBuf::from(&instance.target.worktree.path).join("handoffs");
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
        let instance = create_started_execution_instance(&fixture);
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
