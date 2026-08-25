use crate::orchestration::conversation_harness_working_copy::{
    HarnessApprovalPolicy, HarnessContextCompressionDelivery, HarnessDiscoveryPolicy,
    HarnessEffectiveConfiguration, HarnessHookConfiguration, HarnessHookStatus,
    HarnessIdentityConfiguration, HarnessInitialDelivery, HarnessMcpServerAccess,
    HarnessMcpServerExposure, HarnessModelConstraint, HarnessModelPolicyMode,
    HarnessPromptPrefixConfiguration, HarnessReasoningLevel, HarnessRuntimeConfiguration,
    HarnessSandbox, HarnessSkillConfiguration, HarnessSkillPolicy, HarnessSkillsConfiguration,
    HarnessToolConfiguration, HarnessToolsConfiguration, HarnessUpdatePolicy,
    HarnessVisualIdentity,
};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

pub(crate) type WorkflowMcpServerExposure = HarnessMcpServerExposure;
pub(crate) type WorkflowMcpServerAccess = HarnessMcpServerAccess;

/// A Workflow Role owns the same detached Harness definition used by Harness Management. Session,
/// version, binding, recipe, and token identity are deliberately stored elsewhere.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub(crate) struct WorkflowHarnessConfig(pub(crate) HarnessEffectiveConfiguration);

impl<'de> Deserialize<'de> for WorkflowHarnessConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if let Ok(configuration) =
            serde_json::from_value::<HarnessEffectiveConfiguration>(value.clone())
        {
            return Ok(Self(configuration));
        }
        serde_json::from_value::<LegacyWorkflowHarnessConfig>(value)
            .map(Self::from_legacy)
            .map_err(D::Error::custom)
    }
}

impl Default for WorkflowHarnessConfig {
    fn default() -> Self {
        Self(HarnessEffectiveConfiguration {
            identity: HarnessIdentityConfiguration {
                name: String::new(),
                machine_key: String::new(),
                permitted_agent_names: None,
                visual_identity: None,
            },
            prompt_prefix: HarnessPromptPrefixConfiguration {
                content: String::new(),
                initial_delivery: HarnessInitialDelivery::Prepend,
                context_compression_delivery: HarnessContextCompressionDelivery::Deferred,
            },
            skills: HarnessSkillsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: vec![],
            },
            tools: HarnessToolsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: vec![],
                schema_boundary: String::new(),
                mcp_servers: vec![],
            },
            runtime: HarnessRuntimeConfiguration {
                model_policy_mode: HarnessModelPolicyMode::RevisionOwned,
                models: vec![],
                default_model: None,
                default_reasoning: None,
                sandbox: HarnessSandbox::WorkspaceWrite,
                sandbox_options: vec![
                    HarnessSandbox::ReadOnly,
                    HarnessSandbox::WorkspaceWrite,
                    HarnessSandbox::DangerFullAccess,
                ],
                approval_policy: HarnessApprovalPolicy::Never,
                approval_policy_options: vec![HarnessApprovalPolicy::Never],
                authority_summary: String::new(),
            },
            hooks: vec![],
            update_policy: HarnessUpdatePolicy::NotConfigured {
                reason: "Session replacement applies activated Workflow Harness changes.".into(),
            },
        })
    }
}

impl WorkflowHarnessConfig {
    fn from_legacy(legacy: LegacyWorkflowHarnessConfig) -> Self {
        let mut configuration = Self::default().0;
        configuration.identity.name = legacy.harness_name.clone();
        configuration.identity.machine_key = machine_key(&legacy.harness_name);
        configuration.prompt_prefix.content = legacy.instructions;
        configuration.skills.items = legacy
            .skills
            .into_iter()
            .map(|name| HarnessSkillConfiguration {
                path: name.clone(),
                purpose: String::new(),
                use_when: String::new(),
                name,
                policy: HarnessSkillPolicy::Available,
            })
            .collect();
        configuration.tools.mcp_servers = legacy.mcp_servers;
        configuration.runtime.authority_summary = legacy.role_identity;
        if !legacy.runtime.model.trim().is_empty() {
            configuration.runtime.models = vec![HarnessModelConstraint {
                model_id: legacy.runtime.model.clone(),
                allowed: true,
                min_reasoning: HarnessReasoningLevel::Low,
                max_reasoning: HarnessReasoningLevel::Xhigh,
            }];
            configuration.runtime.default_model = Some(legacy.runtime.model);
        }
        configuration.runtime.default_reasoning = reasoning(&legacy.runtime.reasoning_effort);
        configuration.hooks = legacy
            .hooks
            .into_iter()
            .map(|name| HarnessHookConfiguration {
                name,
                status: HarnessHookStatus::Exposed,
                detail: String::new(),
            })
            .collect();
        Self(configuration)
    }

    pub(crate) fn exposes_mcp_tool(&self, server_name: &str, tool_name: &str) -> bool {
        self.0.tools.mcp_servers.iter().any(|server| {
            server.server_name == server_name
                && match &server.access {
                    HarnessMcpServerAccess::EntireServer => true,
                    HarnessMcpServerAccess::SelectedTools { tool_names } => {
                        tool_names.iter().any(|name| name == tool_name)
                    }
                }
        })
    }

    pub(crate) fn name(&self) -> &str {
        &self.0.identity.name
    }

    pub(crate) fn prompt_prefix(&self) -> &str {
        &self.0.prompt_prefix.content
    }

    pub(crate) fn default_model(&self) -> &str {
        self.0.runtime.default_model.as_deref().unwrap_or_default()
    }

    pub(crate) fn default_reasoning(&self) -> Option<&'static str> {
        self.0.runtime.default_reasoning.map(|value| match value {
            HarnessReasoningLevel::Low => "low",
            HarnessReasoningLevel::Medium => "medium",
            HarnessReasoningLevel::High => "high",
            HarnessReasoningLevel::Xhigh => "xhigh",
        })
    }

    pub(crate) fn skills(&self) -> &[HarnessSkillConfiguration] {
        &self.0.skills.items
    }

    pub(crate) fn hooks(&self) -> &[HarnessHookConfiguration] {
        &self.0.hooks
    }

    pub(crate) fn mcp_servers(&self) -> &[HarnessMcpServerExposure] {
        &self.0.tools.mcp_servers
    }

    #[cfg(test)]
    pub(crate) fn test_definition(
        name: &str,
        authority: &str,
        prompt_prefix: &str,
        model: &str,
        reasoning_effort: &str,
    ) -> Self {
        let mut harness = Self::default();
        harness.0.identity.name = name.to_string();
        harness.0.identity.machine_key = machine_key(name);
        harness.0.runtime.authority_summary = authority.to_string();
        harness.0.prompt_prefix.content = prompt_prefix.to_string();
        if !model.is_empty() {
            harness.0.runtime.models = vec![HarnessModelConstraint {
                model_id: model.to_string(),
                allowed: true,
                min_reasoning: HarnessReasoningLevel::Low,
                max_reasoning: HarnessReasoningLevel::Xhigh,
            }];
            harness.0.runtime.default_model = Some(model.to_string());
        }
        harness.0.runtime.default_reasoning = reasoning(reasoning_effort);
        harness
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct LegacyWorkflowHarnessConfig {
    harness_name: String,
    role_identity: String,
    instructions: String,
    skills: Vec<String>,
    mcp_servers: Vec<HarnessMcpServerExposure>,
    hooks: Vec<String>,
    runtime: WorkflowHarnessRuntimeSettings,
}

fn machine_key(name: &str) -> String {
    let normalized = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    normalized.trim_matches('_').to_string()
}

fn reasoning(value: &str) -> Option<HarnessReasoningLevel> {
    match value.trim() {
        "low" => Some(HarnessReasoningLevel::Low),
        "medium" => Some(HarnessReasoningLevel::Medium),
        "high" => Some(HarnessReasoningLevel::High),
        "xhigh" => Some(HarnessReasoningLevel::Xhigh),
        _ => None,
    }
}

#[cfg(test)]
mod harness_definition_tests {
    use super::WorkflowHarnessConfig;

    #[test]
    fn legacy_workflow_harness_reopens_as_the_canonical_detached_definition() {
        let harness: WorkflowHarnessConfig = serde_json::from_value(serde_json::json!({
            "harnessName": "Security Review",
            "roleIdentity": "Review trust boundaries.",
            "instructions": "Inspect this change.",
            "skills": ["security-review"],
            "mcpServers": [{
                "serverName": "workflow",
                "access": { "kind": "selected_tools", "toolNames": ["handoff"] }
            }],
            "hooks": ["turn_finished"],
            "runtime": {
                "provider": "codex",
                "model": "gpt-5.6-sol",
                "reasoningEffort": "high"
            }
        }))
        .expect("legacy v43 Harness");

        assert_eq!(harness.0.identity.name, "Security Review");
        assert_eq!(harness.0.prompt_prefix.content, "Inspect this change.");
        assert_eq!(
            harness.0.runtime.authority_summary,
            "Review trust boundaries."
        );
        assert_eq!(harness.0.tools.mcp_servers.len(), 1);

        let canonical = serde_json::to_value(harness).expect("canonical Harness");
        assert_eq!(canonical["identity"]["name"], "Security Review");
        assert!(canonical.get("harnessName").is_none());
        assert_eq!(
            canonical["tools"]["mcpServers"][0]["serverName"],
            "workflow"
        );
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowHarnessRuntimeSettings {
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) reasoning_effort: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct WorkflowHarnessOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) identity_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) identity_machine_key: Option<String>,
    #[serde(
        deserialize_with = "deserialize_explicit_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) permitted_agent_names: Option<Option<Vec<String>>>,
    #[serde(
        deserialize_with = "deserialize_explicit_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) visual_identity: Option<Option<HarnessVisualIdentity>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) prompt_prefix_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) skill_discovery_policy: Option<HarnessDiscoveryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) skill_items: Option<Vec<HarnessSkillConfiguration>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_discovery_policy: Option<HarnessDiscoveryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_items: Option<Vec<HarnessToolConfiguration>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_schema_boundary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) mcp_servers: Option<Vec<WorkflowMcpServerExposure>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) runtime_model_policy_mode: Option<HarnessModelPolicyMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) runtime_models: Option<Vec<HarnessModelConstraint>>,
    #[serde(
        deserialize_with = "deserialize_explicit_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) runtime_default_model: Option<Option<String>>,
    #[serde(
        deserialize_with = "deserialize_explicit_nullable",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) runtime_default_reasoning: Option<Option<HarnessReasoningLevel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) runtime_sandbox: Option<HarnessSandbox>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) runtime_sandbox_options: Option<Vec<HarnessSandbox>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) runtime_approval_policy: Option<HarnessApprovalPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) runtime_approval_policy_options: Option<Vec<HarnessApprovalPolicy>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) runtime_authority_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) hook_items: Option<Vec<HarnessHookConfiguration>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) update_policy: Option<HarnessUpdatePolicy>,
    // v43 compatibility fields. New editors write only canonical leaf fields.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) harness_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) role_identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) skills: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) hooks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) runtime: Option<WorkflowHarnessRuntimeSettings>,
}

fn deserialize_explicit_nullable<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer).map(Some)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum WorkflowNodeHarness {
    Role {
        role_id: String,
        #[serde(default)]
        overrides: WorkflowHarnessOverrides,
    },
    Standalone {
        config: WorkflowHarnessConfig,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowRole {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) harness: WorkflowHarnessConfig,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowTypeSummary {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) active_recipe_id: Option<String>,
    pub(crate) edited_element_count: u32,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowNodeConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) harness_name: String,
    pub(crate) role_name: Option<String>,
    pub(crate) position_x: f64,
    pub(crate) position_y: f64,
    pub(crate) is_starting_point: bool,
    #[serde(default)]
    pub(crate) harness: Option<WorkflowNodeHarness>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowConnectionConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) sender_node_id: String,
    pub(crate) receiver_node_id: Option<String>,
    #[serde(default)]
    pub(crate) mechanism: Option<WorkflowConnectionMechanism>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum WorkflowConnectionMechanism {
    TurnFinishedExpectedFile {
        #[serde(alias = "file_selector")]
        file_selector: WorkflowExpectedFileSelector,
        #[serde(alias = "description_text")]
        description_text: String,
        #[serde(alias = "prompt_text")]
        prompt_text: String,
        #[serde(alias = "match_selection")]
        match_selection: WorkflowMatchSelection,
        #[serde(alias = "initial_check")]
        initial_check: WorkflowInitialCheck,
    },
    McpNativePromptAgent {
        #[serde(alias = "server_name")]
        server_name: String,
        #[serde(alias = "tool_name")]
        tool_name: String,
        #[serde(alias = "warning_text")]
        warning_text: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowMcpComponent {
    pub(crate) server_name: String,
    pub(crate) tool_name: String,
    pub(crate) title: String,
    pub(crate) participation_mode: String,
    pub(crate) interface_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowInvocation {
    pub(crate) contract_version: String,
    pub(crate) connection_activation_reference: String,
    pub(crate) recipe_reference: String,
    pub(crate) connection_reference: String,
    pub(crate) sender_node_reference: String,
    pub(crate) sender_activation_reference: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparedWorkflowMcpHandoff {
    pub(crate) invocation: WorkflowInvocation,
    pub(crate) warning_text: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowMcpOutput {
    pub(crate) file_paths: Vec<String>,
    pub(crate) prompt_text: String,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowMcpActivationContext {
    pub(crate) activation_id: String,
    pub(crate) trigger: WorkflowCompletedTurnTrigger,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum WorkflowExpectedFileSelector {
    FolderFilenamePattern {
        folder: String,
        #[serde(alias = "filename_pattern")]
        filename_pattern: String,
    },
    FolderOutputRegex {
        folder: String,
        #[serde(alias = "output_regex")]
        output_regex: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowMatchSelection {
    Newest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowInitialCheck {
    OnceImmediately,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowNodeElement {
    pub(crate) id: String,
    pub(crate) draft: Option<WorkflowNodeConfig>,
    pub(crate) live: Option<WorkflowNodeConfig>,
    pub(crate) has_unpublished_changes: bool,
    pub(crate) draft_effective_harness: Option<WorkflowHarnessConfig>,
    pub(crate) live_effective_harness: Option<WorkflowHarnessConfig>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EffectiveWorkflowNodeConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) position_x: f64,
    pub(crate) position_y: f64,
    pub(crate) is_starting_point: bool,
    pub(crate) harness: WorkflowHarnessConfig,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowConnectionElement {
    pub(crate) id: String,
    pub(crate) draft: Option<WorkflowConnectionConfig>,
    pub(crate) live: Option<WorkflowConnectionConfig>,
    pub(crate) has_unpublished_changes: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EffectiveRecipe {
    pub(crate) id: String,
    pub(crate) workflow_type_id: String,
    pub(crate) ordinal: u32,
    pub(crate) created_at: String,
    pub(crate) nodes: Vec<EffectiveWorkflowNodeConfig>,
    pub(crate) connections: Vec<WorkflowConnectionConfig>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowDefinition {
    pub(crate) workflow_type: WorkflowTypeSummary,
    pub(crate) nodes: Vec<WorkflowNodeElement>,
    pub(crate) connections: Vec<WorkflowConnectionElement>,
    pub(crate) active_recipe: Option<EffectiveRecipe>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowConnectionActivationStatus {
    Requested,
    Resolved,
    Associated,
    LaunchRequested,
    LaunchAccepted,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowConnectionActivation {
    pub(crate) id: String,
    pub(crate) recipe_id: String,
    pub(crate) connection_id: String,
    pub(crate) sender_node_id: String,
    pub(crate) receiver_node_id: String,
    pub(crate) source_session_id: String,
    pub(crate) source_invocation_id: String,
    pub(crate) target_session_id: Option<String>,
    pub(crate) target_invocation_id: Option<String>,
    pub(crate) delivery_kind: String,
    pub(crate) session_mode: Option<String>,
    pub(crate) context_inheritance: String,
    pub(crate) compression: String,
    pub(crate) resolved_file_path: Option<String>,
    pub(crate) status: WorkflowConnectionActivationStatus,
    pub(crate) requested_at: String,
    pub(crate) resolved_at: Option<String>,
    pub(crate) associated_at: Option<String>,
    pub(crate) launch_requested_at: Option<String>,
    pub(crate) launch_accepted_at: Option<String>,
    pub(crate) failed_at: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowCompletedTurnTrigger {
    pub(crate) workflow_instance_id: String,
    pub(crate) worktree_root: String,
    pub(crate) sender_node_id: String,
    pub(crate) recipe: EffectiveRecipe,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowConnectionActivationPreparation {
    pub(crate) id: String,
    pub(crate) workflow_instance_id: String,
    pub(crate) recipe_id: String,
    pub(crate) connection_id: String,
    pub(crate) sender_node_id: String,
    pub(crate) receiver_node_id: String,
    pub(crate) source_session_id: String,
    pub(crate) source_invocation_id: String,
    pub(crate) requested_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkflowConnectionActivationRecord {
    pub(crate) id: String,
    pub(crate) workflow_instance_id: String,
    pub(crate) recipe_id: String,
    pub(crate) connection_id: String,
    pub(crate) sender_node_id: String,
    pub(crate) receiver_node_id: String,
    pub(crate) source_session_id: String,
    pub(crate) source_invocation_id: String,
    pub(crate) target_session_id: Option<String>,
    pub(crate) target_invocation_id: Option<String>,
    pub(crate) delivery_kind: String,
    pub(crate) session_mode: Option<String>,
    pub(crate) context_inheritance: String,
    pub(crate) compression: String,
    pub(crate) resolved_file_path: Option<String>,
    pub(crate) resolved_output_json: Option<String>,
    pub(crate) requested_at: String,
    pub(crate) resolved_at: Option<String>,
    pub(crate) associated_at: Option<String>,
    pub(crate) launch_requested_at: Option<String>,
    pub(crate) launch_accepted_at: Option<String>,
    pub(crate) failed_at: Option<String>,
    pub(crate) failure_stage: Option<String>,
    pub(crate) failure_reason: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowElementKind {
    Node,
    Connection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowElementRef {
    pub(crate) kind: WorkflowElementKind,
    pub(crate) id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowNativeQuery {
    pub(crate) schema_version: &'static str,
    pub(crate) workflow_types: Vec<WorkflowDefinition>,
    pub(crate) roles: Vec<WorkflowRole>,
}
