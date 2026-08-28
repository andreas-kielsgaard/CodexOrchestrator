use super::{
    DirectUserInvocationOptions, PromptSource, PromptSourceDefinition, ReferenceIdentity,
    SessionEventCommand, SessionEventDefinition, SessionEventDomainError, SessionEventSource,
    SessionEventTrigger, SessionEventTriggerBinding, SessionLogicalAddress,
};
use std::{collections::BTreeMap, error::Error, fmt};

/// Concrete data supplied when one compiled Session-event definition occurs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SessionEventOccurrence {
    pub(crate) event_group_id: ReferenceIdentity,
    pub(crate) trigger: SessionEventOccurrenceTrigger,
    pub(crate) referenced_content: BTreeMap<ReferenceIdentity, String>,
    pub(crate) created_by_session: Option<ReferenceIdentity>,
    pub(crate) direct_user_options: Option<DirectUserInvocationOptions>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SessionEventOccurrenceTrigger {
    UserRequest {
        request: ReferenceIdentity,
        text: String,
    },
    InvocationCompleted {
        session: ReferenceIdentity,
        session_address: Option<SessionLogicalAddress>,
        invocation: ReferenceIdentity,
        output: String,
    },
    McpCall {
        call: ReferenceIdentity,
        server: ReferenceIdentity,
        tool: ReferenceIdentity,
        arguments: BTreeMap<String, String>,
    },
    ApplicationEvent {
        event: ReferenceIdentity,
        event_kind: ReferenceIdentity,
        fields: BTreeMap<String, String>,
    },
    EventGroupCompleted {
        event_group: ReferenceIdentity,
        source_definition: ReferenceIdentity,
    },
}

pub(crate) struct SessionEventMaterializer;

impl SessionEventMaterializer {
    pub(crate) fn materialize(
        definition: &SessionEventDefinition,
        occurrence: SessionEventOccurrence,
    ) -> Result<SessionEventCommand, SessionEventMaterializationError> {
        definition
            .validate()
            .map_err(SessionEventMaterializationError::Domain)?;
        validate_trigger_binding(&definition.trigger, &occurrence.trigger)?;

        let prompt_sources = materialize_sources(
            &definition.prompt_sources,
            &occurrence.trigger,
            &occurrence.referenced_content,
        )?;
        let created_session_prompt_sources = materialize_sources(
            &definition.created_session_prompt_sources,
            &occurrence.trigger,
            &occurrence.referenced_content,
        )?;
        let (trigger, source) = runtime_trigger_and_source(&occurrence.trigger);
        let command = SessionEventCommand {
            event_group_id: occurrence.event_group_id,
            definition_ref: definition.definition_ref.clone(),
            trigger,
            source,
            prompt_sources,
            created_session_prompt_sources,
            creation_configuration: definition.creation_configuration.clone(),
            target: definition.target.clone(),
            created_by_session: occurrence.created_by_session,
            direct_user_options: occurrence.direct_user_options,
        };
        command
            .validate()
            .map_err(SessionEventMaterializationError::Domain)?;
        Ok(command)
    }
}

fn validate_trigger_binding(
    binding: &SessionEventTriggerBinding,
    occurrence: &SessionEventOccurrenceTrigger,
) -> Result<(), SessionEventMaterializationError> {
    let matches = match (binding, occurrence) {
        (
            SessionEventTriggerBinding::UserRequest,
            SessionEventOccurrenceTrigger::UserRequest { .. },
        ) => true,
        (
            SessionEventTriggerBinding::InvocationCompleted { source_address },
            SessionEventOccurrenceTrigger::InvocationCompleted {
                session_address, ..
            },
        ) => source_address
            .as_ref()
            .is_none_or(|expected| session_address.as_ref() == Some(expected)),
        (
            SessionEventTriggerBinding::McpCall { server, tool },
            SessionEventOccurrenceTrigger::McpCall {
                server: actual_server,
                tool: actual_tool,
                ..
            },
        ) => server == actual_server && tool == actual_tool,
        (
            SessionEventTriggerBinding::ApplicationEvent { event_kind },
            SessionEventOccurrenceTrigger::ApplicationEvent {
                event_kind: actual_kind,
                ..
            },
        ) => event_kind == actual_kind,
        (
            SessionEventTriggerBinding::EventGroupCompleted { source_definition },
            SessionEventOccurrenceTrigger::EventGroupCompleted {
                source_definition: actual_definition,
                ..
            },
        ) => source_definition == actual_definition,
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        Err(SessionEventMaterializationError::TriggerMismatch)
    }
}

fn materialize_sources(
    definitions: &[PromptSourceDefinition],
    occurrence: &SessionEventOccurrenceTrigger,
    referenced_content: &BTreeMap<ReferenceIdentity, String>,
) -> Result<Vec<PromptSource>, SessionEventMaterializationError> {
    definitions
        .iter()
        .map(|definition| match definition {
            PromptSourceDefinition::Literal { text } => {
                Ok(PromptSource::Literal { text: text.clone() })
            }
            PromptSourceDefinition::UserRequestText => {
                let SessionEventOccurrenceTrigger::UserRequest { request, text } = occurrence
                else {
                    return Err(SessionEventMaterializationError::MissingPromptValue(
                        "user request text".into(),
                    ));
                };
                Ok(PromptSource::UserRequestText {
                    request: request.clone(),
                    text: text.clone(),
                })
            }
            PromptSourceDefinition::InvocationOutput => {
                let SessionEventOccurrenceTrigger::InvocationCompleted {
                    invocation, output, ..
                } = occurrence
                else {
                    return Err(SessionEventMaterializationError::MissingPromptValue(
                        "invocation output".into(),
                    ));
                };
                Ok(PromptSource::InvocationOutput {
                    invocation: invocation.clone(),
                    text: output.clone(),
                })
            }
            PromptSourceDefinition::McpArgument { name } => {
                let SessionEventOccurrenceTrigger::McpCall {
                    call, arguments, ..
                } = occurrence
                else {
                    return Err(SessionEventMaterializationError::MissingPromptValue(
                        format!("MCP argument `{name}`"),
                    ));
                };
                let text = arguments.get(name).cloned().ok_or_else(|| {
                    SessionEventMaterializationError::MissingPromptValue(format!(
                        "MCP argument `{name}`"
                    ))
                })?;
                Ok(PromptSource::McpArgument {
                    call: call.clone(),
                    name: name.clone(),
                    text,
                })
            }
            PromptSourceDefinition::ApplicationEventField { field } => {
                let SessionEventOccurrenceTrigger::ApplicationEvent { event, fields, .. } =
                    occurrence
                else {
                    return Err(SessionEventMaterializationError::MissingPromptValue(
                        format!("application event field `{field}`"),
                    ));
                };
                let text = fields.get(field).cloned().ok_or_else(|| {
                    SessionEventMaterializationError::MissingPromptValue(format!(
                        "application event field `{field}`"
                    ))
                })?;
                Ok(PromptSource::ApplicationEventField {
                    event: event.clone(),
                    field: field.clone(),
                    text,
                })
            }
            PromptSourceDefinition::ReferencedContent { reference } => {
                let text = referenced_content.get(reference).cloned().ok_or_else(|| {
                    SessionEventMaterializationError::MissingPromptValue(format!(
                        "referenced content `{reference}`"
                    ))
                })?;
                Ok(PromptSource::ReferencedContent {
                    reference: reference.clone(),
                    text,
                })
            }
        })
        .collect()
}

fn runtime_trigger_and_source(
    occurrence: &SessionEventOccurrenceTrigger,
) -> (SessionEventTrigger, SessionEventSource) {
    match occurrence {
        SessionEventOccurrenceTrigger::UserRequest { request, .. } => (
            SessionEventTrigger::UserRequest {
                request: request.clone(),
            },
            SessionEventSource::UserRequest {
                request: request.clone(),
            },
        ),
        SessionEventOccurrenceTrigger::InvocationCompleted {
            session,
            invocation,
            ..
        } => (
            SessionEventTrigger::InvocationCompleted {
                session: session.clone(),
                invocation: invocation.clone(),
            },
            SessionEventSource::SessionInvocation {
                session: session.clone(),
                invocation: invocation.clone(),
            },
        ),
        SessionEventOccurrenceTrigger::McpCall {
            call, server, tool, ..
        } => (
            SessionEventTrigger::McpCall {
                call: call.clone(),
                server: server.clone(),
                tool: tool.clone(),
            },
            SessionEventSource::McpCall {
                call: call.clone(),
                server: server.clone(),
                tool: tool.clone(),
            },
        ),
        SessionEventOccurrenceTrigger::ApplicationEvent { event, .. } => (
            SessionEventTrigger::ApplicationEvent {
                event: event.clone(),
            },
            SessionEventSource::ApplicationEvent {
                event: event.clone(),
            },
        ),
        SessionEventOccurrenceTrigger::EventGroupCompleted { event_group, .. } => (
            SessionEventTrigger::EventGroupCompleted {
                event_group: event_group.clone(),
            },
            SessionEventSource::EventGroup {
                event_group: event_group.clone(),
            },
        ),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SessionEventMaterializationError {
    Domain(SessionEventDomainError),
    TriggerMismatch,
    MissingPromptValue(String),
}

impl fmt::Display for SessionEventMaterializationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::TriggerMismatch => {
                formatter.write_str("Session-event occurrence does not match its trigger binding")
            }
            Self::MissingPromptValue(value) => {
                write!(
                    formatter,
                    "Session-event occurrence did not provide {value}"
                )
            }
        }
    }
}

impl Error for SessionEventMaterializationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_events::{
        MissingTargetPolicy, RunningFilter, SessionTarget, TargetCardinality, TargetOrdering,
        TargetSelection,
    };

    #[test]
    fn materializes_ordered_mcp_and_referenced_prompt_sources() {
        let server = reference("mcp_server", "orchestrator");
        let tool = reference("mcp_tool", "continue_workflow");
        let content = reference("workflow_file", "plan.md");
        let definition = definition(
            SessionEventTriggerBinding::McpCall {
                server: server.clone(),
                tool: tool.clone(),
            },
            vec![
                PromptSourceDefinition::McpArgument {
                    name: "instruction".into(),
                },
                PromptSourceDefinition::ReferencedContent {
                    reference: content.clone(),
                },
                PromptSourceDefinition::Literal {
                    text: "Review the plan.".into(),
                },
            ],
        );
        let mut arguments = BTreeMap::new();
        arguments.insert("instruction".into(), "Be strict.".into());
        let mut referenced_content = BTreeMap::new();
        referenced_content.insert(content.clone(), "Plan contents".into());

        let command = SessionEventMaterializer::materialize(
            &definition,
            SessionEventOccurrence {
                event_group_id: reference("event_group", "group-1"),
                trigger: SessionEventOccurrenceTrigger::McpCall {
                    call: reference("mcp_call", "call-1"),
                    server,
                    tool,
                    arguments,
                },
                referenced_content,
                created_by_session: Some(reference("session", "parent")),
                direct_user_options: None,
            },
        )
        .unwrap();

        assert_eq!(
            command
                .prompt_sources
                .iter()
                .map(PromptSource::text)
                .collect::<Vec<_>>(),
            vec!["Be strict.", "Plan contents", "Review the plan."]
        );
        assert_eq!(
            command.created_by_session,
            Some(reference("session", "parent"))
        );
        assert!(matches!(command.source, SessionEventSource::McpCall { .. }));
    }

    #[test]
    fn rejects_an_occurrence_that_does_not_match_the_compiled_binding() {
        let definition = definition(
            SessionEventTriggerBinding::UserRequest,
            vec![PromptSourceDefinition::UserRequestText],
        );

        let result = SessionEventMaterializer::materialize(
            &definition,
            SessionEventOccurrence {
                event_group_id: reference("event_group", "group-2"),
                trigger: SessionEventOccurrenceTrigger::ApplicationEvent {
                    event: reference("application_event", "event-1"),
                    event_kind: reference("application_event_kind", "work_completed"),
                    fields: BTreeMap::new(),
                },
                referenced_content: BTreeMap::new(),
                created_by_session: None,
                direct_user_options: None,
            },
        );

        assert_eq!(
            result,
            Err(SessionEventMaterializationError::TriggerMismatch)
        );
    }

    fn definition(
        trigger: SessionEventTriggerBinding,
        prompt_sources: Vec<PromptSourceDefinition>,
    ) -> SessionEventDefinition {
        SessionEventDefinition {
            definition_ref: reference("event_definition", "definition-1"),
            trigger,
            target: TargetSelection {
                target: SessionTarget::Exact {
                    session: reference("session", "session-1"),
                },
                cardinality: TargetCardinality::First,
                ordering: TargetOrdering::Newest,
                running: RunningFilter::Any,
                created_by: None,
                missing: MissingTargetPolicy::Fail,
            },
            prompt_sources,
            created_session_prompt_sources: Vec::new(),
            creation_configuration: None,
        }
    }

    fn reference(kind: &str, id: &str) -> ReferenceIdentity {
        ReferenceIdentity::new("test", kind, id).unwrap()
    }
}
