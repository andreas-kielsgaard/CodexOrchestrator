//! Versioned product-owned configuration for orchestration conversations.

use super::conversation_harness_working_copy::{
    HarnessApprovalPolicy as RevisionApprovalPolicy, HarnessContextCompressionDelivery,
    HarnessDiscoveryPolicy, HarnessEffectiveConfiguration, HarnessHookConfiguration,
    HarnessHookStatus, HarnessIdentityConfiguration, HarnessInitialDelivery,
    HarnessModelConstraint, HarnessModelPolicyMode, HarnessPromptPrefixConfiguration,
    HarnessReasoningLevel, HarnessRuntimeConfiguration, HarnessSandbox, HarnessSkillConfiguration,
    HarnessSkillPolicy, HarnessSkillsConfiguration, HarnessToolConfiguration, HarnessToolPolicy, HarnessToolsConfiguration, HarnessUpdatePolicy,
    HarnessVisualIdentity,
};
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
    EpicRunnerEscalationReassessment,
    SprintRunner,
    SprintRunnerPlanningControl,
    SprintRunnerHandbackReassessment,
    WorkSlicePlanner,
    WorkUnitHandler,
    WorkUnitImplementer,
}

impl ConversationHarnessRole {
    fn key(self) -> &'static str {
        match self {
            Self::EpicPlanBuilder => "epic_plan_builder",
            Self::EpicBootstrapGenerator => "epic_bootstrap_generator",
            Self::EpicRunner => "epic_runner",
            Self::EpicRunnerEscalationReassessment => "epic_runner_escalation_reassessment",
            Self::SprintRunner => "sprint_runner",
            Self::SprintRunnerPlanningControl => "sprint_runner_planning_control",
            Self::SprintRunnerHandbackReassessment => "sprint_runner_handback_reassessment",
            Self::WorkSlicePlanner => "work_slice_planner",
            Self::WorkUnitHandler => "work_unit_handler",
            Self::WorkUnitImplementer => "work_unit_implementer",
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

/// The only catalog-to-revision bridge. It is used solely to bootstrap the first application-owned
/// immutable Handler revision; later activation loads verified revision evidence, never this catalog.
pub(crate) fn initial_work_unit_handler_revision_configuration(
) -> Result<HarnessEffectiveConfiguration, String> {
    let profile = profile(ConversationHarnessRole::WorkUnitHandler)?;
    if profile.mcp.enabled_tools != ["request_work_unit_implementer"] {
        return Err("Work Unit Handler catalog profile exposes unsupported MCP tools".into());
    }
    Ok(HarnessEffectiveConfiguration {
        identity: HarnessIdentityConfiguration {
            name: "Work Unit Handler".into(),
            machine_key: profile.key,
            permitted_agent_names: None,
            visual_identity: Some(HarnessVisualIdentity {
                token: "handler".into(),
                accent: "blue".into(),
            }),
        },
        prompt_prefix: HarnessPromptPrefixConfiguration {
            content: profile.context,
            initial_delivery: HarnessInitialDelivery::Prepend,
            context_compression_delivery: HarnessContextCompressionDelivery::Deferred,
        },
        skills: HarnessSkillsConfiguration {
            available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
            items: profile
                .skill_guidance
                .into_iter()
                .map(|skill| HarnessSkillConfiguration {
                    name: skill.canonical_name,
                    path: skill.canonical_path,
                    purpose: skill.purpose,
                    use_when: skill.use_when,
                    policy: HarnessSkillPolicy::AlwaysApplicable,
                })
                .collect(),
        },
        tools: HarnessToolsConfiguration {
            available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
            items: vec![HarnessToolConfiguration { name: "request_work_unit_implementer".into(), policy: HarnessToolPolicy::Available }],
            schema_boundary: "Only the application-derived Handler-to-Implementer request is exposed.".into(),
        },
        runtime: HarnessRuntimeConfiguration {
            model_policy_mode: HarnessModelPolicyMode::RevisionOwned,
            models: profile
                .runtime
                .model
                .iter()
                .map(|model| HarnessModelConstraint {
                    model_id: model.clone(),
                    allowed: true,
                    min_reasoning: HarnessReasoningLevel::Low,
                    max_reasoning: HarnessReasoningLevel::Xhigh,
                })
                .collect(),
            default_model: profile.runtime.model,
            default_reasoning: profile
                .runtime
                .reasoning_effort
                .as_deref()
                .and_then(reasoning_from_catalog),
            sandbox: sandbox_to_revision(profile.runtime.sandbox),
            sandbox_options: vec![sandbox_to_revision(profile.runtime.sandbox)],
            approval_policy: RevisionApprovalPolicy::Never,
            approval_policy_options: vec![RevisionApprovalPolicy::Never],
            authority_summary:
                "Read-only bounded Handler evidence plus one application-derived Implementer request."
                    .into(),
        },
        hooks: vec![HarnessHookConfiguration {
            name: "completion".into(),
            status: HarnessHookStatus::NotConnected,
            detail: "No Handler completion, review, or settlement hook is connected.".into(),
        }],
        update_policy: HarnessUpdatePolicy::NotConfigured {
            reason: "Pinned revision is immutable.".into(),
        },
    })
}

/// The initial Handler invocation is deliberately actionless.  The action belongs to the later
/// same-Session continuation, which publishes the configuration above as a new revision.
pub(crate) fn initial_work_unit_handler_baseline_revision_configuration(
) -> Result<HarnessEffectiveConfiguration, String> {
    let mut configuration = initial_work_unit_handler_revision_configuration()?;
    configuration.prompt_prefix.content = "You are the Work Unit Handler for an already-authorized execution attempt. This original immutable Handler invocation is read-only and exposes no downstream action. Inspect only the application-supplied bounded evidence. Do not create Work Units, execution attempts, sessions, Implementers, requests, outcomes, acceptance, integration, settlement, or continuations.".into();
    configuration.tools.items.clear();
    configuration.tools.schema_boundary = "No Handler action is exposed by the original invocation.".into();
    configuration.runtime.authority_summary = "Read-only bounded Handler evidence; no downstream action.".into();
    Ok(configuration)
}

/// A third, separate Handler revision used only after the application has accepted one exact
/// Implementer outcome for independent review. It leaves historical Handler revisions unchanged.
pub(crate) fn handler_outcome_review_revision_configuration(
) -> Result<HarnessEffectiveConfiguration, String> {
    let mut configuration = initial_work_unit_handler_baseline_revision_configuration()?;
    configuration.prompt_prefix.content = "You are the independent Work Unit Handler review continuation for one application-accepted Implementer outcome. Read only the bounded application-delivered claims and evidence. Submit exactly one identity-free accept or structured return judgment, then end successfully. Do not request an Implementer, create a retry, settle planning, activate dependents, or continue Sprint or Epic work.".into();
    configuration.tools.items = vec![
        HarnessToolConfiguration { name: "read_handler_review_evidence".into(), policy: HarnessToolPolicy::Available },
        HarnessToolConfiguration { name: "accept_implementation_outcome".into(), policy: HarnessToolPolicy::Available },
        HarnessToolConfiguration { name: "return_implementation_outcome".into(), policy: HarnessToolPolicy::Available },
    ];
    configuration.tools.schema_boundary = "Only application-bound review evidence plus one identity-free accept or structured return judgment are exposed.".into();
    configuration.runtime.authority_summary = "Read-only exact attempt evidence and one non-settling Handler review judgment.".into();
    configuration.hooks = vec![HarnessHookConfiguration { name: "completion".into(), status: HarnessHookStatus::NotConnected, detail: "A judgment is not durable until the application observes this exact review invocation Completed.".into() }];
    Ok(configuration)
}

pub(crate) fn profile_from_immutable_handler_revision(
    configuration: &HarnessEffectiveConfiguration,
    revision_version: u16,
) -> Result<ConversationHarnessProfile, String> {
    let tools = configuration.tools.items.iter().map(|tool| tool.name.as_str()).collect::<Vec<_>>();
    let valid_tools = tools.is_empty()
        || tools.as_slice() == ["request_work_unit_implementer"]
        || tools.as_slice() == ["read_handler_review_evidence", "accept_implementation_outcome", "return_implementation_outcome"];
    if configuration.identity.machine_key != "work_unit_handler"
        || !valid_tools
        || configuration.tools.items.iter().any(|tool| tool.policy != HarnessToolPolicy::Available)
        || configuration.runtime.sandbox != HarnessSandbox::ReadOnly
        || configuration.runtime.approval_policy != RevisionApprovalPolicy::Never
    {
        return Err("immutable Handler revision is outside the bounded Handler contract".into());
    }
    let model = configuration.runtime.default_model.clone();
    let reasoning_effort = configuration
        .runtime
        .default_reasoning
        .map(reasoning_to_catalog);
    Ok(ConversationHarnessProfile {
        key: configuration.identity.machine_key.clone(),
        version: revision_version,
        context: configuration.prompt_prefix.content.clone(),
        skill_guidance: configuration
            .skills
            .items
            .iter()
            .map(|skill| SkillGuidance {
                canonical_name: skill.name.clone(),
                canonical_path: skill.path.clone(),
                purpose: skill.purpose.clone(),
                use_when: skill.use_when.clone(),
            })
            .collect(),
        runtime: HarnessRuntime {
            model,
            reasoning_effort,
            sandbox: sandbox_from_revision(configuration.runtime.sandbox)?,
            approval_policy: HarnessApprovalPolicy::Never,
        },
        // An empty historical revision is an immutable v1 read-only Handler.  Do not infer the
        // later action merely because the current catalog now exposes it.
        mcp: HarnessMcpExposure {
            required: !configuration.tools.items.is_empty(),
            enabled_tools: configuration.tools.items.iter().map(|tool| tool.name.clone()).collect(),
        },
        lifecycle: HarnessLifecycle {
            context_delivery: HarnessContextDelivery::FirstQuery,
            completion_criteria: vec![
                "application_observed_bounded_evidence_or_terminal_observation".into(),
            ],
        },
    })
}

pub(crate) fn initial_work_unit_implementer_revision_configuration() -> Result<HarnessEffectiveConfiguration, String> {
    let profile = profile(ConversationHarnessRole::WorkUnitImplementer)?;
    if profile.mcp.required || !profile.mcp.enabled_tools.is_empty() { return Err("Work Unit Implementer catalog profile exposes unsupported MCP tools".into()); }
    Ok(HarnessEffectiveConfiguration {
        identity: HarnessIdentityConfiguration { name: "Work Unit Implementer".into(), machine_key: profile.key, permitted_agent_names: None, visual_identity: Some(HarnessVisualIdentity { token: "implementer".into(), accent: "green".into() }) },
        prompt_prefix: HarnessPromptPrefixConfiguration { content: profile.context, initial_delivery: HarnessInitialDelivery::Prepend, context_compression_delivery: HarnessContextCompressionDelivery::Deferred },
        skills: HarnessSkillsConfiguration { available_discovery_policy: HarnessDiscoveryPolicy::Whitelist, items: profile.skill_guidance.into_iter().map(|skill| HarnessSkillConfiguration { name: skill.canonical_name, path: skill.canonical_path, purpose: skill.purpose, use_when: skill.use_when, policy: HarnessSkillPolicy::AlwaysApplicable }).collect() },
        tools: HarnessToolsConfiguration { available_discovery_policy: HarnessDiscoveryPolicy::Whitelist, items: vec![], schema_boundary: "No Implementer MCP action is exposed.".into() },
        runtime: HarnessRuntimeConfiguration { model_policy_mode: HarnessModelPolicyMode::RevisionOwned, models: profile.runtime.model.iter().map(|model| HarnessModelConstraint { model_id: model.clone(), allowed: true, min_reasoning: HarnessReasoningLevel::Low, max_reasoning: HarnessReasoningLevel::Xhigh }).collect(), default_model: profile.runtime.model, default_reasoning: profile.runtime.reasoning_effort.as_deref().and_then(reasoning_from_catalog), sandbox: sandbox_to_revision(profile.runtime.sandbox), sandbox_options: vec![sandbox_to_revision(profile.runtime.sandbox)], approval_policy: RevisionApprovalPolicy::Never, approval_policy_options: vec![RevisionApprovalPolicy::Never], authority_summary: "Writable only within the isolated Implementer execution workspace.".into() },
        hooks: vec![HarnessHookConfiguration { name: "completion".into(), status: HarnessHookStatus::NotConnected, detail: "No Implementer completion, review, or settlement hook is connected.".into() }], update_policy: HarnessUpdatePolicy::NotConfigured { reason: "Pinned revision is immutable.".into() },
    })
}

/// A later, distinct immutable revision for the reporting continuation.  The original
/// Implementer revision stays actionless; this configuration is never applied to it.
pub(crate) fn implementer_outcome_reporting_revision_configuration() -> Result<HarnessEffectiveConfiguration, String> {
    let mut configuration = initial_work_unit_implementer_revision_configuration()?;
    configuration.prompt_prefix.content = "You are the Work Unit Implementer reporting continuation for one completed isolated attempt. Use only submit_implementation_outcome and complete_implementation_outcome. Submit one ReviewPending summary and validation statement as claims. The application derives every identity and captures file evidence itself. Claims are not evidence, and tool success is not application acceptance or Handler review. Do not accept, review, return, retry, settle, activate dependents, or continue a Sprint or Epic.".into();
    configuration.tools.items = vec![
        HarnessToolConfiguration { name: "submit_implementation_outcome".into(), policy: HarnessToolPolicy::Available },
        HarnessToolConfiguration { name: "complete_implementation_outcome".into(), policy: HarnessToolPolicy::Available },
    ];
    configuration.tools.schema_boundary = "Only one context-bound ReviewPending claim submission and semantic completion are exposed; neither is evidence, application acceptance, or Handler review.".into();
    configuration.hooks = vec![HarnessHookConfiguration { name: "completion".into(), status: HarnessHookStatus::NotConnected, detail: "Application acceptance requires separate application-owned evidence and a matching Completed terminal lifecycle observation; Handler-review readiness is later still.".into() }];
    Ok(configuration)
}

pub(crate) fn profile_from_immutable_implementer_revision(configuration: &HarnessEffectiveConfiguration, revision_version: u16) -> Result<ConversationHarnessProfile, String> {
    let reporting_tools = ["submit_implementation_outcome", "complete_implementation_outcome"];
    let tools = configuration.tools.items.iter().map(|tool| tool.name.as_str()).collect::<Vec<_>>();
    if configuration.identity.machine_key != "work_unit_implementer"
        || configuration.tools.available_discovery_policy != HarnessDiscoveryPolicy::Whitelist
        || (!tools.is_empty() && tools.as_slice() != reporting_tools)
        || configuration.tools.items.iter().any(|tool| tool.policy != HarnessToolPolicy::Available)
        || configuration.runtime.sandbox != HarnessSandbox::WorkspaceWrite
        || configuration.runtime.approval_policy != RevisionApprovalPolicy::Never {
        return Err("immutable Implementer revision is outside the bounded Implementer contract".into());
    }
    let completion_criteria = if tools.is_empty() {
        "application_observed_implementer_ready_state"
    } else {
        "semantic_completion_and_application_observed_terminal_lifecycle"
    };
    Ok(ConversationHarnessProfile { key: configuration.identity.machine_key.clone(), version: revision_version, context: configuration.prompt_prefix.content.clone(), skill_guidance: configuration.skills.items.iter().map(|skill| SkillGuidance { canonical_name: skill.name.clone(), canonical_path: skill.path.clone(), purpose: skill.purpose.clone(), use_when: skill.use_when.clone() }).collect(), runtime: HarnessRuntime { model: configuration.runtime.default_model.clone(), reasoning_effort: configuration.runtime.default_reasoning.map(reasoning_to_catalog), sandbox: sandbox_from_revision(configuration.runtime.sandbox)?, approval_policy: HarnessApprovalPolicy::Never }, mcp: HarnessMcpExposure { required: !tools.is_empty(), enabled_tools: configuration.tools.items.iter().map(|tool| tool.name.clone()).collect() }, lifecycle: HarnessLifecycle { context_delivery: HarnessContextDelivery::FirstQuery, completion_criteria: vec![completion_criteria.into()] } })
}

fn reasoning_from_catalog(value: &str) -> Option<HarnessReasoningLevel> {
    match value {
        "low" => Some(HarnessReasoningLevel::Low),
        "medium" => Some(HarnessReasoningLevel::Medium),
        "high" => Some(HarnessReasoningLevel::High),
        "xhigh" => Some(HarnessReasoningLevel::Xhigh),
        _ => None,
    }
}
fn reasoning_to_catalog(value: HarnessReasoningLevel) -> String {
    match value {
        HarnessReasoningLevel::Low => "low",
        HarnessReasoningLevel::Medium => "medium",
        HarnessReasoningLevel::High => "high",
        HarnessReasoningLevel::Xhigh => "xhigh",
    }
    .into()
}
fn sandbox_to_revision(value: RuntimeSandboxMode) -> HarnessSandbox {
    match value {
        RuntimeSandboxMode::ReadOnly => HarnessSandbox::ReadOnly,
        RuntimeSandboxMode::WorkspaceWrite => HarnessSandbox::WorkspaceWrite,
        RuntimeSandboxMode::DangerFullAccess => HarnessSandbox::DangerFullAccess,
    }
}
fn sandbox_from_revision(value: HarnessSandbox) -> Result<RuntimeSandboxMode, String> {
    Ok(match value {
        HarnessSandbox::ReadOnly => RuntimeSandboxMode::ReadOnly,
        HarnessSandbox::WorkspaceWrite => RuntimeSandboxMode::WorkspaceWrite,
        HarnessSandbox::DangerFullAccess => RuntimeSandboxMode::DangerFullAccess,
    })
}

/// Resolve a durable Harness binding by its recorded identity, never by a newer current profile.
pub(crate) fn pinned_profile(
    key: &str,
    version: u16,
) -> Result<ConversationHarnessProfile, String> {
    let profile = profile_from_catalog(CATALOG_JSON, key)?;
    if profile.version != version {
        return Err(format!(
            "Conversation Harness configuration '{key}' revision {version} is unavailable"
        ));
    }
    Ok(profile)
}

/// Reopen an already-applied immutable Harness binding without consulting the current catalog.
pub(crate) fn pinned_profile_snapshot(
    key: &str,
    version: u16,
    snapshot: &str,
) -> Result<ConversationHarnessProfile, String> {
    let profile: ConversationHarnessProfile = serde_json::from_str(snapshot)
        .map_err(|error| format!("invalid persisted Conversation Harness snapshot: {error}"))?;
    validate_profile(&profile)?;
    if profile.key != key || profile.version != version {
        return Err(
            "persisted Conversation Harness snapshot does not match its pinned identity".into(),
        );
    }
    Ok(profile)
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
        ConversationHarnessRole::EpicRunnerEscalationReassessment => "epic-runner",
        ConversationHarnessRole::SprintRunner => "sprint-runner",
        ConversationHarnessRole::SprintRunnerPlanningControl => "sprint-runner",
        ConversationHarnessRole::SprintRunnerHandbackReassessment => "sprint-runner",
        ConversationHarnessRole::WorkSlicePlanner => "work-slice-planner",
        ConversationHarnessRole::WorkUnitHandler => "work-unit-handler",
        ConversationHarnessRole::WorkUnitImplementer => "work-unit-implementer",
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
    fn four_profiles_validate_with_truthful_enforceable_settings() {
        let plan_builder = profile(ConversationHarnessRole::EpicPlanBuilder).unwrap();
        let bootstrap = profile(ConversationHarnessRole::EpicBootstrapGenerator).unwrap();
        let runner = profile(ConversationHarnessRole::EpicRunner).unwrap();
        let epic_reassessment = profile(ConversationHarnessRole::EpicRunnerEscalationReassessment).unwrap();
        let sprint_runner = profile(ConversationHarnessRole::SprintRunner).unwrap();
        let planning_control =
            profile(ConversationHarnessRole::SprintRunnerPlanningControl).unwrap();
        let handback_reassessment =
            profile(ConversationHarnessRole::SprintRunnerHandbackReassessment).unwrap();
        let planner = profile(ConversationHarnessRole::WorkSlicePlanner).unwrap();
        let handler = profile(ConversationHarnessRole::WorkUnitHandler).unwrap();
        let implementer = profile(ConversationHarnessRole::WorkUnitImplementer).unwrap();

        assert_eq!(plan_builder.version, 4);
        assert_eq!(runner.key, "epic_runner");
        assert_eq!(runner.version, 3);
        assert_eq!(epic_reassessment.key, "epic_runner_escalation_reassessment");
        assert_eq!(epic_reassessment.version, 1);
        assert_eq!(epic_reassessment.mcp.enabled_tools, ["read_epic_escalation_reassessment_context"]);
        assert!(epic_reassessment.mcp.required);
        assert_eq!(sprint_runner.key, "sprint_runner");
        assert_eq!(sprint_runner.version, 2);
        assert_eq!(
            planning_control.mcp.enabled_tools,
            ["request_work_slice_planner"]
        );
        assert!(planning_control.mcp.required);
        assert_eq!(
            handback_reassessment.mcp.enabled_tools,
            ["read_sprint_handback_reassessment_context", "record_sprint_handback_disposition"]
        );
        assert!(handback_reassessment.mcp.required);
        assert_eq!(planner.key, "work_slice_planner");
        assert_eq!(handler.key, "work_unit_handler");
        assert_eq!(implementer.key, "work_unit_implementer");
        assert_eq!(handler.version, 2);
        assert_eq!(implementer.version, 2);
        assert_eq!(handler.mcp.enabled_tools, ["request_work_unit_implementer"]);
        assert!(handler.mcp.required);
        assert!(implementer.mcp.enabled_tools.is_empty());
        assert!(!implementer.mcp.required);
        assert_eq!(implementer.runtime_options().sandbox, Some(RuntimeSandboxMode::WorkspaceWrite));
        assert_eq!(
            planner.mcp.enabled_tools,
            [
                "read_current_planning_context",
                "submit_work_slice_proposal",
                "request_work_slice_refinement",
                "complete_work_slice_planning",
            ]
        );
        assert!(planner.mcp.required);
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
        assert_eq!(runner.mcp.enabled_tools, ["request_next_sprint_runner"]);
        assert!(runner.mcp.required);
        assert_eq!(sprint_runner.mcp.enabled_tools, Vec::<String>::new());
        assert!(!sprint_runner.mcp.required);
        assert_eq!(
            plan_builder.runtime_configuration_args(),
            ["-c", "approval_policy=\"never\""]
        );
        assert!(plan_builder.runtime_options().model.is_none());
        assert!(plan_builder.runtime.reasoning_effort.is_none());
    }

    #[test]
    fn handler_to_implementer_contract_has_one_action_and_scoped_write_runtime() {
        let handler = profile(ConversationHarnessRole::WorkUnitHandler).unwrap();
        let implementer = profile(ConversationHarnessRole::WorkUnitImplementer).unwrap();
        assert_eq!(handler.mcp.enabled_tools, ["request_work_unit_implementer"]);
        assert!(handler.mcp.required);
        assert!(implementer.mcp.enabled_tools.is_empty());
        assert_eq!(implementer.runtime.sandbox, RuntimeSandboxMode::WorkspaceWrite);
    }

    #[test]
    fn implementer_reporting_revision_is_distinct_and_historical_revision_stays_actionless() {
        let historical = initial_work_unit_implementer_revision_configuration().unwrap();
        let reporting = implementer_outcome_reporting_revision_configuration().unwrap();
        let historical_profile = profile_from_immutable_implementer_revision(&historical, 2).unwrap();
        let reporting_profile = profile_from_immutable_implementer_revision(&reporting, 3).unwrap();
        assert!(historical_profile.mcp.enabled_tools.is_empty());
        assert!(!historical_profile.mcp.required);
        assert_eq!(historical_profile.lifecycle.completion_criteria, ["application_observed_implementer_ready_state"]);
        assert_eq!(reporting_profile.mcp.enabled_tools, ["submit_implementation_outcome", "complete_implementation_outcome"]);
        assert!(reporting_profile.mcp.required);
        assert_eq!(reporting_profile.lifecycle.completion_criteria, ["semantic_completion_and_application_observed_terminal_lifecycle"]);
        assert!(reporting.prompt_prefix.content.contains("Claims are not evidence"));

        let mut wrong_policy = reporting.clone();
        wrong_policy.tools.items[0].policy = HarnessToolPolicy::InitialInvocation;
        assert!(profile_from_immutable_implementer_revision(&wrong_policy, 3).is_err());

        let mut wrong_discovery = reporting;
        wrong_discovery.tools.available_discovery_policy = HarnessDiscoveryPolicy::Blacklist;
        assert!(profile_from_immutable_implementer_revision(&wrong_discovery, 3).is_err());
    }

    #[test]
    fn old_immutable_handler_revision_remains_actionless() {
        let old = initial_work_unit_handler_baseline_revision_configuration().unwrap();
        assert!(!old.prompt_prefix.content.contains("Use only request_work_unit_implementer"));
        let reopened = profile_from_immutable_handler_revision(&old, 1).unwrap();
        assert!(reopened.mcp.enabled_tools.is_empty());
        assert!(!reopened.mcp.required);

        let mut unsafe_revision = initial_work_unit_handler_revision_configuration().unwrap();
        unsafe_revision.runtime.sandbox = HarnessSandbox::WorkspaceWrite;
        assert!(profile_from_immutable_handler_revision(&unsafe_revision, 2).is_err());
    }

    #[test]
    fn review_handler_revision_is_read_only_and_cannot_gain_execution_authority() {
        let historical = initial_work_unit_handler_baseline_revision_configuration().unwrap();
        let review = handler_outcome_review_revision_configuration().unwrap();
        let historical_profile = profile_from_immutable_handler_revision(&historical, 1).unwrap();
        let review_profile = profile_from_immutable_handler_revision(&review, 3).unwrap();
        assert!(historical_profile.mcp.enabled_tools.is_empty());
        assert_eq!(review_profile.mcp.enabled_tools, ["read_handler_review_evidence", "accept_implementation_outcome", "return_implementation_outcome"]);
        assert_eq!(review_profile.runtime_options().sandbox, Some(RuntimeSandboxMode::ReadOnly));
        assert!(!review_profile.mcp.enabled_tools.iter().any(|tool| tool == "request_work_unit_implementer"));
        let mut unsafe_revision = review;
        unsafe_revision.runtime.sandbox = HarnessSandbox::WorkspaceWrite;
        assert!(profile_from_immutable_handler_revision(&unsafe_revision, 3).is_err());
    }

    #[test]
    fn persisted_pinned_profile_reopens_without_current_catalog_and_denies_tampering() {
        let profile = profile(ConversationHarnessRole::WorkSlicePlanner).unwrap();
        let snapshot = serde_json::to_string(&profile).unwrap();
        assert_eq!(
            pinned_profile_snapshot(&profile.key, profile.version, &snapshot).unwrap(),
            profile
        );
        assert!(pinned_profile_snapshot("different", profile.version, &snapshot).is_err());
        assert!(pinned_profile_snapshot(&profile.key, profile.version + 1, &snapshot).is_err());
        assert!(pinned_profile_snapshot(&profile.key, profile.version, "{}").is_err());
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
            .join(".agents/product-skills/epic-plan-builder/SKILL.md")
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
