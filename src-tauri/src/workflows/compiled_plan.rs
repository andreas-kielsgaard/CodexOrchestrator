use super::address_references::{
    WorkflowConnectionReference, WorkflowInstanceReference, WorkflowNodeReference,
    WorkflowRecipeReference,
};
use crate::{
    execution_configuration::SessionCreationRequest,
    session_events::{
        MissingTargetPolicy, ReferenceIdentity, RunningFilter, SessionCreationFilter,
        TargetCardinality, TargetOrdering,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkflowCompilationInput {
    pub(crate) instance: WorkflowInstanceReference,
    pub(crate) recipe: WorkflowRecipeReference,
    pub(crate) starting_node: WorkflowNodeReference,
    pub(crate) nodes: Vec<WorkflowCompiledNode>,
    pub(crate) connections: Vec<WorkflowCompiledConnection>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct WorkflowCompiledNode {
    pub(crate) reference: WorkflowNodeReference,
    pub(crate) initial_prompt: Option<String>,
    /// Contains the Capability Profile and the embedded Node Profile that will be resolved only
    /// if the generic Session directory creates this node's Session.
    pub(crate) session_creation: SessionCreationRequest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkflowCompiledConnection {
    pub(crate) reference: WorkflowConnectionReference,
    pub(crate) source_node: WorkflowNodeReference,
    pub(crate) destination_node: WorkflowNodeReference,
    pub(crate) trigger: WorkflowConnectionTrigger,
    pub(crate) prompt_inputs: Vec<WorkflowConnectionPromptInput>,
    pub(crate) prompt_text: String,
    pub(crate) target: WorkflowConnectionTargetPlan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum WorkflowConnectionPromptInput {
    InvocationOutput,
    McpArgument { name: String },
    ApplicationEventField { field: String },
    ReferencedContent { reference: ReferenceIdentity },
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
