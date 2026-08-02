//! Product-owned persistence for complete Conversation Harness draft working copies.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::Path};

pub(crate) const HARNESS_EFFECTIVE_CONFIGURATION_V1: &str = "harness-effective-configuration/v1";

pub(crate) const HARNESS_WORKING_COPY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS harness_working_copies (
  harness_key TEXT PRIMARY KEY,
  configuration_contract_version TEXT NOT NULL CHECK (configuration_contract_version = 'harness-effective-configuration/v1'),
  configuration_json TEXT NOT NULL CHECK (json_valid(configuration_json)),
  working_copy_digest TEXT NOT NULL,
  draft_revision INTEGER NOT NULL CHECK (draft_revision > 0),
  dirty INTEGER NOT NULL CHECK (dirty = 1),
  editor_kind TEXT NOT NULL CHECK (editor_kind IN ('application_user', 'agent_session')),
  editor_reference TEXT NOT NULL,
  saved_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS harness_working_copy_commands (
  idempotency_key TEXT PRIMARY KEY,
  payload_fingerprint TEXT NOT NULL,
  harness_key TEXT NOT NULL,
  expected_current_revision INTEGER NOT NULL CHECK (expected_current_revision >= 0),
  result_revision INTEGER NOT NULL CHECK (result_revision > 0),
  result_json TEXT NOT NULL CHECK (json_valid(result_json)),
  result_digest TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (harness_key) REFERENCES harness_working_copies(harness_key) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS harness_working_copy_commands_by_harness
  ON harness_working_copy_commands(harness_key, result_revision);
"#;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessEffectiveConfigurationEnvelope {
    pub(crate) contract_version: String,
    pub(crate) configuration: HarnessEffectiveConfiguration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessEffectiveConfiguration {
    pub(crate) identity: HarnessIdentityConfiguration,
    pub(crate) prompt_prefix: HarnessPromptPrefixConfiguration,
    pub(crate) skills: HarnessSkillsConfiguration,
    pub(crate) tools: HarnessToolsConfiguration,
    pub(crate) runtime: HarnessRuntimeConfiguration,
    pub(crate) hooks: Vec<HarnessHookConfiguration>,
    pub(crate) update_policy: HarnessUpdatePolicy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessIdentityConfiguration {
    pub(crate) name: String,
    pub(crate) machine_key: String,
    pub(crate) permitted_agent_names: Option<Vec<String>>,
    pub(crate) visual_identity: Option<HarnessVisualIdentity>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessVisualIdentity {
    pub(crate) token: String,
    pub(crate) accent: String,
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessSkillsConfiguration {
    pub(crate) available_discovery_policy: HarnessDiscoveryPolicy,
    pub(crate) items: Vec<HarnessSkillConfiguration>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessDiscoveryPolicy {
    Whitelist,
    Blacklist,
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
pub(crate) struct HarnessRuntimeConfiguration {
    pub(crate) model_policy_mode: HarnessModelPolicyMode,
    pub(crate) models: Vec<HarnessModelConstraint>,
    pub(crate) default_model: Option<String>,
    pub(crate) default_reasoning: Option<HarnessReasoningLevel>,
    pub(crate) sandbox: HarnessSandbox,
    pub(crate) sandbox_options: Vec<HarnessSandbox>,
    pub(crate) approval_policy: HarnessApprovalPolicy,
    pub(crate) approval_policy_options: Vec<HarnessApprovalPolicy>,
    pub(crate) authority_summary: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessModelPolicyMode {
    RevisionOwned,
    DelegatedShared,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessModelConstraint {
    pub(crate) model_id: String,
    pub(crate) allowed: bool,
    pub(crate) min_reasoning: HarnessReasoningLevel,
    pub(crate) max_reasoning: HarnessReasoningLevel,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessReasoningLevel {
    Low,
    Medium,
    High,
    Xhigh,
}

impl HarnessReasoningLevel {
    fn ordinal(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Xhigh => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessSandbox {
    ReadOnly,
    WorkspaceWrite,
    DangerFullAccess,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessEditorKind {
    ApplicationUser,
    AgentSession,
}

impl HarnessEditorKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ApplicationUser => "application_user",
            Self::AgentSession => "agent_session",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "application_user" => Some(Self::ApplicationUser),
            "agent_session" => Some(Self::AgentSession),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessWorkingCopyEditor {
    pub(crate) kind: HarnessEditorKind,
    pub(crate) reference: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveHarnessWorkingCopyCommand {
    pub(crate) harness_key: String,
    pub(crate) configuration: HarnessEffectiveConfiguration,
    pub(crate) expected_current_revision: u64,
    pub(crate) editor: HarnessWorkingCopyEditor,
    pub(crate) idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessWorkingCopy {
    pub(crate) harness_key: String,
    pub(crate) configuration: HarnessEffectiveConfiguration,
    pub(crate) draft_revision: u64,
    pub(crate) dirty: bool,
    pub(crate) editor: HarnessWorkingCopyEditor,
    pub(crate) saved_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SaveHarnessWorkingCopyResult {
    Stored(HarnessWorkingCopy),
    IdempotentReplay(HarnessWorkingCopy),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum HarnessWorkingCopyError {
    Invalid,
    Conflict,
    InvalidStoredState,
    Unavailable,
}

pub(crate) fn validate_command(
    command: &SaveHarnessWorkingCopyCommand,
) -> Result<(), HarnessWorkingCopyError> {
    if !valid_harness_key(&command.harness_key)
        || command.configuration.identity.machine_key != command.harness_key
        || !required_text(&command.editor.reference, 240)
        || !bounded_token(&command.idempotency_key, 240)
        || command.expected_current_revision >= i64::MAX as u64
    {
        return Err(HarnessWorkingCopyError::Invalid);
    }
    validate_draft_configuration(&command.configuration)
}

pub(crate) fn validate_working_copy(
    working_copy: &HarnessWorkingCopy,
) -> Result<(), HarnessWorkingCopyError> {
    if !valid_harness_key(&working_copy.harness_key)
        || working_copy.configuration.identity.machine_key != working_copy.harness_key
        || working_copy.draft_revision == 0
        || working_copy.draft_revision > i64::MAX as u64
        || !working_copy.dirty
        || !required_text(&working_copy.editor.reference, 240)
    {
        return Err(HarnessWorkingCopyError::InvalidStoredState);
    }
    validate_draft_configuration(&working_copy.configuration)
        .map_err(|_| HarnessWorkingCopyError::InvalidStoredState)
}

pub(crate) fn validate_harness_key(value: &str) -> Result<(), HarnessWorkingCopyError> {
    valid_harness_key(value)
        .then_some(())
        .ok_or(HarnessWorkingCopyError::Invalid)
}

pub(crate) fn validate_draft_configuration(
    configuration: &HarnessEffectiveConfiguration,
) -> Result<(), HarnessWorkingCopyError> {
    if !draft_text(&configuration.identity.name, 120)
        || !valid_harness_key(&configuration.identity.machine_key)
        || !draft_text(&configuration.prompt_prefix.content, 200_000)
        || !draft_text(&configuration.tools.schema_boundary, 4_000)
        || !draft_text(&configuration.runtime.authority_summary, 4_000)
    {
        return Err(HarnessWorkingCopyError::Invalid);
    }
    if let Some(names) = &configuration.identity.permitted_agent_names {
        if !unique_bounded_text(names, 120) {
            return Err(HarnessWorkingCopyError::Invalid);
        }
    }
    if let Some(visual) = &configuration.identity.visual_identity {
        if !bounded_token(&visual.token, 120) || !draft_text(&visual.accent, 120) {
            return Err(HarnessWorkingCopyError::Invalid);
        }
    }
    if configuration.skills.items.len() > 256
        || configuration.tools.items.len() > 256
        || configuration.runtime.models.len() > 128
        || configuration.hooks.len() > 128
        || configuration.runtime.sandbox_options.is_empty()
        || configuration.runtime.approval_policy_options != [HarnessApprovalPolicy::Never]
        || !configuration
            .runtime
            .sandbox_options
            .contains(&configuration.runtime.sandbox)
        || configuration
            .runtime
            .sandbox_options
            .iter()
            .copied()
            .collect::<HashSet<_>>()
            .len()
            != configuration.runtime.sandbox_options.len()
    {
        return Err(HarnessWorkingCopyError::Invalid);
    }
    let mut skill_names = HashSet::new();
    for skill in &configuration.skills.items {
        let path = Path::new(&skill.path);
        if !bounded_token(&skill.name, 160)
            || !skill_names.insert(skill.name.as_str())
            || !required_text(&skill.path, 500)
            || !path.is_relative()
            || path.components().any(|part| {
                matches!(
                    part,
                    std::path::Component::ParentDir
                        | std::path::Component::RootDir
                        | std::path::Component::Prefix(_)
                )
            })
            || !draft_text(&skill.purpose, 4_000)
            || !draft_text(&skill.use_when, 4_000)
        {
            return Err(HarnessWorkingCopyError::Invalid);
        }
    }
    let mut tool_names = HashSet::new();
    if configuration
        .tools
        .items
        .iter()
        .any(|tool| !bounded_token(&tool.name, 160) || !tool_names.insert(tool.name.as_str()))
    {
        return Err(HarnessWorkingCopyError::Invalid);
    }
    let mut model_names = HashSet::new();
    for model in &configuration.runtime.models {
        if !bounded_token(&model.model_id, 160)
            || !model_names.insert(model.model_id.as_str())
            || model.min_reasoning.ordinal() > model.max_reasoning.ordinal()
        {
            return Err(HarnessWorkingCopyError::Invalid);
        }
    }
    match (
        configuration.runtime.default_model.as_deref(),
        configuration.runtime.default_reasoning,
    ) {
        (None, None) => {}
        (Some(model), reasoning) => {
            let Some(constraint) = configuration
                .runtime
                .models
                .iter()
                .find(|candidate| candidate.model_id == model && candidate.allowed)
            else {
                return Err(HarnessWorkingCopyError::Invalid);
            };
            if reasoning.is_some_and(|level| {
                level.ordinal() < constraint.min_reasoning.ordinal()
                    || level.ordinal() > constraint.max_reasoning.ordinal()
            }) {
                return Err(HarnessWorkingCopyError::Invalid);
            }
        }
        (None, Some(_)) => return Err(HarnessWorkingCopyError::Invalid),
    }
    let mut hook_names = HashSet::new();
    if configuration.hooks.iter().any(|hook| {
        !bounded_token(&hook.name, 160)
            || !hook_names.insert(hook.name.as_str())
            || !draft_text(&hook.detail, 4_000)
    }) {
        return Err(HarnessWorkingCopyError::Invalid);
    }
    if let HarnessUpdatePolicy::NotConfigured { reason } = &configuration.update_policy {
        if !draft_text(reason, 4_000) {
            return Err(HarnessWorkingCopyError::Invalid);
        }
    }
    Ok(())
}

/// Publication requires every intentionally draftable semantic text field to be complete. Draft
/// saves continue to use `validate_draft_configuration` and may preserve empty in-progress text.
pub(crate) fn validate_complete_configuration(
    configuration: &HarnessEffectiveConfiguration,
) -> Result<(), HarnessWorkingCopyError> {
    validate_draft_configuration(configuration)?;
    if !required_text(&configuration.identity.name, 120)
        || !required_text(&configuration.prompt_prefix.content, 200_000)
        || !required_text(&configuration.tools.schema_boundary, 4_000)
        || !required_text(&configuration.runtime.authority_summary, 4_000)
    {
        return Err(HarnessWorkingCopyError::Invalid);
    }
    if configuration
        .identity
        .visual_identity
        .as_ref()
        .is_some_and(|visual| !required_text(&visual.accent, 120))
        || configuration.skills.items.iter().any(|skill| {
            !required_text(&skill.purpose, 4_000) || !required_text(&skill.use_when, 4_000)
        })
        || configuration
            .hooks
            .iter()
            .any(|hook| !required_text(&hook.detail, 4_000))
        || matches!(
            &configuration.update_policy,
            HarnessUpdatePolicy::NotConfigured { reason } if !required_text(reason, 4_000)
        )
    {
        return Err(HarnessWorkingCopyError::Invalid);
    }
    Ok(())
}

fn valid_harness_key(value: &str) -> bool {
    value.len() <= 120
        && value.bytes().enumerate().all(|(index, byte)| match byte {
            b'a'..=b'z' => true,
            b'0'..=b'9' | b'_' | b'-' => index > 0,
            _ => false,
        })
}

fn draft_text(value: &str, maximum: usize) -> bool {
    value.len() <= maximum
        && value
            .chars()
            .all(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
}

fn required_text(value: &str, maximum: usize) -> bool {
    draft_text(value, maximum) && !value.trim().is_empty()
}

fn bounded_token(value: &str, maximum: usize) -> bool {
    required_text(value, maximum) && value.trim() == value
}

fn unique_bounded_text(values: &[String], maximum: usize) -> bool {
    let mut unique = HashSet::new();
    values
        .iter()
        .all(|value| required_text(value, maximum) && unique.insert(value.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{
        application::OrchestrationApplication,
        repository::{OrchestrationClock, SqliteOrchestrationRepository},
    };
    use chrono::TimeZone;
    use rusqlite::Connection;
    use std::{path::Path, sync::Arc};

    struct FixedClock(DateTime<Utc>);
    impl OrchestrationClock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn fixed_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 1, 10, 30, 0)
            .single()
            .unwrap()
    }

    fn open_repository(path: &Path) -> Arc<SqliteOrchestrationRepository> {
        let connection = Connection::open(path).unwrap();
        crate::storage::configure_sqlite_connection(&connection).unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        Arc::new(
            SqliteOrchestrationRepository::new_with_clock(
                connection,
                Arc::new(FixedClock(fixed_time())),
            )
            .unwrap(),
        )
    }

    fn configuration(key: &str) -> HarnessEffectiveConfiguration {
        HarnessEffectiveConfiguration {
            identity: HarnessIdentityConfiguration {
                name: "Epic Plan Builder".into(),
                machine_key: key.into(),
                permitted_agent_names: Some(vec!["Avery".into(), "Morgan".into()]),
                visual_identity: Some(HarnessVisualIdentity {
                    token: "sunflower".into(),
                    accent: "gold".into(),
                }),
            },
            prompt_prefix: HarnessPromptPrefixConfiguration {
                content: "Build the Epic plan from the accepted discussion.".into(),
                initial_delivery: HarnessInitialDelivery::Prepend,
                context_compression_delivery: HarnessContextCompressionDelivery::Deferred,
            },
            skills: HarnessSkillsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: vec![HarnessSkillConfiguration {
                    name: "epic-plan-builder".into(),
                    path: ".agents/skills/epic-plan-builder/SKILL.md".into(),
                    purpose: "Build one Epic proposal.".into(),
                    use_when: "The product owns an Epic planning Session.".into(),
                    policy: HarnessSkillPolicy::AlwaysApplicable,
                }],
            },
            tools: HarnessToolsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: vec![HarnessToolConfiguration {
                    name: "submit_epic_plan_proposal".into(),
                    policy: HarnessToolPolicy::Available,
                }],
                schema_boundary: "Application-owned proposal schema.".into(),
            },
            runtime: HarnessRuntimeConfiguration {
                model_policy_mode: HarnessModelPolicyMode::RevisionOwned,
                models: vec![HarnessModelConstraint {
                    model_id: "gpt-5.6-terra".into(),
                    allowed: true,
                    min_reasoning: HarnessReasoningLevel::Medium,
                    max_reasoning: HarnessReasoningLevel::High,
                }],
                default_model: Some("gpt-5.6-terra".into()),
                default_reasoning: Some(HarnessReasoningLevel::Medium),
                sandbox: HarnessSandbox::ReadOnly,
                sandbox_options: vec![
                    HarnessSandbox::ReadOnly,
                    HarnessSandbox::WorkspaceWrite,
                    HarnessSandbox::DangerFullAccess,
                ],
                approval_policy: HarnessApprovalPolicy::Never,
                approval_policy_options: vec![HarnessApprovalPolicy::Never],
                authority_summary: "Plan discussion and proposal submission only.".into(),
            },
            hooks: vec![HarnessHookConfiguration {
                name: "completion".into(),
                status: HarnessHookStatus::NotConnected,
                detail: "No typed completion hook is connected.".into(),
            }],
            update_policy: HarnessUpdatePolicy::NotConfigured {
                reason: "Runtime updates are deferred.".into(),
            },
        }
    }

    fn command(key: &str, expected: u64, idempotency: &str) -> SaveHarnessWorkingCopyCommand {
        SaveHarnessWorkingCopyCommand {
            harness_key: key.into(),
            configuration: configuration(key),
            expected_current_revision: expected,
            editor: HarnessWorkingCopyEditor {
                kind: HarnessEditorKind::ApplicationUser,
                reference: "local-user".into(),
            },
            idempotency_key: idempotency.into(),
        }
    }

    #[test]
    fn application_saves_whole_draft_with_optimistic_revision_and_exact_replay() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let repository = open_repository(&path);
        let application = OrchestrationApplication::new(repository.clone());

        let first_command = command("epic_plan_builder", 0, "save-1");
        let first = application
            .save_harness_working_copy(first_command.clone())
            .unwrap();
        let SaveHarnessWorkingCopyResult::Stored(first) = first else {
            panic!("first save must store");
        };
        assert_eq!(first.draft_revision, 1);
        assert!(first.dirty);
        assert_eq!(first.saved_at, fixed_time());

        assert_eq!(
            application
                .save_harness_working_copy(first_command.clone())
                .unwrap(),
            SaveHarnessWorkingCopyResult::IdempotentReplay(first.clone())
        );
        let mut differing_replay = first_command.clone();
        differing_replay.configuration.prompt_prefix.content = "Different".into();
        assert_eq!(
            application.save_harness_working_copy(differing_replay),
            Err(HarnessWorkingCopyError::Conflict)
        );
        assert_eq!(
            application.save_harness_working_copy(command("epic_plan_builder", 0, "stale-save")),
            Err(HarnessWorkingCopyError::Conflict)
        );

        let second = application
            .save_harness_working_copy(command("epic_plan_builder", 1, "save-2"))
            .unwrap();
        let SaveHarnessWorkingCopyResult::Stored(second) = second else {
            panic!("second save must store");
        };
        assert_eq!(second.draft_revision, 2);
        assert_eq!(
            application
                .save_harness_working_copy(first_command)
                .unwrap(),
            SaveHarnessWorkingCopyResult::IdempotentReplay(first)
        );
        assert_eq!(
            application
                .load_harness_working_copy("epic_plan_builder")
                .unwrap(),
            Some(second)
        );
    }

    #[test]
    fn effective_configuration_envelope_is_lossless_and_denies_unknown_json() {
        let envelope = HarnessEffectiveConfigurationEnvelope {
            contract_version: HARNESS_EFFECTIVE_CONFIGURATION_V1.into(),
            configuration: configuration("epic_plan_builder"),
        };
        let json = serde_json::to_value(&envelope).unwrap();
        assert_eq!(
            serde_json::from_value::<HarnessEffectiveConfigurationEnvelope>(json.clone()).unwrap(),
            envelope
        );
        let mut unknown = json;
        unknown
            .as_object_mut()
            .unwrap()
            .insert("runtimeApplied".into(), serde_json::Value::Bool(true));
        assert!(serde_json::from_value::<HarnessEffectiveConfigurationEnvelope>(unknown).is_err());
    }

    #[test]
    fn partial_draft_saves_loads_and_replays_without_coercion() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let repository = open_repository(&path);
        let application = OrchestrationApplication::new(repository);
        let mut partial = command("epic_plan_builder", 0, "partial-save");
        partial.configuration.identity.name.clear();
        partial.configuration.prompt_prefix.content.clear();
        partial
            .configuration
            .identity
            .visual_identity
            .as_mut()
            .unwrap()
            .accent
            .clear();
        partial.configuration.skills.items[0].purpose.clear();
        partial.configuration.skills.items[0].use_when.clear();
        partial.configuration.tools.schema_boundary.clear();
        partial.configuration.runtime.authority_summary.clear();
        partial.configuration.hooks[0].detail.clear();
        partial.configuration.update_policy = HarnessUpdatePolicy::NotConfigured {
            reason: String::new(),
        };

        let stored = application
            .save_harness_working_copy(partial.clone())
            .unwrap();
        let SaveHarnessWorkingCopyResult::Stored(stored) = stored else {
            unreachable!()
        };
        assert_eq!(stored.configuration, partial.configuration);
        assert_eq!(
            application
                .load_harness_working_copy("epic_plan_builder")
                .unwrap(),
            Some(stored.clone())
        );
        assert_eq!(
            application.save_harness_working_copy(partial).unwrap(),
            SaveHarnessWorkingCopyResult::IdempotentReplay(stored)
        );
    }

    #[test]
    fn malformed_or_unsafe_command_structure_fails_without_mutation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let repository = open_repository(&path);
        let application = OrchestrationApplication::new(repository);

        let mut cases = Vec::new();
        let mut invalid_key = command("Invalid Key", 0, "invalid-key");
        invalid_key.configuration.identity.machine_key = "Invalid Key".into();
        cases.push(invalid_key);
        let mut invalid_editor = command("epic_plan_builder", 0, "invalid-editor");
        invalid_editor.editor.reference = " ".into();
        cases.push(invalid_editor);
        let mut invalid_revision = command("epic_plan_builder", i64::MAX as u64, "invalid-rev");
        invalid_revision.configuration.identity.machine_key = "epic_plan_builder".into();
        cases.push(invalid_revision);
        let mut invalid_model = command("epic_plan_builder", 0, "invalid-model");
        invalid_model.configuration.runtime.models[0].min_reasoning = HarnessReasoningLevel::High;
        invalid_model.configuration.runtime.models[0].max_reasoning = HarnessReasoningLevel::Low;
        cases.push(invalid_model);
        let mut invalid_path = command("epic_plan_builder", 0, "invalid-path");
        invalid_path.configuration.skills.items[0].path = "../outside/SKILL.md".into();
        cases.push(invalid_path);
        let mut invalid_control = command("epic_plan_builder", 0, "invalid-control");
        invalid_control.configuration.prompt_prefix.content = "unsafe\u{0007}control".into();
        cases.push(invalid_control);

        for candidate in cases {
            assert_eq!(
                application.save_harness_working_copy(candidate),
                Err(HarnessWorkingCopyError::Invalid)
            );
        }
        assert_eq!(
            application
                .load_harness_working_copy("epic_plan_builder")
                .unwrap(),
            None
        );
    }

    #[test]
    fn command_write_is_atomic_when_the_idempotency_ledger_rejects_insert() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let repository = open_repository(&path);
        repository
            .save_harness_working_copy(command("epic_plan_builder", 0, "save-1"))
            .unwrap();
        drop(repository);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch("CREATE TRIGGER reject_harness_command BEFORE INSERT ON harness_working_copy_commands BEGIN SELECT RAISE(ABORT, 'test rollback'); END;")
            .unwrap();
        drop(connection);

        let repository = open_repository(&path);
        assert_eq!(
            repository.save_harness_working_copy(command("epic_plan_builder", 1, "save-rejected")),
            Err(HarnessWorkingCopyError::Unavailable)
        );
        assert_eq!(
            repository
                .load_harness_working_copy("epic_plan_builder")
                .unwrap()
                .unwrap()
                .draft_revision,
            1
        );
    }

    #[test]
    fn exact_replay_rejects_tampered_result_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let repository = open_repository(&path);
        let original = command("epic_plan_builder", 0, "save-1");
        repository
            .save_harness_working_copy(original.clone())
            .unwrap();
        drop(repository);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE harness_working_copy_commands SET result_json='{}' WHERE idempotency_key='save-1'",
                [],
            )
            .unwrap();
        drop(connection);

        let repository = open_repository(&path);
        assert_eq!(
            repository.save_harness_working_copy(original),
            Err(HarnessWorkingCopyError::InvalidStoredState)
        );
    }

    #[test]
    fn private_read_reopens_and_rejects_any_tampered_envelope_field() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let repository = open_repository(&path);
        let stored = repository
            .save_harness_working_copy(command("epic_plan_builder", 0, "save-1"))
            .unwrap();
        let SaveHarnessWorkingCopyResult::Stored(stored) = stored else {
            unreachable!()
        };
        drop(repository);
        let reopened = open_repository(&path);
        assert_eq!(
            reopened
                .load_harness_working_copy("epic_plan_builder")
                .unwrap(),
            Some(stored)
        );
        drop(reopened);

        for statement in [
            "UPDATE harness_working_copies SET configuration_json='{}'",
            "UPDATE harness_working_copies SET working_copy_digest='0000000000000000000000000000000000000000000000000000000000000000'",
            "PRAGMA ignore_check_constraints=ON; UPDATE harness_working_copies SET configuration_contract_version='harness-effective-configuration/v2'; PRAGMA ignore_check_constraints=OFF",
            "UPDATE harness_working_copies SET draft_revision=2",
            "PRAGMA ignore_check_constraints=ON; UPDATE harness_working_copies SET draft_revision=0; PRAGMA ignore_check_constraints=OFF",
            "PRAGMA ignore_check_constraints=ON; UPDATE harness_working_copies SET dirty=0; PRAGMA ignore_check_constraints=OFF",
            "UPDATE harness_working_copies SET editor_kind='agent_session'",
            "UPDATE harness_working_copies SET editor_reference='another-valid-editor'",
            "UPDATE harness_working_copies SET saved_at='2026-08-01T10:31:00.000Z'",
            "UPDATE harness_working_copies SET saved_at='not-a-time'",
        ] {
            let connection = Connection::open(&path).unwrap();
            connection.execute_batch("DELETE FROM harness_working_copy_commands; DELETE FROM harness_working_copies;").unwrap();
            drop(connection);
            let repository = open_repository(&path);
            repository
                .save_harness_working_copy(command("epic_plan_builder", 0, "tamper-seed"))
                .unwrap();
            drop(repository);
            let connection = Connection::open(&path).unwrap();
            connection.execute_batch(statement).unwrap();
            drop(connection);
            let repository = open_repository(&path);
            assert_eq!(
                repository.load_harness_working_copy("epic_plan_builder"),
                Err(HarnessWorkingCopyError::InvalidStoredState)
            );
            drop(repository);
        }
    }
}
