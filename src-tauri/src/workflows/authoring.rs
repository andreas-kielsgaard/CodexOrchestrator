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
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const WORKFLOW_RECIPE_CONTRACT_VERSION: u32 = 2;

/// Reads persisted V1 drafts into the OTP-shaped recipe contract. The next save persists V2.
pub(crate) fn decode_recipe_value(value: Value) -> Result<WorkflowRecipeDraft, String> {
    let value = match value.get("contractVersion").and_then(Value::as_u64) {
        Some(1) => migrate_v1_recipe(value)?,
        Some(version) if version == WORKFLOW_RECIPE_CONTRACT_VERSION as u64 => value,
        Some(version) => {
            return Err(format!(
                "Workflow recipe contract version {version} is unsupported"
            ))
        }
        None => return Err("Workflow recipe has no contractVersion".into()),
    };
    let draft: WorkflowRecipeDraft = serde_json::from_value(value)
        .map_err(|error| format!("Unable to decode Workflow recipe: {error}"))?;
    draft.validate_storable()?;
    Ok(draft)
}

fn migrate_v1_recipe(mut value: Value) -> Result<Value, String> {
    let recipe = value
        .as_object_mut()
        .ok_or("Workflow recipe must be an object")?;
    recipe.insert(
        "contractVersion".into(),
        json!(WORKFLOW_RECIPE_CONTRACT_VERSION),
    );
    recipe.insert(
        "entryAction".into(),
        json!({"package":"workflow","tool":"prompt_agent"}),
    );
    recipe.insert("entryConfiguration".into(), json!({}));
    for node in required_array_mut(recipe, "nodes")? {
        node.as_object_mut()
            .ok_or("Workflow node must be an object")?
            .entry("agentMcpConfiguration")
            .or_insert_with(|| json!({}));
    }
    let connections = required_array_mut(recipe, "connections")?;
    for connection in connections {
        migrate_v1_connection(connection)?;
    }
    Ok(value)
}

fn required_array_mut<'a>(
    object: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Vec<Value>, String> {
    object
        .get_mut(key)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("Workflow recipe {key} must be an array"))
}

fn migrate_v1_connection(value: &mut Value) -> Result<(), String> {
    let connection = value
        .as_object_mut()
        .ok_or("Workflow connection must be an object")?;
    let trigger = connection
        .remove("trigger")
        .ok_or("Workflow connection has no trigger")?;
    let target = connection.remove("target").unwrap_or_else(|| json!({}));
    let prompt_inputs = connection
        .remove("promptInputs")
        .and_then(|value| value.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .map(migrate_v1_prompt_input)
        .collect::<Result<Vec<_>, _>>()?;
    connection.insert("trigger".into(), migrate_v1_trigger(trigger)?);
    connection.insert(
        "action".into(),
        json!({"package":"workflow","tool":"prompt_agent"}),
    );
    connection.insert("configuration".into(), v1_target_configuration(&target));
    connection.insert("promptInputs".into(), Value::Array(prompt_inputs));
    Ok(())
}

fn migrate_v1_trigger(value: Value) -> Result<Value, String> {
    let trigger = value
        .as_object()
        .ok_or("Workflow trigger must be an object")?;
    let kind = trigger
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tool = match kind {
        "invocation_completed" => "on_invocation_completed",
        "mcp_call" => match trigger
            .get("tool")
            .and_then(|tool| tool.get("id"))
            .and_then(Value::as_str)
        {
            Some("handoff_to_agent") => "handoff_to_agent",
            Some("trigger_workflow_continuation") => "trigger_workflow_continuation",
            Some(tool) => return Err(format!("Cannot migrate Workflow MCP trigger `{tool}`")),
            None => return Err("Cannot migrate Workflow MCP trigger without a tool id".into()),
        },
        "application_event" | "event_group_completed" => {
            return Err(format!(
                "Cannot migrate Workflow trigger kind `{kind}` to an OTP output"
            ))
        }
        _ => {
            return Err(format!(
                "Cannot migrate unknown Workflow trigger kind `{kind}`"
            ))
        }
    };
    let output = match tool {
        "on_invocation_completed" => "completed",
        "handoff_to_agent" => "handoff",
        "trigger_workflow_continuation" => "continuation",
        _ => unreachable!(),
    };
    Ok(json!({"capability":{"package":"workflow","tool":tool},"output":output}))
}

fn migrate_v1_prompt_input(value: Value) -> Result<Value, String> {
    let input = value
        .as_object()
        .ok_or("Workflow prompt input must be an object")?;
    let kind = input
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match kind {
        "invocation_output" => Ok(json!({"kind":"output_field","field":"output"})),
        "mcp_argument" => Ok(json!({
            "kind":"output_field",
            "field": input.get("name").and_then(Value::as_str).unwrap_or_default(),
        })),
        "application_event_field" => Ok(json!({
            "kind":"output_field",
            "field": input.get("field").and_then(Value::as_str).unwrap_or_default(),
        })),
        "referenced_content" => Ok(json!({
            "kind":"file_content",
            "path": input.get("reference").and_then(|reference| reference.get("id")).and_then(Value::as_str).unwrap_or_default(),
        })),
        _ => Err(format!(
            "Cannot migrate Workflow prompt input kind `{kind}`"
        )),
    }
}

fn v1_target_configuration(target: &Value) -> Value {
    let source = target.as_object();
    let value = |key: &str, fallback: &str| {
        source
            .and_then(|target| target.get(key))
            .and_then(Value::as_str)
            .unwrap_or(fallback)
            .to_string()
    };
    json!({
        "mode":"select",
        "cardinality":value("cardinality", "first"),
        "ordering":value("ordering", "newest"),
        "running":value("running", "any"),
        "missing":value("missing", "create"),
    })
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkflowRecipeDraft {
    pub(crate) contract_version: u32,
    pub(crate) recipe_id: String,
    pub(crate) name: String,
    pub(crate) revision: u64,
    pub(crate) starting_node_id: Option<String>,
    pub(crate) entry_action: CapabilityRef,
    #[serde(default = "empty_configuration")]
    pub(crate) entry_configuration: serde_json::Value,
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
    #[serde(default)]
    pub(crate) agent_mcp_configuration: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkflowAuthoringConnection {
    pub(crate) connection_id: String,
    pub(crate) name: String,
    pub(crate) source_node_id: String,
    pub(crate) destination_node_id: String,
    pub(crate) trigger: OutputRef,
    pub(crate) action: CapabilityRef,
    pub(crate) configuration: serde_json::Value,
    pub(crate) prompt_inputs: Vec<WorkflowConnectionPromptInput>,
    pub(crate) prompt_text: String,
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

    pub(crate) fn compilation_input(
        &self,
        instance_id: &str,
        capability_profiles: &BTreeMap<String, CapabilityProfile>,
    ) -> Result<WorkflowCompiledPlan, String> {
        self.build_compilation_input(instance_id, Some(capability_profiles), "")
    }

    pub(crate) fn runtime_compilation_input(
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
                let session_creation = if let Some(capability_profiles) = capability_profiles {
                    let capability_profile = capability_profiles
                        .get(&node.capability_profile_id)
                        .ok_or_else(|| {
                        format!(
                            "Workflow node `{}` references unavailable Capability Profile `{}`",
                            node.node_id, node.capability_profile_id
                        )
                    })?;
                    validate_node_capabilities(node, capability_profile)?;
                    WorkflowSessionCreation::ResolvedInput(SessionCreationRequest {
                        contract_version: 1,
                        capability_profile: capability_profile.clone(),
                        node_profile: node.node_profile.clone(),
                        agent_mcp_configuration: node.agent_mcp_configuration.clone(),
                        session_skill_inputs: Vec::new(),
                    })
                } else {
                    WorkflowSessionCreation::AtBirth(
                        crate::execution_configuration::SessionCreationIntent {
                            capability_profile_id: node.capability_profile_id.clone(),
                            node_profile: node.node_profile.clone(),
                            agent_mcp_configuration: node.agent_mcp_configuration.clone(),
                            working_directory: working_directory.into(),
                            title: node.name.clone(),
                        },
                    )
                };
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
            entry_action: self.entry_action.clone(),
            entry_configuration: self.entry_configuration.clone(),
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
    // Route-model allowances are design-time intent. MCP/skill narrowing is checked
    // against the selected route when a Session instance is materialized.
    if !capability_profile.route_policies.is_empty() {
        return Ok(());
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
            starting_node_id: Some("planner".into()),
            entry_configuration: serde_json::json!({}),
            entry_action: crate::otp_api::CapabilityRef {
                package: "workflow".into(),
                tool: "prompt_agent".into(),
            },
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
                agent_mcp_configuration: Default::default(),
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
        assert_eq!(recipe.entry_action.tool, "prompt_agent");
        assert!(recipe.nodes[0].agent_mcp_configuration.is_empty());
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

fn empty_configuration() -> serde_json::Value {
    serde_json::json!({})
}
