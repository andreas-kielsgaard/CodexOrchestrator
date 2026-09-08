use crate::{
    otp_api::*,
    session_events::*,
    workflows::{
        compiled_plan::WorkflowSessionCreation,
        execution::{node_address, WorkflowExecutionService},
        instances::{RecipeInstance, WorkflowEventAttempt},
    },
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub(crate) struct WorkflowHost<'a> {
    pub execution: &'a WorkflowExecutionService,
    pub instance: &'a RecipeInstance,
    pub context: &'a InvocationContext,
}
impl WorkflowHost<'_> {
    fn check(&self, context: &InvocationContext) -> Result<(), String> {
        if context != self.context || context.instance_id != self.instance.id {
            return Err("OTP context does not match this invocation".into());
        }
        Ok(())
    }
    fn check_node(&self, context: &InvocationContext, node: &str) -> Result<(), String> {
        self.check(context)?;
        if context.output_node_id.as_deref() != Some(node)
            && context.source.as_ref().map(|s| s.node_id.as_str()) != Some(node)
        {
            return Err("Node is not bound to this OTP invocation".into());
        }
        Ok(())
    }
}
impl OtpHost for WorkflowHost<'_> {
    fn node(&self, context: &InvocationContext, node_id: &str) -> Result<NodeDefinition, String> {
        self.check_node(context, node_id)?;
        let node = self
            .instance
            .recipe
            .nodes
            .iter()
            .find(|n| n.node_id == node_id)
            .ok_or("Bound node is missing")?;
        Ok(NodeDefinition {
            id: node.node_id.clone(),
            name: node.name.clone(),
            initial_prompt: node.initial_prompt.clone(),
            configuration: json!({"capabilityProfileId":node.capability_profile_id,"nodeProfile":node.node_profile,"agentIdentityId":node.agent_identity_id}),
        })
    }
    fn connection(
        &self,
        context: &InvocationContext,
    ) -> Result<Option<ConnectionDefinition>, String> {
        self.check(context)?;
        context
            .connection_id
            .as_ref()
            .map(|id| {
                let edge = self
                    .instance
                    .recipe
                    .connections
                    .iter()
                    .find(|e| &e.connection_id == id)
                    .ok_or("Connection is missing")?;
                Ok(ConnectionDefinition {
                    id: id.clone(),
                    configuration: serde_json::to_value(edge).map_err(|e| e.to_string())?,
                })
            })
            .transpose()
    }
    fn sessions(
        &self,
        context: &InvocationContext,
        node_id: &str,
    ) -> Result<Vec<NodeSession>, String> {
        self.check_node(context, node_id)?;
        Ok(self
            .execution
            .directory
            .list_at_address(&node_address(&self.instance.id, node_id)?)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|s| NodeSession {
                id: s.session.id().into(),
                running: s.running,
                created_sequence: s.created_sequence,
                last_addressed_sequence: s.last_addressed_sequence,
                created_by_event: s.created_by_event.map(|r| r.to_string()),
                created_by_session: s.created_by_session.map(|r| r.to_string()),
            })
            .collect())
    }
    fn emit(
        &self,
        context: &InvocationContext,
        output: &str,
        payload: Value,
    ) -> Result<RoutingReceipt, String> {
        self.check(context)?;
        self.execution
            .route_otp_output(self.instance, context, output, payload)
    }
}

impl WorkflowExecutionService {
    pub(crate) fn dispatch_otp_action(
        &self,
        instance: &RecipeInstance,
        context: &InvocationContext,
        output: Option<OutputRef>,
        payload: Value,
        configuration: Value,
        inputs: Result<Vec<ResolvedInput>, String>,
    ) -> Result<Vec<SessionEventResult>, String> {
        let identity = serde_json::to_vec(&(
            context.instance_id.as_str(),
            &context.source,
            context.occurrence_id.as_str(),
            &context.connection_id,
            &context.capability,
            &context.output_node_id,
            &output,
        ))
        .map_err(|e| e.to_string())?;
        let id = format!("otp-attempt-{:x}", Sha256::digest(identity));
        let definition_ref = match &context.connection_id {
            Some(id) => reference("workflow", "connection", id)?,
            None => reference(
                "workflow",
                "node",
                context
                    .output_node_id
                    .as_deref()
                    .ok_or("Missing entry node")?,
            )?,
        };
        let mut attempt = WorkflowEventAttempt {
            id,
            instance_id: instance.id.clone(),
            definition_ref,
            context: context.clone(),
            output,
            payload,
            created_at: chrono::Utc::now().to_rfc3339(),
            session_requests: vec![],
            event_groups: vec![],
            error: None,
        };
        if !self.instances.begin_attempt(&attempt)? {
            return Ok(vec![]);
        }
        let mut results = Vec::new();
        let run = (|| -> Result<(), String> {
            let host = WorkflowHost {
                execution: self,
                instance,
                context,
            };
            let result = self.registry.invoke(
                context,
                ToolInput::Action {
                    configuration,
                    inputs: inputs?,
                },
                &host,
            )?;
            let descriptor = self.registry.tool(&context.capability)?;
            if !result.session_requests.is_empty()
                && !descriptor
                    .outputs
                    .iter()
                    .any(|o| o.kind == OutputKind::SessionRequest)
            {
                return Err("Tool did not declare a Session Request output".into());
            }
            attempt.session_requests = result.session_requests;
            // Store the selected requests before any external invocation is launched.
            self.instances.update_attempt(&attempt)?;
            for (ordinal, request) in attempt.session_requests.iter().enumerate() {
                let dispatched =
                    self.execute_session_request(instance, context, &attempt, ordinal, request)?;
                attempt
                    .event_groups
                    .push(dispatched.group.event_group_id.clone());
                self.instances.update_attempt(&attempt)?;
                results.push(dispatched);
            }
            Ok(())
        })();
        attempt.error = run.as_ref().err().cloned();
        self.instances.update_attempt(&attempt)?;
        if let Some(observer) = &self.record_observer {
            observer(&instance.id)
        }
        run?;
        Ok(results)
    }

    fn execute_session_request(
        &self,
        instance: &RecipeInstance,
        context: &InvocationContext,
        attempt: &WorkflowEventAttempt,
        ordinal: usize,
        request: &SessionRequest,
    ) -> Result<SessionEventResult, String> {
        if context.output_node_id.as_deref() != Some(request.node_id.as_str()) {
            return Err("Session request targets an unbound node".into());
        }
        let address = node_address(&instance.id, &request.node_id)?;
        let plan = self.compile_instance(&instance.id, None)?;
        let node = plan
            .nodes
            .iter()
            .find(|n| n.reference.identity().id() == request.node_id)
            .ok_or("Destination node is missing")?;
        let fresh = matches!(request.target, SessionRequestTarget::New);
        let target = match &request.target {
            SessionRequestTarget::New => SessionTarget::New {
                address: address.clone(),
            },
            SessionRequestTarget::Exact { session_id } => {
                let session = reference("orchestrator.agent_sessions", "session", session_id)?;
                let stored = self
                    .directory
                    .find_exact(&session)
                    .map_err(|e| e.to_string())?
                    .ok_or("Destination Session is missing")?;
                if stored.logical_address.as_ref() != Some(&address) {
                    return Err("Exact Session belongs to another Workflow node".into());
                }
                SessionTarget::Exact { session }
            }
        };
        let (kind, value) = match &node.session_creation {
            WorkflowSessionCreation::ResolvedInput(input) => {
                ("session_creation_request", serde_json::to_value(input))
            }
            WorkflowSessionCreation::AtBirth(input) => {
                ("session_creation_intent", serde_json::to_value(input))
            }
        };
        let creation = fresh
            .then(|| -> Result<SessionCreationConfiguration, String> {
                Ok(SessionCreationConfiguration {
                    contract: reference("orchestrator.execution_configuration", kind, "v1")?,
                    payload: value.map_err(|e| e.to_string())?,
                    assigned_identity: node.assigned_identity.clone(),
                })
            })
            .transpose()?;
        let cause = reference("otp", "occurrence", &context.occurrence_id)?;
        let (trigger, source, created_by_session) = match &context.source {
            Some(source) => (
                SessionEventTrigger::ApplicationEvent {
                    event: cause.clone(),
                },
                SessionEventSource::SessionInvocation {
                    session: reference(
                        "orchestrator.agent_sessions",
                        "session",
                        &source.session_id,
                    )?,
                    invocation: reference(
                        "orchestrator.agent_sessions",
                        "invocation",
                        &source.invocation_id,
                    )?,
                },
                Some(reference(
                    "orchestrator.agent_sessions",
                    "session",
                    &source.session_id,
                )?),
            ),
            None => (
                SessionEventTrigger::UserRequest {
                    request: cause.clone(),
                },
                SessionEventSource::UserRequest { request: cause },
                None,
            ),
        };
        let prompt_sources = request
            .prompt
            .iter()
            .filter(|p| !p.text.trim().is_empty())
            .map(|p| {
                Ok(PromptSource::ReferencedContent {
                    reference: reference("otp", "prompt_input", &p.reference)?,
                    text: p.text.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let created_session_prompt_sources = if fresh {
            node.initial_prompt
                .as_ref()
                .map(|text| {
                    vec![PromptSource::ReferencedContent {
                        reference: reference(
                            "workflow",
                            "initial_prompt",
                            &format!(
                                "{}/{}/{}",
                                instance.recipe.recipe_id,
                                instance.recipe.revision,
                                request.node_id
                            ),
                        )
                        .expect("nonempty prompt identity"),
                        text: text.clone(),
                    }]
                })
                .unwrap_or_default()
        } else {
            vec![]
        };
        self.session_events
            .dispatch(SessionEventCommand {
                event_group_id: reference(
                    "workflow",
                    "event_group",
                    &format!("{}-{}", attempt.id, ordinal),
                )?,
                definition_ref: attempt.definition_ref.clone(),
                trigger,
                source,
                prompt_sources,
                created_session_prompt_sources,
                creation_configuration: creation,
                target: TargetSelection {
                    target,
                    cardinality: TargetCardinality::First,
                    ordering: TargetOrdering::Newest,
                    running: RunningFilter::Any,
                    created_by: None,
                    missing: MissingTargetPolicy::Fail,
                },
                created_by_session,
                direct_user_options: None,
            })
            .map_err(|e| e.to_string())
    }

    pub(crate) fn invoke_mcp(
        &self,
        package: &str,
        tool: &str,
        session_id: &str,
        invocation_id: &str,
        arguments: Value,
    ) -> Result<ToolResult, String> {
        let session = crate::agent_sessions::domain::AgentSessionId::new(session_id)
            .map_err(|e| e.to_string())?;
        let history = self
            .sessions
            .load_session_history(&session)
            .map_err(|e| e.to_string())?
            .ok_or("Source Session is missing")?;
        if !history
            .invocations
            .iter()
            .any(|i| i.invocation.id.as_str() == invocation_id && i.invocation.status.is_active())
        {
            return Err("OTP call requires the Session's current invocation".into());
        }
        let profile = history
            .session
            .session_profile
            .ok_or("Source Session has no pinned profile")?;
        if !profile
            .session_profile()
            .node_capabilities()
            .mcp_tools
            .get(package)
            .is_some_and(|tools| tools.contains(tool))
        {
            return Err("OTP tool is not exposed to this Session".into());
        }
        let (instance, source) = self.otp_source(session_id, invocation_id)?;
        let context = InvocationContext {
            instance_id: instance.id.clone(),
            occurrence_id: format!("mcp-call-{}", uuid::Uuid::new_v4()),
            capability: CapabilityRef {
                package: package.into(),
                tool: tool.into(),
            },
            source: Some(source),
            connection_id: None,
            output_node_id: None,
        };
        self.registry.invoke(
            &context,
            ToolInput::Mcp(arguments),
            &WorkflowHost {
                execution: self,
                instance: &instance,
                context: &context,
            },
        )
    }

    pub(crate) fn otp_source(
        &self,
        session_id: &str,
        invocation_id: &str,
    ) -> Result<(RecipeInstance, SourceContext), String> {
        let entry = self
            .directory
            .find_exact(&reference(
                "orchestrator.agent_sessions",
                "session",
                session_id,
            )?)
            .map_err(|e| e.to_string())?
            .ok_or("Source Session is missing")?;
        let address = entry
            .logical_address
            .ok_or("Source Session has no Workflow node address")?;
        if address.scope.namespace() != "workflow"
            || address.scope.kind() != "instance"
            || address.subject.namespace() != "workflow"
            || address.subject.kind() != "node"
        {
            return Err("Source Session is not bound to a Workflow node".into());
        }
        let instance = self.instances.load(address.scope.id())?;
        let node = instance
            .recipe
            .nodes
            .iter()
            .find(|n| n.node_id == address.subject.id())
            .ok_or("Source node is missing from the instance")?;
        let source = SourceContext {
            node_id: node.node_id.clone(),
            node_name: node.name.clone(),
            session_id: session_id.into(),
            invocation_id: invocation_id.into(),
        };
        Ok((instance, source))
    }

    pub(crate) fn route_otp_output(
        &self,
        instance: &RecipeInstance,
        context: &InvocationContext,
        output_id: &str,
        payload: Value,
    ) -> Result<RoutingReceipt, String> {
        let producer = self.registry.tool(&context.capability)?;
        let output = producer
            .outputs
            .iter()
            .find(|o| o.id == output_id && o.kind == OutputKind::Data)
            .ok_or("OTP emitted an undeclared data output")?;
        crate::otp_api::validate_json(&output.schema, &payload)?;
        let source = context
            .source
            .as_ref()
            .ok_or("A Workflow trigger requires a source node")?;
        let trigger = OutputRef {
            capability: context.capability.clone(),
            output: output_id.into(),
        };
        let plan = self.compile_instance(&instance.id, None)?;
        let mut deliveries = 0;
        let mut failures = vec![];
        for connection in plan
            .connections
            .iter()
            .filter(|c| c.source_node.identity().id() == source.node_id && c.trigger == trigger)
        {
            let action = InvocationContext {
                capability: connection.action.clone(),
                connection_id: Some(connection.reference.identity().id().into()),
                output_node_id: Some(connection.destination_node.identity().id().into()),
                ..context.clone()
            };
            let result = (|| {
                let inputs = crate::workflows::prompt_content::resolve_inputs(
                    instance,
                    connection,
                    &payload,
                    self.sessions.as_ref(),
                );
                self.dispatch_otp_action(
                    instance,
                    &action,
                    Some(trigger.clone()),
                    payload.clone(),
                    connection.configuration.clone(),
                    inputs,
                )
            })();
            match result {
                Ok(results) => {
                    for result in results {
                        for delivery in result.deliveries {
                            match delivery.outcome {
                                DeliveryOutcome::Dispatched { .. } => deliveries += 1,
                                DeliveryOutcome::Failed { message } => failures.push(message),
                            }
                        }
                    }
                }
                Err(error) => failures.push(error),
            }
        }
        if failures.is_empty() {
            Ok(RoutingReceipt { deliveries })
        } else {
            Err(failures.join("; "))
        }
    }
}
fn reference(namespace: &str, kind: &str, id: &str) -> Result<ReferenceIdentity, String> {
    ReferenceIdentity::new(namespace, kind, id).map_err(|e| e.to_string())
}
