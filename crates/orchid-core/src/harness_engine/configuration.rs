use serde::{Deserialize, Serialize};

pub const HARNESS_CONFIGURATION_VERSION: &str = "harness-configuration/v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessMetadata {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HarnessIdentityAssignmentPolicy {
    Unrestricted,
    AllowList { identity_ids: Vec<String> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessPromptPrefixConfiguration {
    pub content: String,
    pub initial_delivery: HarnessInitialDelivery,
    pub context_compression_delivery: HarnessContextCompressionDelivery,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessInitialDelivery {
    Prepend,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessContextCompressionDelivery {
    Deferred,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessDiscoveryPolicy {
    Whitelist,
    Blacklist,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessSkillsConfiguration {
    pub available_discovery_policy: HarnessDiscoveryPolicy,
    pub items: Vec<HarnessSkillConfiguration>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessSkillConfiguration {
    pub name: String,
    pub path: String,
    pub purpose: String,
    pub use_when: String,
    pub policy: HarnessSkillPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessSkillPolicy {
    AlwaysApplicable,
    InitialIngestion,
    Available,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessToolsConfiguration {
    pub available_discovery_policy: HarnessDiscoveryPolicy,
    pub items: Vec<HarnessToolConfiguration>,
    pub schema_boundary: String,
    #[serde(default)]
    pub mcp_servers: Vec<HarnessMcpServerExposure>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessToolConfiguration {
    pub name: String,
    pub policy: HarnessToolPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessToolPolicy {
    EveryInvocation,
    InitialInvocation,
    Available,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessMcpServerExposure {
    pub server_name: String,
    pub access: HarnessMcpServerAccess,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HarnessMcpServerAccess {
    EntireServer,
    SelectedTools { tool_names: Vec<String> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessRuntimeConfiguration {
    pub preferred_model: Option<HarnessModelPreference>,
    pub sandbox: HarnessSandbox,
    pub approval_policy: HarnessApprovalPolicy,
    pub authority_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessModelPreference {
    pub model_id: String,
    pub reasoning: Option<HarnessReasoningLevel>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessReasoningLevel {
    Low,
    Medium,
    High,
    Xhigh,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessSandbox {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessApprovalPolicy {
    Never,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessHookConfiguration {
    pub name: String,
    pub status: HarnessHookStatus,
    pub detail: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessHookStatus {
    Exposed,
    Proposed,
    NotConnected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HarnessUpdatePolicy {
    Configured {
        delivery: HarnessUpdateDelivery,
        avoid_duplicate_guidance: bool,
        notify_removed_items: bool,
        prompt_reconstruction: HarnessPromptReconstruction,
    },
    NotConfigured {
        reason: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessUpdateDelivery {
    NextPrompt,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessPromptReconstruction {
    Deferred,
}

/// Retained composite configuration for the current Harness catalog and Session binding path.
/// New capability-exposure and node-invocation contracts live in `execution_configuration`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HarnessConfiguration {
    pub identity_assignment: HarnessIdentityAssignmentPolicy,
    pub prompt_prefix: HarnessPromptPrefixConfiguration,
    pub skills: HarnessSkillsConfiguration,
    pub tools: HarnessToolsConfiguration,
    pub runtime: HarnessRuntimeConfiguration,
    pub hooks: Vec<HarnessHookConfiguration>,
    pub update_policy: HarnessUpdatePolicy,
}

impl HarnessConfiguration {
    pub fn validate(&self) -> Result<(), String> {
        required(&self.prompt_prefix.content, "Harness prompt prefix")?;
        required(&self.tools.schema_boundary, "Harness tool schema boundary")?;
        required(&self.runtime.authority_summary, "Harness authority summary")?;
        if let HarnessIdentityAssignmentPolicy::AllowList { identity_ids } =
            &self.identity_assignment
        {
            unique_required(identity_ids, "Harness identity allow-list")?;
        }
        unique_required(
            &self
                .skills
                .items
                .iter()
                .map(|item| item.name.clone())
                .collect::<Vec<_>>(),
            "Harness skills",
        )?;
        unique_required(
            &self
                .tools
                .items
                .iter()
                .map(|item| item.name.clone())
                .collect::<Vec<_>>(),
            "Harness tools",
        )?;
        unique_required(
            &self
                .tools
                .mcp_servers
                .iter()
                .map(|item| item.server_name.clone())
                .collect::<Vec<_>>(),
            "Harness MCP servers",
        )?;
        if let Some(preference) = &self.runtime.preferred_model {
            required(&preference.model_id, "preferred model")?;
        }
        Ok(())
    }
}

fn required(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{label} must not be empty."))
    } else {
        Ok(())
    }
}

fn unique_required(values: &[String], label: &str) -> Result<(), String> {
    let mut normalized = values.iter().map(|value| value.trim()).collect::<Vec<_>>();
    if normalized.iter().any(|value| value.is_empty()) {
        return Err(format!("{label} must not contain empty values."));
    }
    normalized.sort_unstable();
    if normalized.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(format!("{label} must not contain duplicates."));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configuration() -> HarnessConfiguration {
        HarnessConfiguration {
            identity_assignment: HarnessIdentityAssignmentPolicy::Unrestricted,
            prompt_prefix: HarnessPromptPrefixConfiguration {
                content: "You are a reviewer.".into(),
                initial_delivery: HarnessInitialDelivery::Prepend,
                context_compression_delivery: HarnessContextCompressionDelivery::Deferred,
            },
            skills: HarnessSkillsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: Vec::new(),
            },
            tools: HarnessToolsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: Vec::new(),
                schema_boundary: "Application tool contracts.".into(),
                mcp_servers: Vec::new(),
            },
            runtime: HarnessRuntimeConfiguration {
                preferred_model: None,
                sandbox: HarnessSandbox::WorkspaceWrite,
                approval_policy: HarnessApprovalPolicy::Never,
                authority_summary: "User-directed application authority.".into(),
            },
            hooks: Vec::new(),
            update_policy: HarnessUpdatePolicy::NotConfigured {
                reason: "No live update protocol.".into(),
            },
        }
    }

    #[test]
    fn legacy_catalog_configuration_has_no_machine_or_provider_identity() {
        let value = serde_json::to_value(configuration()).unwrap();

        assert!(value.get("machineKey").is_none());
        assert!(value.get("provider").is_none());
        assert!(value["runtime"].get("models").is_none());
        assert!(configuration().validate().is_ok());
    }

    #[test]
    fn identity_scope_is_about_assignment_not_session_authority() {
        let mut candidate = configuration();
        candidate.identity_assignment = HarnessIdentityAssignmentPolicy::AllowList {
            identity_ids: vec!["avery".into(), "avery".into()],
        };

        assert!(candidate.validate().unwrap_err().contains("duplicates"));
    }
}
