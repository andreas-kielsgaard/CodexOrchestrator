use super::*;

impl WorkflowExecutionService {
    pub fn route_otp_output(
        &self,
        instance: &RecipeInstance,
        context: &InvocationContext,
        output_id: &str,
        payload: Value,
    ) -> Result<RoutingReceipt, String> {
        let producer = self.registry.tool(&context.capability)?;
        producer
            .outputs
            .iter()
            .find(|o| o.id == output_id && o.kind == OutputKind::Data)
            .ok_or("OTP emitted an undeclared data output")?;
        self.registry
            .validate_output(&context.capability, output_id, &payload)?;
        let source = context
            .source
            .as_ref()
            .ok_or("A Workflow trigger requires a source node")?;
        let trigger = OutputRef {
            capability: context.capability.clone(),
            output: output_id.into(),
        };
        let plan = self.compile_graph(&instance.id)?;
        let mut deliveries = 0;
        let mut stops = 0;
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
                let action_tool = self.registry.tool(&connection.action)?;
                let inputs = if matches!(
                    action_tool.entrypoint,
                    Entrypoint::Action { uses_prompt: true }
                ) {
                    crate::workflows::prompt_content::resolve_inputs(
                        instance,
                        connection,
                        &payload,
                        self.sessions.as_ref(),
                    )
                } else {
                    Ok(vec![])
                };
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
                    stops += results
                        .stop_outcomes
                        .iter()
                        .filter(|s| s.status == "requested")
                        .count();
                    for result in results.event_groups {
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
            Ok(RoutingReceipt { deliveries, stops })
        } else {
            Err(failures.join("; "))
        }
    }
}
