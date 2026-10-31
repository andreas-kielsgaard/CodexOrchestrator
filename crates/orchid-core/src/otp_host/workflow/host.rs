use super::*;

pub struct WorkflowHost<'a> {
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
    fn invoke_service(
        &self,
        context: &InvocationContext,
        payload: Value,
    ) -> Result<ToolResult, String> {
        self.check(context)?;
        let configuration = if let Some(node_id) = &context.output_node_id {
            let node = self
                .instance
                .recipe
                .nodes
                .iter()
                .find(|node| &node.node_id == node_id)
                .ok_or("Capability node is missing")?;
            match &node.binding {
                crate::workflows::authoring::WorkflowNodeBinding::Capability(binding) => {
                    let descriptor = self
                        .execution
                        .registry
                        .node_capability(&binding.capability)?;
                    if !descriptor.actions.contains(&context.capability) {
                        return Err("Action is not offered by this capability node".into());
                    }
                    binding.configuration.clone()
                }
                crate::workflows::authoring::WorkflowNodeBinding::Agent(_) => serde_json::json!({}),
            }
        } else {
            serde_json::json!({})
        };
        self.execution
            .capability_runtimes
            .invoke(context, &configuration, payload)
    }

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
            initial_prompt: node.agent()?.initial_prompt.clone(),
            configuration: serde_json::to_value(node.agent()?).map_err(|e| e.to_string())?,
        })
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
    pub fn invoke_mcp(
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

    pub fn otp_source(
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
            session_id: Some(session_id.into()),
            invocation_id: Some(invocation_id.into()),
        };
        Ok((instance, source))
    }
}
