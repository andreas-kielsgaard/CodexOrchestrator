use super::{
    address_references::{
        WorkflowConnectionReference, WorkflowEventDefinitionReference, WorkflowRecipeReference,
    },
    compiled_plan::WorkflowConnectionTrigger,
    execution::WorkflowExecutionService,
};
use crate::{
    agent_sessions::{
        application::AgentSessionNotification,
        domain::{AgentInvocationStatus, NormalizedRuntimeEventKind},
    },
    session_events::{ReferenceIdentity, SessionEventOccurrence, SessionEventOccurrenceTrigger},
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

impl WorkflowExecutionService {
    pub(crate) fn on_agent_notification(
        &self,
        notification: &AgentSessionNotification,
    ) -> Result<(), String> {
        let AgentSessionNotification::InvocationTerminal {
            session_id,
            invocation,
        } = notification
        else {
            return Ok(());
        };
        if invocation.status != AgentInvocationStatus::Completed {
            return Ok(());
        }
        let session = ReferenceIdentity::new(
            "orchestrator.agent_sessions",
            "session",
            session_id.as_str(),
        )
        .map_err(|error| error.to_string())?;
        let Some(entry) = self
            .directory
            .find_exact(&session)
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let Some(address) = entry.logical_address else {
            return Ok(());
        };
        if address.scope.namespace() != "workflow" || address.scope.kind() != "instance" {
            return Ok(());
        }
        let history = self
            .sessions
            .load_session_history(session_id)
            .map_err(|error| error.to_string())?
            .ok_or("Completed Session is missing")?;
        let output = history
            .invocations
            .iter()
            .find(|item| item.invocation.id == invocation.id)
            .and_then(|item| {
                item.events.iter().rev().find_map(|event| {
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
            })
            .unwrap_or_default();
        self.receive_source_event(
            &session,
            invocation.id.as_str(),
            SessionEventOccurrenceTrigger::InvocationCompleted {
                session: session.clone(),
                session_address: Some(address),
                invocation: ReferenceIdentity::new(
                    "orchestrator.agent_sessions",
                    "invocation",
                    invocation.id.as_str(),
                )
                .map_err(|error| error.to_string())?,
                output,
            },
        )?;
        Ok(())
    }

    /// Managed adapters supply the trusted source Session; routing is derived from its address.
    pub(crate) fn receive_source_event(
        &self,
        source: &ReferenceIdentity,
        occurrence_id: &str,
        trigger: SessionEventOccurrenceTrigger,
    ) -> Result<usize, String> {
        self.route_source_event(source, occurrence_id, trigger, BTreeMap::new())
    }

    pub(crate) fn trigger_continuation(
        &self,
        source: &ReferenceIdentity,
        occurrence_id: &str,
        output_files: Vec<String>,
    ) -> Result<usize, String> {
        use super::compiled_plan::{input_reference, WorkflowConnectionPromptInput};
        let capability = super::trigger_capabilities::continuation();
        let content = [(
            input_reference(&WorkflowConnectionPromptInput::TriggerField {
                field: "outputFiles".into(),
            }),
            serde_json::to_string_pretty(&output_files).map_err(|error| error.to_string())?,
        )]
        .into_iter()
        .collect();
        self.route_source_event(
            source,
            occurrence_id,
            SessionEventOccurrenceTrigger::McpCall {
                call: ReferenceIdentity::new("mcp", "call", occurrence_id)
                    .map_err(|error| error.to_string())?,
                server: ReferenceIdentity::new("mcp", "server", capability.server)
                    .map_err(|error| error.to_string())?,
                tool: ReferenceIdentity::new("mcp", "tool", capability.tool)
                    .map_err(|error| error.to_string())?,
                arguments: BTreeMap::new(),
            },
            content,
        )
    }

    fn route_source_event(
        &self,
        source: &ReferenceIdentity,
        occurrence_id: &str,
        trigger: SessionEventOccurrenceTrigger,
        mut content: BTreeMap<ReferenceIdentity, String>,
    ) -> Result<usize, String> {
        let entry = self
            .directory
            .find_exact(source)
            .map_err(|error| error.to_string())?
            .ok_or("Source Session is missing")?;
        let address = entry
            .logical_address
            .ok_or("Source Session has no Workflow address")?;
        if address.scope.namespace() != "workflow"
            || address.scope.kind() != "instance"
            || address.subject.namespace() != "workflow"
            || address.subject.kind() != "node"
        {
            return Err("Source Session does not have a Workflow address".into());
        }
        let instance = self.instances.load(address.scope.id())?;
        let node = instance
            .recipe
            .nodes
            .iter()
            .find(|node| node.node_id == address.subject.id())
            .ok_or("Source node is absent from the instance recipe")?;
        content.insert(
            super::compiled_plan::input_reference(
                &super::compiled_plan::WorkflowConnectionPromptInput::TriggerField {
                    field: "sourceNode".into(),
                },
            ),
            serde_json::to_string_pretty(&serde_json::json!({"id":node.node_id,"name":node.name}))
                .map_err(|error| error.to_string())?,
        );
        let definitions = self.compile_instance(&instance.id, None)?;
        let mut dispatched = 0;
        let mut failures = Vec::new();
        for connection in &instance.recipe.connections {
            if connection.source_node_id != address.subject.id()
                || !matches_source(&connection.trigger, &trigger)
            {
                continue;
            }
            let reference = WorkflowEventDefinitionReference::for_connection(
                &WorkflowRecipeReference::new(&instance.recipe.recipe_id)
                    .map_err(|error| error.to_string())?,
                &WorkflowConnectionReference::new(&connection.connection_id)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?
            .into_identity();
            let definition = definitions
                .iter()
                .find(|definition| definition.definition_ref == reference)
                .ok_or("Connection definition is missing")?;
            let encoded = serde_json::to_vec(&(source, occurrence_id, &connection.connection_id))
                .map_err(|error| error.to_string())?;
            let identity = format!("occurrence-{:x}", Sha256::digest(encoded));
            // The attempt is durable before dispatch. A repeated notifier does not launch again.
            if self
                .instances
                .attempts(&instance.id)?
                .iter()
                .any(|attempt| attempt.id == identity)
            {
                continue;
            }
            let occurrence = SessionEventOccurrence {
                event_group_id: ReferenceIdentity::new("workflow", "event_group", identity)
                    .map_err(|error| error.to_string())?,
                trigger: trigger.clone(),
                referenced_content: content.clone(),
                created_by_session: Some(source.clone()),
                direct_user_options: None,
            };
            match self.dispatch_definition(&instance.id, definition, occurrence) {
                Ok(result) => {
                    for delivery in &result.deliveries {
                        match &delivery.outcome {
                            crate::session_events::DeliveryOutcome::Dispatched { .. } => {
                                dispatched += 1
                            }
                            crate::session_events::DeliveryOutcome::Failed { message } => {
                                failures.push(message.clone())
                            }
                        }
                    }
                }
                Err(error) => failures.push(error),
            }
        }
        if failures.is_empty() {
            Ok(dispatched)
        } else {
            Err(failures.join("; "))
        }
    }
}

fn matches_source(
    binding: &WorkflowConnectionTrigger,
    trigger: &SessionEventOccurrenceTrigger,
) -> bool {
    match (binding, trigger) {
        (
            WorkflowConnectionTrigger::InvocationCompleted,
            SessionEventOccurrenceTrigger::InvocationCompleted { .. },
        ) => true,
        (
            WorkflowConnectionTrigger::McpCall { server, tool },
            SessionEventOccurrenceTrigger::McpCall {
                server: actual_server,
                tool: actual_tool,
                ..
            },
        ) => server == actual_server && tool == actual_tool,
        (
            WorkflowConnectionTrigger::ApplicationEvent { event_kind },
            SessionEventOccurrenceTrigger::ApplicationEvent {
                event_kind: actual_kind,
                ..
            },
        ) => event_kind == actual_kind,
        _ => false,
    }
}
