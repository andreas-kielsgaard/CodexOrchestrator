use super::{
    address_references::{workflow_node_address, WorkflowEventDefinitionReference},
    compiled_plan::{
        WorkflowCompilationInput, WorkflowCompiledConnection, WorkflowCompiledNode,
        WorkflowConnectionPromptInput, WorkflowConnectionTrigger,
    },
};
use crate::session_events::{
    MissingTargetPolicy, PromptSourceDefinition, ReferenceIdentity, SessionCreationConfiguration,
    SessionEventDefinition, SessionEventDomainError, SessionEventTriggerBinding, SessionTarget,
    TargetSelection,
};
use std::{collections::BTreeMap, error::Error, fmt};

const CREATION_CONFIGURATION_NAMESPACE: &str = "orchestrator.execution_configuration";
const CREATION_CONFIGURATION_KIND: &str = "session_creation_request";
const CREATION_CONFIGURATION_VERSION: &str = "v1";

pub(crate) struct WorkflowCompiler;

impl WorkflowCompiler {
    pub(crate) fn compile(
        input: WorkflowCompilationInput,
    ) -> Result<Vec<SessionEventDefinition>, WorkflowCompilationError> {
        let nodes = index_nodes(&input.nodes)?;
        let starting_node = nodes
            .get(input.starting_node.identity().id())
            .copied()
            .ok_or_else(|| {
                WorkflowCompilationError::InvalidAuthoring(format!(
                    "Starting Workflow node `{}` is absent from the compiled node set",
                    input.starting_node.identity().id()
                ))
            })?;

        let mut definitions = Vec::with_capacity(input.connections.len() + 1);
        definitions.push(compile_user_entry(&input, starting_node)?);
        for connection in &input.connections {
            definitions.push(compile_connection(&input, &nodes, connection)?);
        }
        Ok(definitions)
    }
}

fn compile_user_entry(
    input: &WorkflowCompilationInput,
    node: &WorkflowCompiledNode,
) -> Result<SessionEventDefinition, WorkflowCompilationError> {
    let definition = SessionEventDefinition {
        definition_ref: WorkflowEventDefinitionReference::for_node(&input.recipe, &node.reference)?
            .into_identity(),
        trigger: SessionEventTriggerBinding::UserRequest,
        target: create_target(&input.instance, &node.reference),
        prompt_sources: vec![PromptSourceDefinition::UserRequestText],
        created_session_prompt_sources: initial_prompt_sources(node),
        creation_configuration: Some(creation_configuration(node)?),
    };
    definition.validate()?;
    Ok(definition)
}

fn compile_connection(
    input: &WorkflowCompilationInput,
    nodes: &BTreeMap<&str, &WorkflowCompiledNode>,
    connection: &WorkflowCompiledConnection,
) -> Result<SessionEventDefinition, WorkflowCompilationError> {
    if !nodes.contains_key(connection.source_node.identity().id()) {
        return Err(WorkflowCompilationError::InvalidAuthoring(format!(
            "Workflow connection `{}` references absent source node `{}`",
            connection.reference.identity().id(),
            connection.source_node.identity().id()
        )));
    }
    let destination = nodes
        .get(connection.destination_node.identity().id())
        .copied()
        .ok_or_else(|| {
            WorkflowCompilationError::InvalidAuthoring(format!(
                "Workflow connection `{}` references absent destination node `{}`",
                connection.reference.identity().id(),
                connection.destination_node.identity().id()
            ))
        })?;

    let creation_configuration = (connection.target.missing == MissingTargetPolicy::Create)
        .then(|| creation_configuration(destination))
        .transpose()?;
    let definition = SessionEventDefinition {
        definition_ref: WorkflowEventDefinitionReference::for_connection(
            &input.recipe,
            &connection.reference,
        )?
        .into_identity(),
        trigger: compile_trigger(input, connection),
        target: TargetSelection {
            target: SessionTarget::Logical {
                address: workflow_node_address(&input.instance, &destination.reference),
            },
            cardinality: connection.target.cardinality,
            ordering: connection.target.ordering,
            running: connection.target.running,
            created_by: connection.target.created_by.clone(),
            missing: connection.target.missing,
        },
        prompt_sources: compile_connection_prompt(connection),
        created_session_prompt_sources: if connection.target.missing == MissingTargetPolicy::Create
        {
            initial_prompt_sources(destination)
        } else {
            Vec::new()
        },
        creation_configuration,
    };
    definition.validate()?;
    Ok(definition)
}

fn compile_trigger(
    input: &WorkflowCompilationInput,
    connection: &WorkflowCompiledConnection,
) -> SessionEventTriggerBinding {
    match &connection.trigger {
        WorkflowConnectionTrigger::InvocationCompleted => {
            SessionEventTriggerBinding::InvocationCompleted {
                source_address: Some(workflow_node_address(
                    &input.instance,
                    &connection.source_node,
                )),
            }
        }
        WorkflowConnectionTrigger::McpCall { server, tool } => {
            SessionEventTriggerBinding::McpCall {
                server: server.clone(),
                tool: tool.clone(),
            }
        }
        WorkflowConnectionTrigger::ApplicationEvent { event_kind } => {
            SessionEventTriggerBinding::ApplicationEvent {
                event_kind: event_kind.clone(),
            }
        }
        WorkflowConnectionTrigger::EventGroupCompleted { source_definition } => {
            SessionEventTriggerBinding::EventGroupCompleted {
                source_definition: source_definition.clone(),
            }
        }
    }
}

fn compile_connection_prompt(
    connection: &WorkflowCompiledConnection,
) -> Vec<PromptSourceDefinition> {
    let mut sources = connection
        .prompt_inputs
        .iter()
        .map(|input| match input {
            WorkflowConnectionPromptInput::InvocationOutput => {
                PromptSourceDefinition::InvocationOutput
            }
            WorkflowConnectionPromptInput::McpArgument { name } => {
                PromptSourceDefinition::McpArgument { name: name.clone() }
            }
            WorkflowConnectionPromptInput::ApplicationEventField { field } => {
                PromptSourceDefinition::ApplicationEventField {
                    field: field.clone(),
                }
            }
            WorkflowConnectionPromptInput::ReferencedContent { reference } => {
                PromptSourceDefinition::ReferencedContent {
                    reference: reference.clone(),
                }
            }
        })
        .collect::<Vec<_>>();
    if !connection.prompt_text.trim().is_empty() {
        sources.push(PromptSourceDefinition::Literal {
            text: connection.prompt_text.clone(),
        });
    }
    sources
}

fn initial_prompt_sources(node: &WorkflowCompiledNode) -> Vec<PromptSourceDefinition> {
    node.initial_prompt
        .as_ref()
        .map(|text| vec![PromptSourceDefinition::Literal { text: text.clone() }])
        .unwrap_or_default()
}

fn create_target(
    instance: &super::address_references::WorkflowInstanceReference,
    node: &super::address_references::WorkflowNodeReference,
) -> TargetSelection {
    TargetSelection {
        target: SessionTarget::Logical {
            address: workflow_node_address(instance, node),
        },
        cardinality: crate::session_events::TargetCardinality::First,
        ordering: crate::session_events::TargetOrdering::Newest,
        running: crate::session_events::RunningFilter::Any,
        created_by: None,
        missing: MissingTargetPolicy::Create,
    }
}

fn creation_configuration(
    node: &WorkflowCompiledNode,
) -> Result<SessionCreationConfiguration, WorkflowCompilationError> {
    Ok(SessionCreationConfiguration {
        contract: ReferenceIdentity::new(
            CREATION_CONFIGURATION_NAMESPACE,
            CREATION_CONFIGURATION_KIND,
            CREATION_CONFIGURATION_VERSION,
        )?,
        payload: serde_json::to_value(&node.session_creation).map_err(|error| {
            WorkflowCompilationError::Encoding(format!(
                "Unable to encode Workflow node Session creation request: {error}"
            ))
        })?,
        assigned_identity: node.assigned_identity.clone(),
    })
}

fn index_nodes<'a>(
    nodes: &'a [WorkflowCompiledNode],
) -> Result<BTreeMap<&'a str, &'a WorkflowCompiledNode>, WorkflowCompilationError> {
    let mut indexed = BTreeMap::new();
    for node in nodes {
        if indexed
            .insert(node.reference.identity().id(), node)
            .is_some()
        {
            return Err(WorkflowCompilationError::InvalidAuthoring(format!(
                "Workflow compilation contains duplicate node `{}`",
                node.reference.identity().id()
            )));
        }
    }
    Ok(indexed)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkflowCompilationError {
    InvalidAuthoring(String),
    InvalidDefinition(String),
    Encoding(String),
}

impl fmt::Display for WorkflowCompilationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAuthoring(message)
            | Self::InvalidDefinition(message)
            | Self::Encoding(message) => formatter.write_str(message),
        }
    }
}

impl Error for WorkflowCompilationError {}

impl From<SessionEventDomainError> for WorkflowCompilationError {
    fn from(value: SessionEventDomainError) -> Self {
        Self::InvalidDefinition(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        execution_configuration::{
            CapabilityProfile, CapabilitySet, NodeProfile, RuntimeSelections,
            SessionCreationRequest,
        },
        session_events::{
            MissingTargetPolicy, RunningFilter, SessionEventMaterializer, SessionEventOccurrence,
            SessionEventOccurrenceTrigger, SessionEventTriggerBinding, SessionTarget,
            TargetCardinality, TargetOrdering,
        },
        workflows::{
            address_references::{
                WorkflowConnectionReference, WorkflowInstanceReference, WorkflowNodeReference,
                WorkflowRecipeReference,
            },
            compiled_plan::{
                WorkflowCompilationInput, WorkflowCompiledConnection, WorkflowCompiledNode,
                WorkflowConnectionPromptInput, WorkflowConnectionTargetPlan,
                WorkflowConnectionTrigger,
            },
        },
    };

    #[test]
    fn compiles_user_entry_and_connection_into_generic_definitions() {
        let start = node("start", Some("You plan the work."));
        let review = node("review", Some("You review plans."));
        let input = WorkflowCompilationInput {
            instance: WorkflowInstanceReference::new("instance-1").unwrap(),
            recipe: WorkflowRecipeReference::new("recipe-1").unwrap(),
            starting_node: start.reference.clone(),
            nodes: vec![start.clone(), review.clone()],
            connections: vec![WorkflowCompiledConnection {
                reference: WorkflowConnectionReference::new("review-plan").unwrap(),
                source_node: start.reference.clone(),
                destination_node: review.reference.clone(),
                trigger: WorkflowConnectionTrigger::McpCall {
                    server: ReferenceIdentity::new("mcp", "server", "workflow").unwrap(),
                    tool: ReferenceIdentity::new("mcp", "tool", "prompt_agent").unwrap(),
                },
                prompt_inputs: vec![
                    WorkflowConnectionPromptInput::McpArgument {
                        name: "promptText".into(),
                    },
                    WorkflowConnectionPromptInput::ReferencedContent {
                        reference: ReferenceIdentity::new("workflow", "file", "plan.md").unwrap(),
                    },
                ],
                prompt_text: "Review this plan.".into(),
                target: WorkflowConnectionTargetPlan::default(),
            }],
        };

        let definitions = WorkflowCompiler::compile(input).unwrap();

        assert_eq!(definitions.len(), 2);
        let entry = &definitions[0];
        assert_eq!(entry.trigger, SessionEventTriggerBinding::UserRequest);
        assert_eq!(
            entry.prompt_sources,
            vec![PromptSourceDefinition::UserRequestText]
        );
        assert_eq!(
            entry.created_session_prompt_sources,
            vec![PromptSourceDefinition::Literal {
                text: "You plan the work.".into()
            }]
        );
        assert_eq!(entry.target.missing, MissingTargetPolicy::Create);
        assert_creation_payload(entry, &start.session_creation);
        let user_request =
            ReferenceIdentity::new("application", "user_request", "request-1").unwrap();
        let command = SessionEventMaterializer::materialize(
            entry,
            SessionEventOccurrence {
                event_group_id: ReferenceIdentity::new("session_events", "event_group", "entry-1")
                    .unwrap(),
                trigger: SessionEventOccurrenceTrigger::UserRequest {
                    request: user_request.clone(),
                    text: "Build a plan.".into(),
                },
                referenced_content: BTreeMap::new(),
                created_by_session: None,
                direct_user_options: None,
            },
        )
        .unwrap();
        assert_eq!(
            command.prompt_sources,
            vec![crate::session_events::PromptSource::UserRequestText {
                request: user_request,
                text: "Build a plan.".into(),
            }]
        );

        let connection = &definitions[1];
        assert!(matches!(
            connection.trigger,
            SessionEventTriggerBinding::McpCall { .. }
        ));
        assert_eq!(
            connection.prompt_sources,
            vec![
                PromptSourceDefinition::McpArgument {
                    name: "promptText".into()
                },
                PromptSourceDefinition::ReferencedContent {
                    reference: ReferenceIdentity::new("workflow", "file", "plan.md").unwrap()
                },
                PromptSourceDefinition::Literal {
                    text: "Review this plan.".into()
                },
            ]
        );
        assert_eq!(
            connection.created_session_prompt_sources,
            vec![PromptSourceDefinition::Literal {
                text: "You review plans.".into()
            }]
        );
        let SessionTarget::Logical { address } = &connection.target.target else {
            panic!("logical target");
        };
        assert_eq!(address.subject, review.reference.identity().clone());
        assert_creation_payload(connection, &review.session_creation);
    }

    #[test]
    fn compiles_invocation_trigger_and_concrete_target_plan() {
        let start = node("start", None);
        let review = node("review", None);
        let input = WorkflowCompilationInput {
            instance: WorkflowInstanceReference::new("instance-1").unwrap(),
            recipe: WorkflowRecipeReference::new("recipe-1").unwrap(),
            starting_node: start.reference.clone(),
            nodes: vec![start.clone(), review.clone()],
            connections: vec![WorkflowCompiledConnection {
                reference: WorkflowConnectionReference::new("review-plan").unwrap(),
                source_node: start.reference.clone(),
                destination_node: review.reference.clone(),
                trigger: WorkflowConnectionTrigger::InvocationCompleted,
                prompt_inputs: vec![WorkflowConnectionPromptInput::InvocationOutput],
                prompt_text: "Continue.".into(),
                target: WorkflowConnectionTargetPlan {
                    cardinality: TargetCardinality::All,
                    ordering: TargetOrdering::LastAddressed,
                    running: RunningFilter::RunningOnly,
                    created_by: None,
                    missing: MissingTargetPolicy::Fail,
                },
            }],
        };

        let definitions = WorkflowCompiler::compile(input).unwrap();
        let connection = &definitions[1];

        assert_eq!(connection.target.cardinality, TargetCardinality::All);
        assert_eq!(connection.target.ordering, TargetOrdering::LastAddressed);
        assert_eq!(connection.target.running, RunningFilter::RunningOnly);
        assert_eq!(connection.target.missing, MissingTargetPolicy::Fail);
        assert_eq!(connection.creation_configuration, None);
        let SessionEventTriggerBinding::InvocationCompleted {
            source_address: Some(source),
        } = &connection.trigger
        else {
            panic!("invocation-completed trigger");
        };
        assert_eq!(source.subject, start.reference.identity().clone());
    }

    fn node(id: &str, initial_prompt: Option<&str>) -> WorkflowCompiledNode {
        WorkflowCompiledNode {
            reference: WorkflowNodeReference::new(id).unwrap(),
            initial_prompt: initial_prompt.map(str::to_string),
            assigned_identity: None,
            session_creation: SessionCreationRequest {
                contract_version: 1,
                capability_profile: CapabilityProfile {
                    contract_version: 1,
                    capability_profile_id: format!("{id}-capabilities"),
                    name: format!("{id} capabilities"),
                    revision: 1,
                    allowed_capabilities: CapabilitySet::default(),
                },
                node_profile: NodeProfile {
                    contract_version: 1,
                    allowed_capabilities: CapabilitySet::default(),
                    pinned_defaults: RuntimeSelections::default(),
                },
            },
        }
    }

    fn assert_creation_payload(
        definition: &SessionEventDefinition,
        expected: &SessionCreationRequest,
    ) {
        let configuration = definition
            .creation_configuration
            .as_ref()
            .expect("creation configuration");
        assert_eq!(
            configuration.contract,
            ReferenceIdentity::new(
                "orchestrator.execution_configuration",
                "session_creation_request",
                "v1"
            )
            .unwrap()
        );
        assert_eq!(
            serde_json::from_value::<SessionCreationRequest>(configuration.payload.clone())
                .unwrap(),
            *expected
        );
    }
}
