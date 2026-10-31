use super::{
    address_references::{
        WorkflowConnectionReference, WorkflowInstanceReference, WorkflowNodeReference,
        WorkflowRecipeReference,
    },
    compiled_plan::{
        WorkflowCompiledConnection, WorkflowCompiledNode, WorkflowCompiledPlan,
        WorkflowConnectionPromptInput, WorkflowSessionCreation,
    },
};
use crate::otp_api::{CapabilityRef, OutputRef};
use crate::{
    execution_configuration::{CapabilityProfile, NodeProfile, SessionCreationRequest},
    session_events::ReferenceIdentity,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub use super::recipe_migrations::{decode_recipe_value, WORKFLOW_RECIPE_CONTRACT_VERSION};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowRecipeDraft {
    pub contract_version: u32,
    pub recipe_id: String,
    pub name: String,
    pub revision: u64,
    pub entry_node_ids: Vec<String>,
    pub nodes: Vec<WorkflowAuthoringNode>,
    pub connections: Vec<WorkflowAuthoringConnection>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowAuthoringNode {
    pub node_id: String,
    pub name: String,
    pub position_x: f64,
    pub position_y: f64,
    #[serde(flatten)]
    pub binding: WorkflowNodeBinding,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkflowNodeBinding {
    Agent(WorkflowAgentNode),
    Capability(WorkflowCapabilityNode),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowAgentNode {
    pub capability_profile_id: String,
    pub node_profile: NodeProfile,
    pub initial_prompt: Option<String>,
    pub agent_identity_id: Option<String>,
    #[serde(default)]
    pub notifications: crate::agent_sessions::notifications::NodeNotifications,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowCapabilityNode {
    pub capability: CapabilityRef,
    pub configuration: Value,
}

impl WorkflowAuthoringNode {
    pub fn agent(&self) -> Result<&WorkflowAgentNode, String> {
        match &self.binding {
            WorkflowNodeBinding::Agent(agent) => Ok(agent),
            WorkflowNodeBinding::Capability(_) => {
                Err(format!("Node {} is not an agent", self.node_id))
            }
        }
    }
    pub fn agent_mut(&mut self) -> Result<&mut WorkflowAgentNode, String> {
        match &mut self.binding {
            WorkflowNodeBinding::Agent(agent) => Ok(agent),
            WorkflowNodeBinding::Capability(_) => {
                Err(format!("Node {} is not an agent", self.node_id))
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorkflowAuthoringConnection {
    pub connection_id: String,
    pub name: String,
    pub source_node_id: String,
    pub destination_node_id: String,
    pub trigger: OutputRef,
    pub action: CapabilityRef,
    pub configuration: serde_json::Value,
    pub prompt_inputs: Vec<WorkflowConnectionPromptInput>,
    pub prompt_text: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRecipeState {
    pub draft: WorkflowRecipeDraft,
    pub active: Option<WorkflowRecipeDraft>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowRecipeSummary {
    pub recipe_id: String,
    pub name: String,
    pub draft_revision: u64,
    pub active_revision: Option<u64>,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowAuthoringWorkspace {
    pub summaries: Vec<WorkflowRecipeSummary>,
    pub selected: Option<WorkflowRecipeState>,
}

impl WorkflowRecipeDraft {
    pub fn validate_storable(&self) -> Result<(), String> {
        if self.contract_version != WORKFLOW_RECIPE_CONTRACT_VERSION {
            return Err(format!(
                "Workflow recipe contract version {} is unsupported",
                self.contract_version
            ));
        }
        validate_identifier("Workflow recipe", "recipeId", &self.recipe_id)?;
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

    pub fn validate_activatable(&self) -> Result<(), String> {
        self.validate_storable()?;
        if self.nodes.is_empty() {
            return Err("Active Workflow recipe requires at least one node".into());
        }
        if self.entry_node_ids.is_empty() {
            return Err("Active Workflow recipe requires at least one entry node".into());
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
        let mut entry_nodes = BTreeSet::new();
        for entry_node in &self.entry_node_ids {
            if !entry_nodes.insert(entry_node.as_str()) {
                return Err(format!("Workflow entry node `{entry_node}` is repeated"));
            }
            if !node_ids.contains(entry_node.as_str()) {
                return Err(format!(
                    "Workflow entry node `{entry_node}` is absent from the recipe"
                ));
            }
            if self.nodes.iter().any(|node| {
                &node.node_id == entry_node
                    && matches!(node.binding, WorkflowNodeBinding::Capability(_))
            }) {
                return Err("User-request entry nodes must be agents; capability nodes receive service events".into());
            }
        }

        let mut connection_ids = BTreeSet::new();
        for connection in &self.connections {
            for source in &connection.prompt_inputs {
                if let WorkflowConnectionPromptInput::NodeFiles { node_id, .. } = source {
                    if !node_ids.contains(node_id.as_str()) {
                        return Err(format!("File input references missing node {node_id}"));
                    }
                }
            }
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

    pub fn compilation_input(
        &self,
        instance_id: &str,
        capability_profiles: &BTreeMap<String, CapabilityProfile>,
    ) -> Result<WorkflowCompiledPlan, String> {
        self.build_compilation_input(instance_id, Some(capability_profiles), "")
    }

    pub fn runtime_compilation_input(
        &self,
        instance_id: &str,
        working_directory: &str,
    ) -> Result<WorkflowCompiledPlan, String> {
        self.build_compilation_input(instance_id, None, working_directory)
    }

    fn build_compilation_input(
        &self,
        instance_id: &str,
        capability_profiles: Option<&BTreeMap<String, CapabilityProfile>>,
        working_directory: &str,
    ) -> Result<WorkflowCompiledPlan, String> {
        self.validate_activatable()?;
        validate_identifier("Workflow instance", "instanceId", instance_id)?;

        let nodes = self
            .nodes
            .iter()
            .map(|node| {
                if let WorkflowNodeBinding::Capability(capability) = &node.binding {
                    return Ok(WorkflowCompiledNode {
                        reference: WorkflowNodeReference::new(node.node_id.clone())
                            .map_err(|e| e.to_string())?,
                        initial_prompt: None,
                        assigned_identity: None,
                        session_creation: WorkflowSessionCreation::Capability(capability.clone()),
                    });
                }
                let agent = node.agent()?;
                let session_creation = if let Some(capability_profiles) = capability_profiles {
                    let capability_profile = capability_profiles
                        .get(&agent.capability_profile_id)
                        .ok_or_else(|| {
                            format!(
                                "Workflow node `{}` references unavailable Capability Profile `{}`",
                                node.node_id, agent.capability_profile_id
                            )
                        })?;
                    validate_node_capabilities(node, capability_profile)?;
                    WorkflowSessionCreation::ResolvedInput(SessionCreationRequest {
                        contract_version: 1,
                        capability_profile: capability_profile.clone(),
                        node_profile: agent.node_profile.clone(),
                        agent_mcp_configuration: Default::default(),
                        session_skill_inputs: Vec::new(),
                    })
                } else {
                    WorkflowSessionCreation::AtBirth(
                        crate::execution_configuration::SessionCreationIntent {
                            capability_profile_id: agent.capability_profile_id.clone(),
                            node_profile: agent.node_profile.clone(),
                            agent_mcp_configuration: Default::default(),
                            working_directory: working_directory.into(),
                            title: node.name.clone(),
                        },
                    )
                };
                Ok(WorkflowCompiledNode {
                    reference: WorkflowNodeReference::new(node.node_id.clone())
                        .map_err(|error| error.to_string())?,
                    initial_prompt: agent.initial_prompt.clone(),
                    assigned_identity: agent
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
                    session_creation,
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
                    action: connection.action.clone(),
                    configuration: connection.configuration.clone(),
                })
            })
            .collect::<Result<Vec<_>, String>>()?;

        Ok(WorkflowCompiledPlan {
            instance: WorkflowInstanceReference::new(instance_id.to_string())
                .map_err(|error| error.to_string())?,
            recipe: WorkflowRecipeReference::new(self.recipe_id.clone())
                .map_err(|error| error.to_string())?,
            starting_node: WorkflowNodeReference::new(
                self.entry_node_ids
                    .first()
                    .cloned()
                    .expect("validated entry node"),
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
        let agent = match &self.binding {
            WorkflowNodeBinding::Capability(capability) => {
                validate_identifier(
                    "Workflow capability",
                    "package",
                    &capability.capability.package,
                )?;
                validate_identifier("Workflow capability", "tool", &capability.capability.tool)?;
                if !capability.configuration.is_object() {
                    return Err("Capability configuration must be an object".into());
                }
                return Ok(());
            }
            WorkflowNodeBinding::Agent(agent) => agent,
        };
        validate_identifier(
            "Workflow node",
            "capabilityProfileId",
            &agent.capability_profile_id,
        )?;
        agent.node_profile.validate()?;
        if agent
            .initial_prompt
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(format!(
                "Workflow node `{}` initial prompt must contain text when provided",
                self.node_id
            ));
        }
        if let Some(identity_id) = &agent.agent_identity_id {
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
        Ok(())
    }
}

fn validate_node_capabilities(
    node: &WorkflowAuthoringNode,
    capability_profile: &CapabilityProfile,
) -> Result<(), String> {
    let agent = node.agent()?;
    capability_profile.validate()?;
    if capability_profile.capability_profile_id != agent.capability_profile_id {
        return Err(format!(
            "Workflow node `{}` loaded a Capability Profile with a different identity",
            node.node_id
        ));
    }
    // Route-model allowances are design-time intent. MCP/skill narrowing is checked
    // against the selected route when a Session instance is materialized.
    if !capability_profile.route_policies.is_empty() {
        return Ok(());
    }
    if let Some(capability) = agent
        .node_profile
        .allowed_capabilities
        .first_capability_outside(&capability_profile.allowed_capabilities)
    {
        return Err(format!(
            "Workflow node `{}` exposes {capability} outside Capability Profile `{}`",
            node.node_id, agent.capability_profile_id
        ));
    }
    let defaults = &agent.node_profile.pinned_defaults;
    let allowed = &agent.node_profile.allowed_capabilities;
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
            execution: Default::default(),
            contract_version: crate::execution_configuration::CAPABILITY_PROFILE_CONTRACT_VERSION,
            defaults: Default::default(),
            route_policies: Vec::new(),
            default_route_id: None,
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
            entry_node_ids: vec!["planner".into()],
            nodes: vec![WorkflowAuthoringNode {
                node_id: "planner".into(),
                name: "Planner".into(),
                position_x: 10.0,
                position_y: 20.0,
                binding: crate::workflows::authoring::WorkflowNodeBinding::Agent(
                    crate::workflows::authoring::WorkflowAgentNode {
                        capability_profile_id: "capability-default".into(),
                        node_profile: NodeProfile {
                            contract_version:
                                crate::execution_configuration::NODE_PROFILE_CONTRACT_VERSION,
                            allowed_capabilities: capabilities(),
                            pinned_defaults: RuntimeSelections {
                                model: Some("codex-a".into()),
                                reasoning_mode: Some("high".into()),
                                sandbox_mode: Some(SandboxMode::WorkspaceWrite),
                            },
                        },
                        initial_prompt: Some("Plan the work.".into()),
                        agent_identity_id: Some("identity-avery".into()),
                        notifications: Default::default(),
                    },
                ),
            }],
            connections: Vec::new(),
        }
    }

    #[test]
    fn v1_recipe_decodes_as_workflow_otp_connections() {
        let recipe = decode_recipe_value(serde_json::json!({
            "contractVersion": 1,
            "recipeId": "legacy",
            "name": "Legacy",
            "revision": 1,
            "startingNodeId": "planner",
            "nodes": [{
                "nodeId": "planner", "name": "Planner", "positionX": 0.0, "positionY": 0.0,
                "capabilityProfileId": "capability-default",
                "nodeProfile": {"contractVersion": 1, "allowedCapabilities": capabilities(), "pinnedDefaults": RuntimeSelections::default()},
                "initialPrompt": null, "agentIdentityId": null
            }],
            "connections": [{
                "connectionId": "next", "name": "Next", "sourceNodeId": "planner", "destinationNodeId": "planner",
                "trigger": {"kind": "mcp_call", "server": {"namespace":"mcp","kind":"server","id":"workflow_handoff"}, "tool": {"namespace":"mcp","kind":"tool","id":"handoff_to_agent"}},
                "promptInputs": [{"kind":"mcp_argument","name":"promptText"}],
                "promptText": "Continue", "target": {"cardinality":"first","ordering":"newest","running":"any","createdBy":null,"missing":"create"}
            }]
        }))
        .unwrap();
        assert_eq!(recipe.contract_version, WORKFLOW_RECIPE_CONTRACT_VERSION);
        assert_eq!(recipe.entry_node_ids, vec!["planner"]);
        assert_eq!(
            recipe.connections[0].trigger.capability.tool,
            "handoff_to_agent"
        );
        assert_eq!(recipe.connections[0].trigger.output, "handoff");
        assert!(matches!(
            recipe.connections[0].prompt_inputs.as_slice(),
            [WorkflowConnectionPromptInput::OutputField { field }] if field == "promptText"
        ));
    }

    #[test]
    fn complete_draft_compiles_embedded_node_and_capability_profile() {
        let draft = draft();
        let profiles = [("capability-default".into(), profile())]
            .into_iter()
            .collect();

        let input = draft.compilation_input("instance-1", &profiles).unwrap();

        assert_eq!(input.nodes.len(), 1);
        let WorkflowSessionCreation::ResolvedInput(request) = &input.nodes[0].session_creation
        else {
            panic!("expected preview input")
        };
        assert_eq!(
            request.capability_profile.capability_profile_id,
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
        draft.entry_node_ids.clear();

        assert!(draft.validate_storable().is_ok());
        assert!(draft.validate_activatable().is_err());
    }

    #[test]
    fn activation_rejects_node_that_widens_capability_profile() {
        let mut draft = draft();
        draft.nodes[0]
            .agent_mut()
            .unwrap()
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
