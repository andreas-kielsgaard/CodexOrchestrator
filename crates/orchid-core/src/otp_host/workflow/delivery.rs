use super::*;

impl WorkflowExecutionService {
    pub(super) fn execute_stop_request(
        &self,
        instance: &RecipeInstance,
        context: &InvocationContext,
        attempt: &mut WorkflowEventAttempt,
        request: &SessionStopRequest,
    ) -> Result<(), String> {
        let index = attempt.stop_outcomes.len();
        attempt.stop_outcomes.push(SessionStopOutcome {
            node_id: request.node_id.clone(),
            session_id: request.session_id.clone(),
            invocation_id: None,
            status: "pending".into(),
            error: None,
        });
        self.instances.update_attempt(attempt)?;
        let execute = (|| -> Result<(), String> {
            if context.output_node_id.as_deref() != Some(request.node_id.as_str()) {
                return Err("Stop request targets an unbound node".into());
            }
            let address = node_address(&instance.id, &request.node_id)?;
            let entry = self
                .directory
                .find_exact(&reference(
                    "orchestrator.agent_sessions",
                    "session",
                    &request.session_id,
                )?)
                .map_err(|e| e.to_string())?
                .ok_or("Stop Session is missing")?;
            if entry.logical_address.as_ref() != Some(&address) {
                return Err("Stop Session belongs to another Workflow node".into());
            }
            let history = self
                .sessions
                .load_session_history(
                    &crate::agent_sessions::domain::AgentSessionId::new(&request.session_id)
                        .map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?
                .ok_or("Stop Session is missing")?;
            let active = history
                .invocations
                .iter()
                .find(|i| i.invocation.status.is_active())
                .map(|i| i.invocation.id.to_string());
            attempt.stop_outcomes[index].invocation_id = active.clone();
            self.instances.update_attempt(attempt)?;
            let requested = match active {
                Some(id) => self
                    .session_control
                    .as_ref()
                    .ok_or("Session cancellation is unavailable")?
                    .cancel(&id)?,
                None => false,
            };
            attempt.stop_outcomes[index].status =
                if requested { "requested" } else { "no_op" }.into();
            Ok(())
        })();
        if let Err(error) = &execute {
            attempt.stop_outcomes[index].status = "failed".into();
            attempt.stop_outcomes[index].error = Some(error.clone());
        }
        self.instances.update_attempt(attempt)?;
        execute
    }

    pub(super) fn execute_session_request(
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
        let plan = self.compile_graph(&instance.id)?;
        let node = plan
            .nodes
            .iter()
            .find(|n| n.reference.identity().id() == request.node_id)
            .ok_or("Destination node is missing")?;
        let fresh = matches!(request.target, SessionRequestTarget::New);
        let target = match &request.target {
            SessionRequestTarget::New => SessionTarget::Logical {
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
            WorkflowSessionCreation::Capability(_) => {
                return Err("Cannot create an agent session for a capability node".into())
            }
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
            Some(source) if source.session_id.is_some() && source.invocation_id.is_some() => (
                SessionEventTrigger::ApplicationEvent {
                    event: cause.clone(),
                },
                SessionEventSource::SessionInvocation {
                    session: reference(
                        "orchestrator.agent_sessions",
                        "session",
                        source
                            .session_id
                            .as_deref()
                            .ok_or("Missing source session")?,
                    )?,
                    invocation: reference(
                        "orchestrator.agent_sessions",
                        "invocation",
                        source
                            .invocation_id
                            .as_deref()
                            .ok_or("Missing source invocation")?,
                    )?,
                },
                Some(reference(
                    "orchestrator.agent_sessions",
                    "session",
                    source
                        .session_id
                        .as_deref()
                        .ok_or("Missing source session")?,
                )?),
            ),
            Some(_) => (
                SessionEventTrigger::ApplicationEvent {
                    event: cause.clone(),
                },
                SessionEventSource::ApplicationEvent { event: cause },
                None,
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
                delivery_mode: request.delivery_mode,
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
                    missing: if fresh {
                        MissingTargetPolicy::Create
                    } else {
                        MissingTargetPolicy::Fail
                    },
                },
                created_by_session,
                direct_user_options: None,
            })
            .map_err(|e| e.to_string())
    }
}
