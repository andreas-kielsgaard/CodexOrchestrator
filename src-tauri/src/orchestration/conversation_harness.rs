//! Versioned product-owned configuration for orchestration conversations.

use crate::agent_sessions::{
    domain::{AgentRuntimeOptions, RuntimeSandboxMode},
    ports::InitialPromptPrefix,
};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

const CATALOG_JSON: &str = include_str!("conversation_harness_catalog.json");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConversationHarnessRole {
    EpicPlanBuilder,
    EpicBootstrapGenerator,
    EpicRunner,
}

impl ConversationHarnessRole {
    fn key(self) -> &'static str {
        match self {
            Self::EpicPlanBuilder => "epic_plan_builder",
            Self::EpicBootstrapGenerator => "epic_bootstrap_generator",
            Self::EpicRunner => "epic_runner",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HarnessCatalog {
    schema_version: u16,
    harnesses: Vec<ConversationHarnessProfile>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ConversationHarnessProfile {
    pub(crate) key: String,
    pub(crate) version: u16,
    context: String,
    skill_guidance: Vec<SkillGuidance>,
    runtime: HarnessRuntime,
    pub(crate) mcp: HarnessMcpExposure,
    pub(crate) lifecycle: HarnessLifecycle,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SkillGuidance {
    canonical_name: String,
    canonical_path: String,
    purpose: String,
    use_when: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct HarnessRuntime {
    model: Option<String>,
    reasoning_effort: Option<String>,
    sandbox: RuntimeSandboxMode,
    approval_policy: HarnessApprovalPolicy,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum HarnessApprovalPolicy {
    Never,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessMcpExposure {
    pub(crate) required: bool,
    pub(crate) enabled_tools: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessLifecycle {
    pub(crate) context_delivery: HarnessContextDelivery,
    pub(crate) completion_criteria: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessContextDelivery {
    FirstQuery,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationHarnessCatalogProfile {
    pub(crate) catalog_schema_version: u16,
    pub(crate) profile: ConversationHarnessProfile,
}

pub(crate) fn profile(role: ConversationHarnessRole) -> Result<ConversationHarnessProfile, String> {
    catalog_profile(role).map(|catalog| catalog.profile)
}

pub(crate) fn catalog_profile(
    role: ConversationHarnessRole,
) -> Result<ConversationHarnessCatalogProfile, String> {
    catalog_profile_from_catalog(CATALOG_JSON, role.key())
}

fn profile_from_catalog(json: &str, key: &str) -> Result<ConversationHarnessProfile, String> {
    catalog_profile_from_catalog(json, key).map(|catalog| catalog.profile)
}

fn catalog_profile_from_catalog(
    json: &str,
    key: &str,
) -> Result<ConversationHarnessCatalogProfile, String> {
    let catalog: HarnessCatalog = serde_json::from_str(json)
        .map_err(|error| format!("invalid Conversation Harness catalog: {error}"))?;
    if catalog.schema_version != 2 {
        return Err(format!(
            "unsupported Conversation Harness catalog schema version {}",
            catalog.schema_version
        ));
    }
    let configuration = catalog
        .harnesses
        .into_iter()
        .find(|candidate| candidate.key == key)
        .ok_or_else(|| format!("Conversation Harness configuration '{key}' is unavailable"))?;
    validate_profile(&configuration)?;
    Ok(ConversationHarnessCatalogProfile {
        catalog_schema_version: catalog.schema_version,
        profile: configuration,
    })
}

fn validate_profile(profile: &ConversationHarnessProfile) -> Result<(), String> {
    if profile.version == 0 || profile.context.trim().is_empty() {
        return Err(format!(
            "Conversation Harness configuration '{}' is invalid",
            profile.key
        ));
    }
    if profile.skill_guidance.iter().any(|skill| {
        skill.canonical_name.trim().is_empty()
            || !is_safe_repository_skill_path(&skill.canonical_path)
            || skill.purpose.trim().is_empty()
            || skill.use_when.trim().is_empty()
    }) || profile
        .mcp
        .enabled_tools
        .iter()
        .any(|tool| tool.trim().is_empty())
        || profile
            .lifecycle
            .completion_criteria
            .iter()
            .any(|criterion| criterion.trim().is_empty())
        || profile
            .runtime
            .model
            .as_ref()
            .is_some_and(|model| model.trim().is_empty())
        || profile
            .runtime
            .reasoning_effort
            .as_ref()
            .is_some_and(|effort| effort.trim().is_empty())
    {
        return Err(format!(
            "Conversation Harness configuration '{}' has invalid declarative settings",
            profile.key
        ));
    }
    Ok(())
}

impl ConversationHarnessProfile {
    pub(crate) fn initial_prompt_prefix(&self) -> InitialPromptPrefix {
        let mut content = self.context.trim().to_owned();
        if !self.skill_guidance.is_empty() {
            content.push_str("\n\nSkill guidance (availability depends on Codex discovery):");
            for skill in &self.skill_guidance {
                content.push_str(&format!(
                    "\n- {} (canonical repository source: {}): {} Use when {}",
                    skill.canonical_name,
                    skill.canonical_path,
                    skill.purpose.trim(),
                    skill.use_when.trim()
                ));
            }
        }
        InitialPromptPrefix {
            source: self.key.clone(),
            version: self.version,
            content,
        }
    }

    pub(crate) fn runtime_options(&self) -> AgentRuntimeOptions {
        AgentRuntimeOptions {
            model: self.runtime.model.clone(),
            sandbox: Some(self.runtime.sandbox),
        }
    }

    pub(crate) fn runtime_configuration_args(&self) -> Vec<String> {
        let mut values = vec![match self.runtime.approval_policy {
            HarnessApprovalPolicy::Never => "approval_policy=\"never\"".to_string(),
        }];
        if let Some(effort) = self.runtime.reasoning_effort.as_deref() {
            values.push(format!("model_reasoning_effort=\"{effort}\""));
        }
        values
            .into_iter()
            .flat_map(|value| ["-c".into(), value])
            .collect()
    }
}

pub(crate) fn epic_plan_builder_discovery_root() -> Result<String, String> {
    role_discovery_root(ConversationHarnessRole::EpicPlanBuilder)
}

pub(crate) fn role_discovery_root(role: ConversationHarnessRole) -> Result<String, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or_else(|| "Codex Orchestrator repository root is unavailable".to_string())?
        .to_path_buf();
    let profile = profile(role)?;
    let required_skill = match role {
        ConversationHarnessRole::EpicPlanBuilder => "epic-plan-builder",
        ConversationHarnessRole::EpicBootstrapGenerator => "epic-bootstrap-generator",
        ConversationHarnessRole::EpicRunner => "epic-runner",
    };
    let skill = profile
        .skill_guidance
        .iter()
        .find(|skill| skill.canonical_name == required_skill)
        .ok_or_else(|| format!("canonical {required_skill} skill guidance is unavailable"))?;
    if !is_safe_repository_skill_path(&skill.canonical_path) {
        return Err(format!("canonical {required_skill} skill path is invalid"));
    }
    let source = root.join(&skill.canonical_path);
    let content = std::fs::read_to_string(&source).map_err(|error| {
        format!(
            "canonical {required_skill} skill is unavailable at {}: {error}",
            skill.canonical_path
        )
    })?;
    if !content
        .lines()
        .any(|line| line.trim() == format!("name: {required_skill}"))
    {
        return Err(format!(
            "canonical {required_skill} skill metadata is invalid"
        ));
    }
    root.to_str()
        .map(str::to_owned)
        .ok_or_else(|| "Codex Orchestrator repository root is not valid UTF-8".into())
}

fn is_safe_repository_skill_path(value: &str) -> bool {
    let path = Path::new(value);
    !value.trim().is_empty()
        && path.is_relative()
        && path.extension().is_some_and(|extension| extension == "md")
        && !path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_profiles_validate_with_truthful_enforceable_settings() {
        let plan_builder = profile(ConversationHarnessRole::EpicPlanBuilder).unwrap();
        let bootstrap = profile(ConversationHarnessRole::EpicBootstrapGenerator).unwrap();
        let runner = profile(ConversationHarnessRole::EpicRunner).unwrap();

        assert_eq!(plan_builder.version, 4);
        assert_eq!(
            plan_builder.runtime_options().sandbox,
            Some(RuntimeSandboxMode::ReadOnly)
        );
        assert_eq!(
            plan_builder.mcp.enabled_tools,
            ["submit_epic_plan_proposal", "request_epic_initiation"]
        );
        assert!(plan_builder.mcp.required);
        assert_eq!(
            bootstrap.runtime_options().sandbox,
            Some(RuntimeSandboxMode::ReadOnly)
        );
        assert_eq!(bootstrap.mcp.enabled_tools, ["complete_epic_bootstrap"]);
        assert!(bootstrap.mcp.required);
        assert_eq!(
            runner.runtime_options().sandbox,
            Some(RuntimeSandboxMode::ReadOnly)
        );
        assert!(runner.mcp.enabled_tools.is_empty());
        assert_eq!(
            plan_builder.runtime_configuration_args(),
            ["-c", "approval_policy=\"never\""]
        );
        assert!(plan_builder.runtime_options().model.is_none());
        assert!(plan_builder.runtime.reasoning_effort.is_none());
    }

    #[test]
    fn plan_builder_prefix_and_skill_are_first_query_ready() {
        let profile = profile(ConversationHarnessRole::EpicPlanBuilder).unwrap();
        let prefix = profile.initial_prompt_prefix();
        assert!(prefix.content.contains("Ordinary discussion"));
        assert!(prefix.content.contains("request_epic_initiation"));
        assert_eq!(
            profile.lifecycle.context_delivery,
            HarnessContextDelivery::FirstQuery
        );
        let root = PathBuf::from(epic_plan_builder_discovery_root().unwrap());
        assert!(root
            .join(".agents/skills/epic-plan-builder/SKILL.md")
            .is_file());
    }

    #[test]
    fn invalid_unknown_or_old_catalog_fails_truthfully() {
        assert!(profile_from_catalog("{}", "epic_plan_builder")
            .unwrap_err()
            .contains("invalid"));
        let unsupported = r#"{"schemaVersion":1,"harnesses":[]}"#;
        assert!(profile_from_catalog(unsupported, "epic_plan_builder")
            .unwrap_err()
            .contains("unsupported"));
        let missing = r#"{"schemaVersion":2,"harnesses":[]}"#;
        assert!(profile_from_catalog(missing, "epic_plan_builder")
            .unwrap_err()
            .contains("unavailable"));
    }
}
