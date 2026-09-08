use super::execution::WorkflowExecutionService;
use crate::{
    agent_sessions::{application::AgentSessionNotification, domain::NormalizedRuntimeEventKind},
    otp_api::*,
    otp_host::workflow::WorkflowHost,
};
use std::collections::BTreeSet;

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
        let session = crate::session_events::ReferenceIdentity::new(
            "orchestrator.agent_sessions",
            "session",
            session_id.as_str(),
        )
        .map_err(|e| e.to_string())?;
        let Some(entry) = self
            .directory
            .find_exact(&session)
            .map_err(|e| e.to_string())?
        else {
            return Ok(());
        };
        if !entry
            .logical_address
            .as_ref()
            .is_some_and(|a| a.scope.namespace() == "workflow" && a.scope.kind() == "instance")
        {
            return Ok(());
        }
        let (instance, source) = self.otp_source(session_id.as_str(), invocation.id.as_str())?;
        let history = self
            .sessions
            .load_session_history(session_id)
            .map_err(|e| e.to_string())?
            .ok_or("Completed Session is missing")?;
        let output = history
            .invocations
            .iter()
            .find(|i| i.invocation.id == invocation.id)
            .and_then(|i| {
                i.events.iter().rev().find_map(|event| {
                    let n = event.normalized.as_ref()?;
                    (n.kind == NormalizedRuntimeEventKind::AgentMessage
                        && n.details
                            .as_ref()
                            .and_then(|d| d.get("role"))
                            .and_then(|r| r.as_str())
                            == Some("final"))
                    .then(|| n.text.clone())
                    .flatten()
                })
            })
            .unwrap_or_default();
        let plan = self.compile_instance(&instance.id, None)?;
        let consumers = plan
            .connections
            .iter()
            .filter(|c| c.source_node.identity().id() == source.node_id)
            .map(|c| {
                (
                    c.trigger.capability.package.clone(),
                    c.trigger.capability.tool.clone(),
                )
            })
            .collect::<BTreeSet<_>>();
        let mut errors = vec![];
        for (package, tool) in consumers {
            let capability = CapabilityRef { package, tool };
            if !matches!(
                self.registry.tool(&capability)?.entrypoint,
                Entrypoint::SessionEvent {
                    event: SessionEventKind::InvocationTerminal
                }
            ) {
                continue;
            }
            let context = InvocationContext {
                instance_id: instance.id.clone(),
                occurrence_id: invocation.id.as_str().into(),
                capability,
                source: Some(source.clone()),
                connection_id: None,
                output_node_id: None,
            };
            let host = WorkflowHost {
                execution: self,
                instance: &instance,
                context: &context,
            };
            if let Err(error) = self.registry.invoke(
                &context,
                ToolInput::SessionEvent {
                    status: format!("{:?}", invocation.status).to_lowercase(),
                    output: output.clone(),
                },
                &host,
            ) {
                errors.push(error)
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}
