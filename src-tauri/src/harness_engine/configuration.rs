use serde::{Deserialize, Serialize};

pub(crate) const HARNESS_CONFIGURATION_VERSION: &str = "harness-configuration/v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessMetadata {
    pub(crate) name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum HarnessIdentityAssignmentPolicy {
    Unrestricted,
    AllowList { identity_ids: Vec<String> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessPromptPrefixConfiguration {
    pub(crate) content: String,
    pub(crate) initial_delivery: HarnessInitialDelivery,
    pub(crate) context_compression_delivery: HarnessContextCompressionDelivery,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessInitialDelivery {
    Prepend,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessContextCompressionDelivery {
    Deferred,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessDiscoveryPolicy {
    Whitelist,
    Blacklist,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessSkillsConfiguration {
    pub(crate) available_discovery_policy: HarnessDiscoveryPolicy,
    pub(crate) items: Vec<HarnessSkillConfiguration>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessSkillConfiguration {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) purpose: String,
    pub(crate) use_when: String,
    pub(crate) policy: HarnessSkillPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessSkillPolicy {
    AlwaysApplicable,
    InitialIngestion,
    Available,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessToolsConfiguration {
    pub(crate) available_discovery_policy: HarnessDiscoveryPolicy,
    pub(crate) items: Vec<HarnessToolConfiguration>,
    pub(crate) schema_boundary: String,
    #[serde(default)]
    pub(crate) mcp_servers: Vec<HarnessMcpServerExposure>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessToolConfiguration {
    pub(crate) name: String,
    pub(crate) policy: HarnessToolPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessToolPolicy {
    EveryInvocation,
    InitialInvocation,
    Available,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessMcpServerExposure {
    pub(crate) server_name: String,
    pub(crate) access: HarnessMcpServerAccess,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum HarnessMcpServerAccess {
    EntireServer,
    SelectedTools { tool_names: Vec<String> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessRuntimeConfiguration {
    pub(crate) preferred_model: Option<HarnessModelPreference>,
    pub(crate) sandbox: HarnessSandbox,
    pub(crate) approval_policy: HarnessApprovalPolicy,
    pub(crate) authority_summary: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessModelPreference {
    pub(crate) model_id: String,
    pub(crate) reasoning: Option<HarnessReasoningLevel>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessReasoningLevel {
    Low,
    Medium,
    High,
    Xhigh,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessSandbox {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessApprovalPolicy {
    Never,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessHookConfiguration {
    pub(crate) name: String,
    pub(crate) status: HarnessHookStatus,
    pub(crate) detail: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessHookStatus {
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
pub(crate) enum HarnessUpdatePolicy {
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
pub(crate) enum HarnessUpdateDelivery {
    NextPrompt,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessPromptReconstruction {
    Deferred,
}

/// Retained composite configuration for the current Harness catalog and Session binding path.
/// New capability-exposure and node-invocation contracts live in `execution_configuration`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessConfiguration {
    pub(crate) identity_assignment: HarnessIdentityAssignmentPolicy,
    pub(crate) prompt_prefix: HarnessPromptPrefixConfiguration,
    pub(crate) skills: HarnessSkillsConfiguration,
    pub(crate) tools: HarnessToolsConfiguration,
    pub(crate) runtime: HarnessRuntimeConfiguration,
    pub(crate) hooks: Vec<HarnessHookConfiguration>,
    pub(crate) update_policy: HarnessUpdatePolicy,
}

impl HarnessConfiguration {
    pub(crate) fn validate(&self) -> Result<(), String> {
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
