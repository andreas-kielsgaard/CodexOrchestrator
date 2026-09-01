use super::{
    address_references::{
        WorkflowConnectionReference, WorkflowInstanceReference, WorkflowNodeReference,
        WorkflowRecipeReference,
    },
    compiled_plan::{
        WorkflowCompilationInput, WorkflowCompiledConnection, WorkflowCompiledNode,
        WorkflowConnectionPromptInput, WorkflowConnectionTargetPlan, WorkflowConnectionTrigger,
    },
};
use crate::{
    execution_configuration::{CapabilityProfile, NodeProfile, SessionCreationRequest},
    session_events::ReferenceIdentity,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const WORKFLOW_RECIPE_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkflowRecipeDraft {
    pub(crate) contract_version: u32,
    pub(crate) recipe_id: String,
    pub(crate) name: String,
    pub(crate) revision: u64,
    pub(crate) starting_node_id: Option<String>,
    pub(crate) nodes: Vec<WorkflowAuthoringNode>,
    pub(crate) connections: Vec<WorkflowAuthoringConnection>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkflowAuthoringNode {
    pub(crate) node_id: String,
    pub(crate) name: String,
    pub(crate) position_x: f64,
    pub(crate) position_y: f64,
    pub(crate) capability_profile_id: String,
    pub(crate) node_profile: NodeProfile,
    pub(crate) initial_prompt: Option<String>,
    pub(crate) agent_identity_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkflowAuthoringConnection {
    pub(crate) connection_id: String,
    pub(crate) name: String,
    pub(crate) source_node_id: String,
    pub(crate) destination_node_id: String,
    pub(crate) trigger: WorkflowConnectionTrigger,
    pub(crate) prompt_inputs: Vec<WorkflowConnectionPromptInput>,
    pub(crate) prompt_text: String,
    pub(crate) target: WorkflowConnectionTargetPlan,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowRecipeState {
    pub(crate) draft: WorkflowRecipeDraft,
    pub(crate) active: Option<WorkflowRecipeDraft>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowRecipeSummary {
    pub(crate) recipe_id: String,
    pub(crate) name: String,
    pub(crate) draft_revision: u64,
    pub(crate) active_revision: Option<u64>,
    pub(crate) updated_at: String,
}

impl WorkflowRecipeDraft {
    pub(crate) fn validate_storable(&self) -> Result<(), String> {
        if self.contract_version != WORKFLOW_RECIPE_CONTRACT_VERSION {
            return Err(format!(
                "Workflow recipe contract version {} is unsupported",
                self.contract_version
            ));
        }
        validate_identifier("Workflow recipe", "recipeId", &self.recipe_id)?;
        validate_label("Workflow recipe", "name", &self.name)?;
        if self.revision == 0 {
            return Err("Workflow recipe revision must be positive".into());
        }
        for node in &self.nodes {
            node.validate()?;
        }
        for connection in &self.connections {
            connection.validate()?;
        }
        Ok(())
    }

    pub(crate) fn validate_activatable(&self) -> Result<(), String> {
        self.validate_storable()?;
        let starting_node = self
            .starting_node_id
            .as_deref()
            .ok_or_else(|| "Active Workflow recipe requires a starting node".to_string())?;
        if self.nodes.is_empty() {
            return Err("Active Workflow recipe requires at least one node".into());
        }

        let mut node_ids = BTreeSet::new();
        for node in &self.nodes {
            if !node_ids.insert(node.node_id.as_str()) {
                return Err(format!(
                    "Workflow recipe contains duplicate node `{}`",
                    node.node_id
                ));
            }
        }
        if !node_ids.contains(starting_node) {
            return Err(format!(
                "Workflow starting node `{starting_node}` is absent from the recipe"
            ));
        }

        let mut connection_ids = BTreeSet::new();
        for connection in &self.connections {
            if !connection_ids.insert(connection.connection_id.as_str()) {
                return Err(format!(
                    "Workflow recipe contains duplicate connection `{}`",
                    connection.connection_id
                ));
            }
            if !node_ids.contains(connection.source_node_id.as_str()) {
                return Err(format!(
                    "Workflow connection `{}` references absent source node `{}`",
                    connection.connection_id, connection.source_node_id
                ));
            }
            if !node_ids.contains(connection.destination_node_id.as_str()) {
                return Err(format!(
                    "Workflow connection `{}` references absent destination node `{}`",
                    connection.connection_id, connection.destination_node_id
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn compilation_input(
        &self,
        instance_id: &str,
        capability_profiles: &BTreeMap<String, CapabilityProfile>,
    ) -> Result<WorkflowCompilationInput, String> {
        self.validate_activatable()?;
        validate_identifier("Workflow instance", "instanceId", instance_id)?;

        let nodes = self
            .nodes
            .iter()
            .map(|node| {
                let capability_profile = capability_profiles
                    .get(&node.capability_profile_id)
                    .ok_or_else(|| {
                        format!(
                            "Workflow node `{}` references unavailable Capability Profile `{}`",
                            node.node_id, node.capability_profile_id
                        )
                    })?;
                validate_node_capabilities(node, capability_profile)?;
                Ok(WorkflowCompiledNode {
                    reference: WorkflowNodeReference::new(node.node_id.clone())
                        .map_err(|error| error.to_string())?,
                    initial_prompt: node.initial_prompt.clone(),
                    assigned_identity: node
                        .agent_identity_id
                        .as_ref()
                        .map(|identity_id| {
                            ReferenceIdentity::new(
                                "orchestrator.identities",
                                "identity",
                                identity_id.clone(),
                            )
                            .map_err(|error| error.to_string())
                        })
                        .transpose()?,
                    session_creation: SessionCreationRequest {
                        contract_version: crate::execution_configuration::SESSION_CREATION_REQUEST_CONTRACT_VERSION,
                        capability_profile: capability_profile.clone(),
                        node_profile: node.node_profile.clone(),
                    },
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        let connections = self
            .connections
            .iter()
            .map(|connection| {
                Ok(WorkflowCompiledConnection {
                    reference: WorkflowConnectionReference::new(connection.connection_id.clone())
                        .map_err(|error| error.to_string())?,
                    source_node: WorkflowNodeReference::new(connection.source_node_id.clone())
                        .map_err(|error| error.to_string())?,
                    destination_node: WorkflowNodeReference::new(
                        connection.destination_node_id.clone(),
                    )
                    .map_err(|error| error.to_string())?,
                    trigger: connection.trigger.clone(),
                    prompt_inputs: connection.prompt_inputs.clone(),
                    prompt_text: connection.prompt_text.clone(),
                    target: connection.target.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(WorkflowCompilationInput {
            instance: WorkflowInstanceReference::new(instance_id.to_string())
                .map_err(|error| error.to_string())?,
            recipe: WorkflowRecipeReference::new(self.recipe_id.clone())
                .map_err(|error| error.to_string())?,
            starting_node: WorkflowNodeReference::new(
                self.starting_node_id
                    .clone()
                    .expect("validated starting node"),
            )
            .map_err(|error| error.to_string())?,
            nodes,
            connections,
        })
    }
}

impl WorkflowAuthoringNode {
    fn validate(&self) -> Result<(), String> {
        validate_identifier("Workflow node", "nodeId", &self.node_id)?;
        validate_label("Workflow node", "name", &self.name)?;
        if !self.position_x.is_finite() || !self.position_y.is_finite() {
            return Err(format!(
                "Workflow node `{}` position must be finite",
                self.node_id
            ));
        }
        validate_identifier(
            "Workflow node",
            "capabilityProfileId",
            &self.capability_profile_id,
        )?;
        self.node_profile.validate()?;
        if self
            .initial_prompt
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(format!(
                "Workflow node `{}` initial prompt must contain text when provided",
                self.node_id
            ));
        }
        if let Some(identity_id) = &self.agent_identity_id {
            validate_identifier("Workflow node", "agentIdentityId", identity_id)?;
        }
        Ok(())
    }
}

impl WorkflowAuthoringConnection {
    fn validate(&self) -> Result<(), String> {
        validate_identifier("Workflow connection", "connectionId", &self.connection_id)?;
        validate_label("Workflow connection", "name", &self.name)?;
        validate_identifier("Workflow connection", "sourceNodeId", &self.source_node_id)?;
        validate_identifier(
            "Workflow connection",
            "destinationNodeId",
            &self.destination_node_id,
        )?;
        if self.prompt_inputs.is_empty() && self.prompt_text.trim().is_empty() {
            return Err(format!(
                "Workflow connection `{}` requires at least one prompt source",
                self.connection_id
            ));
        }
        Ok(())
    }
}

fn validate_node_capabilities(
    node: &WorkflowAuthoringNode,
    capability_profile: &CapabilityProfile,
) -> Result<(), String> {
    capability_profile.validate()?;
    if capability_profile.capability_profile_id != node.capability_profile_id {
        return Err(format!(
            "Workflow node `{}` loaded a Capability Profile with a different identity",
            node.node_id
        ));
    }
    if let Some(capability) = node
        .node_profile
        .allowed_capabilities
        .first_capability_outside(&capability_profile.allowed_capabilities)
    {
        return Err(format!(
            "Workflow node `{}` exposes {capability} outside Capability Profile `{}`",
            node.node_id, node.capability_profile_id
        ));
    }
    let defaults = &node.node_profile.pinned_defaults;
    let allowed = &node.node_profile.allowed_capabilities;
    if let Some(model) = &defaults.model {
        if !allowed.models.contains(model) {
            return Err(format!(
                "Workflow node `{}` pins unavailable model `{model}`",
                node.node_id
            ));
        }
    }
    if let Some(reasoning) = &defaults.reasoning_mode {
        if !allowed.reasoning_modes.contains(reasoning) {
            return Err(format!(
                "Workflow node `{}` pins unavailable reasoning mode `{reasoning}`",
                node.node_id
            ));
        }
    }
    if let Some(sandbox) = defaults.sandbox_mode {
        if !allowed.sandbox_modes.contains(&sandbox) {
            return Err(format!(
                "Workflow node `{}` pins unavailable sandbox mode `{sandbox:?}`",
                node.node_id
            ));
        }
    }
    Ok(())
}

fn validate_identifier(owner: &str, field: &str, value: &str) -> Result<(), String> {
    if value.is_empty() || value.trim() != value {
        Err(format!("{owner} {field} must be a non-empty trimmed value"))
    } else {
        Ok(())
    }
}

fn validate_label(owner: &str, field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{owner} {field} must contain text"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_configuration::{CapabilitySet, RuntimeSelections, SandboxMode};

    fn capabilities() -> CapabilitySet {
        CapabilitySet {
            models: ["codex-a".into()].into_iter().collect(),
            reasoning_modes: ["high".into()].into_iter().collect(),
            sandbox_modes: [SandboxMode::WorkspaceWrite].into_iter().collect(),
            ..CapabilitySet::default()
        }
    }

    fn profile() -> CapabilityProfile {
        CapabilityProfile {
            contract_version: crate::execution_configuration::CAPABILITY_PROFILE_CONTRACT_VERSION,
            capability_profile_id: "capability-default".into(),
            name: "Default".into(),
            revision: 1,
            allowed_capabilities: capabilities(),
        }
    }

    fn draft() -> WorkflowRecipeDraft {
        WorkflowRecipeDraft {
            contract_version: WORKFLOW_RECIPE_CONTRACT_VERSION,
            recipe_id: "recipe-review".into(),
            name: "Review".into(),
            revision: 1,
            starting_node_id: Some("planner".into()),
            nodes: vec![WorkflowAuthoringNode {
                node_id: "planner".into(),
                name: "Planner".into(),
                position_x: 10.0,
                position_y: 20.0,
                capability_profile_id: "capability-default".into(),
                node_profile: NodeProfile {
                    contract_version: crate::execution_configuration::NODE_PROFILE_CONTRACT_VERSION,
                    allowed_capabilities: capabilities(),
                    pinned_defaults: RuntimeSelections {
                        model: Some("codex-a".into()),
                        reasoning_mode: Some("high".into()),
                        sandbox_mode: Some(SandboxMode::WorkspaceWrite),
                    },
                },
                initial_prompt: Some("Plan the work.".into()),
                agent_identity_id: Some("identity-avery".into()),
            }],
            connections: Vec::new(),
        }
    }

    #[test]
    fn complete_draft_compiles_embedded_node_and_capability_profile() {
        let draft = draft();
        let profiles = [("capability-default".into(), profile())]
            .into_iter()
            .collect();

        let input = draft.compilation_input("instance-1", &profiles).unwrap();

        assert_eq!(input.nodes.len(), 1);
        assert_eq!(
            input.nodes[0]
                .session_creation
                .capability_profile
                .capability_profile_id,
            "capability-default"
        );
        assert_eq!(
            input.nodes[0].initial_prompt.as_deref(),
            Some("Plan the work.")
        );
        assert_eq!(
            input.nodes[0]
                .assigned_identity
                .as_ref()
                .map(ReferenceIdentity::id),
            Some("identity-avery")
        );
    }

    #[test]
    fn draft_can_be_saved_incomplete_but_not_activated() {
        let mut draft = draft();
        draft.starting_node_id = None;

        assert!(draft.validate_storable().is_ok());
        assert!(draft.validate_activatable().is_err());
    }

    #[test]
    fn activation_rejects_node_that_widens_capability_profile() {
        let mut draft = draft();
        draft.nodes[0]
            .node_profile
            .allowed_capabilities
            .models
            .insert("codex-b".into());
        let profiles = [("capability-default".into(), profile())]
            .into_iter()
            .collect();

        assert!(draft.compilation_input("instance-1", &profiles).is_err());
    }
}
