use super::address_references::{
    WorkflowConnectionReference, WorkflowInstanceReference, WorkflowNodeReference,
    WorkflowRecipeReference,
};
use crate::{
    execution_configuration::{SessionCreationIntent, SessionCreationRequest},
    session_events::{
        MissingTargetPolicy, ReferenceIdentity, RunningFilter, SessionCreationFilter,
        TargetCardinality, TargetOrdering,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkflowCompilationInput {
    pub(crate) instance: WorkflowInstanceReference,
    pub(crate) recipe: WorkflowRecipeReference,
    pub(crate) starting_node: WorkflowNodeReference,
    pub(crate) nodes: Vec<WorkflowCompiledNode>,
    pub(crate) connections: Vec<WorkflowCompiledConnection>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkflowCompiledNode {
    pub(crate) reference: WorkflowNodeReference,
    pub(crate) initial_prompt: Option<String>,
    pub(crate) assigned_identity: Option<ReferenceIdentity>,
    /// Contains the Capability Profile and the embedded Node Profile that will be resolved only
    /// if the generic Session directory creates this node's Session.
    pub(crate) session_creation: WorkflowSessionCreation,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum WorkflowSessionCreation {
    ResolvedInput(SessionCreationRequest),
    AtBirth(SessionCreationIntent),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkflowCompiledConnection {
    pub(crate) reference: WorkflowConnectionReference,
    pub(crate) source_node: WorkflowNodeReference,
    pub(crate) destination_node: WorkflowNodeReference,
    pub(crate) trigger: WorkflowConnectionTrigger,
    pub(crate) prompt_inputs: Vec<WorkflowConnectionPromptInput>,
    pub(crate) prompt_text: String,
    pub(crate) target: WorkflowConnectionTargetPlan,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum WorkflowConnectionTrigger {
    InvocationCompleted,
    McpCall {
        server: ReferenceIdentity,
        tool: ReferenceIdentity,
    },
    ApplicationEvent {
        event_kind: ReferenceIdentity,
    },
    EventGroupCompleted {
        source_definition: ReferenceIdentity,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum WorkflowConnectionPromptInput {
    TriggerField {
        field: String,
    },
    NodeFiles {
        node_id: String,
        association: FileAssociation,
    },
    InvocationOutput,
    McpArgument {
        name: String,
    },
    ApplicationEventField {
        field: String,
    },
    ReferencedContent {
        reference: ReferenceIdentity,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FileAssociation {
    Created,
    Edited,
    Either,
}

pub(crate) fn input_reference(input: &WorkflowConnectionPromptInput) -> ReferenceIdentity {
    ReferenceIdentity::new(
        "workflow",
        "connection_input",
        serde_json::to_string(input).expect("serializable prompt input"),
    )
    .expect("nonempty prompt input reference")
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkflowConnectionTargetPlan {
    pub(crate) cardinality: TargetCardinality,
    pub(crate) ordering: TargetOrdering,
    pub(crate) running: RunningFilter,
    pub(crate) created_by: Option<SessionCreationFilter>,
    pub(crate) missing: MissingTargetPolicy,
}

impl Default for WorkflowConnectionTargetPlan {
    fn default() -> Self {
        Self {
            cardinality: TargetCardinality::First,
            ordering: TargetOrdering::Newest,
            running: RunningFilter::Any,
            created_by: None,
            missing: MissingTargetPolicy::Create,
        }
    }
}
