use super::*;

impl WorkflowExecutionService {
    pub fn dispatch_otp_action(
        &self,
        instance: &RecipeInstance,
        context: &InvocationContext,
        output: Option<OutputRef>,
        payload: Value,
        configuration: Value,
        inputs: Result<Vec<ResolvedInput>, String>,
    ) -> Result<WorkflowActionResult, String> {
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
            completed_at: None,
            emitted_outputs: vec![],
            id,
            instance_id: instance.id.clone(),
            definition_ref,
            context: context.clone(),
            output,
            payload,
            created_at: chrono::Utc::now().to_rfc3339(),
            session_requests: vec![],
            stop_outcomes: vec![],
            message: String::new(),
            event_groups: vec![],
            error: None,
        };
        if !self.instances.begin_attempt(&attempt)? {
            return Ok(WorkflowActionResult {
                attempt_id: attempt.id.clone(),
                message: "Already processed".into(),
                ..WorkflowActionResult::default()
            });
        }
        let mut results = Vec::new();
        let run = (|| -> Result<(), String> {
            let host = WorkflowHost {
                execution: self,
                instance,
                context,
            };
            let descriptor = self.registry.tool(&context.capability)?;
            let uses_prompt = matches!(
                descriptor.entrypoint,
                Entrypoint::Action { uses_prompt: true }
            );
            let result = self.registry.invoke(
                context,
                ToolInput::Action {
                    payload: attempt.payload.clone(),
                    configuration,
                    inputs: if uses_prompt { inputs? } else { vec![] },
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
            if !result.stop_requests.is_empty()
                && !descriptor
                    .outputs
                    .iter()
                    .any(|o| o.kind == OutputKind::SessionStopRequest)
            {
                return Err("Tool did not declare a Session Stop Request output".into());
            }
            let emissions = result.emitted_outputs;
            attempt.emitted_outputs = emissions.clone();
            attempt.message = result.text;
            attempt.session_requests = result.session_requests;
            // Store the selected requests before any external invocation is launched.
            self.instances.update_attempt(&attempt)?;
            for (ordinal, request) in attempt.session_requests.iter().enumerate() {
                let dispatched =
                    self.execute_session_request(instance, context, &attempt, ordinal, request)?;
                attempt.event_groups.push(dispatched.clone());
                self.instances.update_attempt(&attempt)?;
                self.capability_runtimes.delivered(
                    context,
                    &attempt.id,
                    ordinal,
                    request,
                    &dispatched,
                )?;
                results.push(dispatched);
            }
            for request in result.stop_requests {
                self.execute_stop_request(instance, context, &mut attempt, &request)?;
            }
            if !emissions.is_empty() {
                let node_id = context
                    .output_node_id
                    .as_deref()
                    .ok_or("Action output requires a destination node")?;
                let node = instance
                    .recipe
                    .nodes
                    .iter()
                    .find(|n| n.node_id == node_id)
                    .ok_or("Action node is missing")?;
                let output_context = InvocationContext {
                    source: Some(SourceContext {
                        node_id: node_id.into(),
                        node_name: node.name.clone(),
                        session_id: None,
                        invocation_id: None,
                    }),
                    output_node_id: None,
                    connection_id: None,
                    occurrence_id: attempt.id.clone(),
                    ..context.clone()
                };
                for emission in emissions {
                    self.route_otp_output(
                        instance,
                        &output_context,
                        &emission.output,
                        emission.payload,
                    )?;
                }
            }
            Ok(())
        })();
        attempt.error = run.as_ref().err().cloned();
        attempt.completed_at = Some(chrono::Utc::now().to_rfc3339());
        self.instances.update_attempt(&attempt)?;
        if let Some(observer) = &self.record_observer {
            observer(&instance.id)
        }
        run?;
        Ok(WorkflowActionResult {
            attempt_id: attempt.id,
            event_groups: results,
            stop_outcomes: attempt.stop_outcomes,
            message: attempt.message,
        })
    }
}
