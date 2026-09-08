use super::address_references::{
    WorkflowConnectionReference, WorkflowInstanceReference, WorkflowNodeReference,
    WorkflowRecipeReference,
};
use crate::{
    execution_configuration::{SessionCreationIntent, SessionCreationRequest},
    otp_api::{CapabilityRef, OutputRef},
    session_events::ReferenceIdentity,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowCompiledPlan {
    pub instance: WorkflowInstanceReference,
    pub recipe: WorkflowRecipeReference,
    pub starting_node: WorkflowNodeReference,
    pub entry_action: CapabilityRef,
    pub nodes: Vec<WorkflowCompiledNode>,
    pub connections: Vec<WorkflowCompiledConnection>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkflowCompiledNode {
    pub reference: WorkflowNodeReference,
    pub initial_prompt: Option<String>,
    pub assigned_identity: Option<ReferenceIdentity>,
    pub session_creation: WorkflowSessionCreation,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) enum WorkflowSessionCreation {
    ResolvedInput(SessionCreationRequest),
    AtBirth(SessionCreationIntent),
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkflowCompiledConnection {
    pub reference: WorkflowConnectionReference,
    pub source_node: WorkflowNodeReference,
    pub destination_node: WorkflowNodeReference,
    pub trigger: OutputRef,
    pub action: CapabilityRef,
    pub configuration: Value,
    pub prompt_inputs: Vec<WorkflowConnectionPromptInput>,
    pub prompt_text: String,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum WorkflowConnectionPromptInput {
    OutputField {
        field: String,
    },
    NodeFiles {
        node_id: String,
        association: FileAssociation,
    },
    FileContent {
        path: String,
    },
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FileAssociation {
    Created,
    Edited,
    Either,
}
