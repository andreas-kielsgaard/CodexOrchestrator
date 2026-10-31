use super::execution::WorkflowExecutionService;
use crate::{
    agent_sessions::application::AgentSessionNotification, otp_api::*,
    otp_host::workflow::WorkflowHost,
};
use std::collections::BTreeSet;

impl WorkflowExecutionService {
    pub fn completion_sound_for_session(&self, session: &str) -> Option<bool> {
        let reference = crate::session_events::ReferenceIdentity::new(
            "orchestrator.agent_sessions",
            "session",
            session,
        )
        .ok()?;
        let address = self
            .directory
            .find_exact(&reference)
            .ok()??
            .logical_address?;
        if address.scope.namespace() != "workflow" || address.scope.kind() != "instance" {
            return None;
        }
        let instance = self.instances.load(address.scope.id()).ok()?;
        instance
            .recipe
            .nodes
            .iter()
            .find(|node| node.node_id == address.subject.id())
            .and_then(|node| {
                node.agent()
                    .ok()
                    .map(|agent| agent.notifications.turn_completion_sound)
            })
    }
    pub fn on_agent_notification(
        &self,
        notification: &AgentSessionNotification,
    ) -> Result<(), String> {
        let changed_session = match notification {
            AgentSessionNotification::EventPersisted { session_id, .. }
            | AgentSessionNotification::InvocationTerminal { session_id, .. } => Some(session_id),
            _ => None,
        };
        if let Some(session) = changed_session {
            self.session_events.reconcile_steering(
                &crate::session_events::ReferenceIdentity::new(
                    "orchestrator.agent_sessions",
                    "session",
                    session.as_str(),
                )
                .map_err(|e| e.to_string())?,
            )?;
        }
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
        for mut attempt in self.instances.attempts(&instance.id)? {
            let mut changed = false;
            for (ordinal, group) in attempt.event_groups.iter_mut().enumerate() {
                if !group
                    .deliveries
                    .iter()
                    .any(|d| d.target_session.id() == session_id.as_str())
                {
                    continue;
                }
                let Some(request) = attempt.session_requests.get(ordinal) else {
                    continue;
                };
                let allow = self
                    .capability_runtimes
                    .allow_delivery_retry(&attempt.context, request)?;
                if let Some(updated) = self
                    .session_events
                    .retry_expired_delivery(&group.group.event_group_id, allow)?
                {
                    self.capability_runtimes.delivered(
                        &attempt.context,
                        &attempt.id,
                        ordinal,
                        request,
                        &updated,
                    )?;
                    *group = updated;
                    changed = true;
                } else if let Some(updated) = self
                    .session_events
                    .recorded_result(&group.group.event_group_id)?
                {
                    self.capability_runtimes.delivered(
                        &attempt.context,
                        &attempt.id,
                        ordinal,
                        request,
                        &updated,
                    )?;
                    *group = updated;
                    changed = true;
                }
            }
            if changed {
                self.instances.update_attempt(&attempt)?;
            }
        }
        let history = self
            .sessions
            .load_session_history(session_id)
            .map_err(|e| e.to_string())?
            .ok_or("Completed Session is missing")?;
        let output = history
            .invocations
            .iter()
            .find(|i| i.invocation.id == invocation.id)
            .and_then(|i| i.final_reply())
            .unwrap_or_default()
            .to_owned();
        let plan = self.compile_graph(&instance.id)?;
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
