use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub(crate) const NATIVE_QUERY_VERSION: &str = "orchestration-native-query/v2";
const MAX_SUGGESTED_EPIC_NAME_BYTES: usize = 240;
const MAX_PROPOSED_SPRINTS: usize = 20;
const MAX_SPRINT_TITLE_BYTES: usize = 240;
const MAX_INTENDED_MOVEMENT_BYTES: usize = 4_000;
const MAX_CONCERN_SUMMARIES_PER_SPRINT: usize = 20;
const MAX_CONCERN_SUMMARY_BYTES: usize = 2_000;

macro_rules! id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
        pub(crate) struct $name(String);
        impl $name {
            pub(crate) fn new(value: impl Into<String>) -> Result<Self, String> {
                let value = value.into();
                if value.trim().is_empty() || value.len() > 200 {
                    return Err(concat!(
                        stringify!($name),
                        " must be a non-empty bounded identifier"
                    )
                    .into());
                }
                Ok(Self(value))
            }
            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

id!(EpicPlanningDraftId);
id!(CapabilityProfileId);
id!(PlanningDraftAgentSessionAssociationId);
id!(ProposalRevisionId);
id!(ProposalCommandId);
id!(ProposalResultId);
id!(ProposalEventId);
id!(EffectProvenanceId);
id!(EpicInitiationCommandId);
id!(EpicInitiationResultId);
id!(EpicInitiationEventId);
id!(EpicInitiationId);
id!(EpicId);

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlanBuilderProposal {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) suggested_epic_name: Option<String>,
    #[schemars(length(min = 1, max = 20))]
    pub(crate) sprints: Vec<ProposedSprint>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProposedSprint {
    pub(crate) title: String,
    pub(crate) intended_movement: String,
    #[schemars(length(max = 20))]
    pub(crate) concern_summaries: Vec<String>,
}

impl PlanBuilderProposal {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if let Some(name) = &self.suggested_epic_name {
            validate_text(
                name,
                MAX_SUGGESTED_EPIC_NAME_BYTES,
                "proposal.suggestedEpicName",
            )?;
        }
        if self.sprints.is_empty() || self.sprints.len() > MAX_PROPOSED_SPRINTS {
            return Err("proposal.sprints must contain 1..20 ordered Sprints".into());
        }
        for (sprint_index, sprint) in self.sprints.iter().enumerate() {
            validate_text(
                &sprint.title,
                MAX_SPRINT_TITLE_BYTES,
                &format!("proposal.sprints[{sprint_index}].title"),
            )?;
            validate_text(
                &sprint.intended_movement,
                MAX_INTENDED_MOVEMENT_BYTES,
                &format!("proposal.sprints[{sprint_index}].intendedMovement"),
            )?;
            if sprint.concern_summaries.len() > MAX_CONCERN_SUMMARIES_PER_SPRINT {
                return Err(format!(
                    "proposal.sprints[{sprint_index}].concernSummaries must contain at most 20 entries"
                ));
            }
            for (summary_index, summary) in sprint.concern_summaries.iter().enumerate() {
                validate_text(
                    summary,
                    MAX_CONCERN_SUMMARY_BYTES,
                    &format!("proposal.sprints[{sprint_index}].concernSummaries[{summary_index}]"),
                )?;
            }
        }
        Ok(())
    }
}

fn validate_text(value: &str, maximum_bytes: usize, name: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > maximum_bytes {
        return Err(format!(
            "{name} must be non-empty and within its size limit"
        ));
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SaveEpicPlanProposalCommand {
    pub(crate) epic_planning_draft_id: EpicPlanningDraftId,
    pub(crate) capability_profile_id: CapabilityProfileId,
    pub(crate) agent_session_association_id: PlanningDraftAgentSessionAssociationId,
    pub(crate) agent_session_id: String,
    pub(crate) actor_id: String,
    pub(crate) expected_revision: Option<String>,
    pub(crate) proposal: PlanBuilderProposal,
    pub(crate) idempotency_key: String,
}

impl SaveEpicPlanProposalCommand {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.actor_id.trim().is_empty() || self.actor_id.len() > 200 {
            return Err("actor_id must be a non-empty bounded identifier".into());
        }
        if self.agent_session_id.trim().is_empty() || self.agent_session_id.len() > 200 {
            return Err("agent_session_id must be a non-empty bounded identifier".into());
        }
        if self.idempotency_key.trim().is_empty() || self.idempotency_key.len() > 200 {
            return Err("idempotency_key must be a non-empty bounded identifier".into());
        }
        self.proposal.validate()?;
        if self
            .expected_revision
            .as_ref()
            .is_some_and(|value| value.trim().is_empty() || value.len() > 200)
        {
            return Err("expected_revision must be an opaque bounded token".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SaveProposalResult {
    pub(crate) command_id: ProposalCommandId,
    pub(crate) result_id: ProposalResultId,
    pub(crate) revision_id: ProposalRevisionId,
    pub(crate) revision_token: String,
    pub(crate) event_id: ProposalEventId,
    pub(crate) provenance_id: EffectProvenanceId,
    /// True only when this response reused the original result of the same authorized command.
    pub(crate) idempotent_replay: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SaveProposalError {
    InvalidInput(String),
    Forbidden,
    DraftNotFound,
    RevisionConflict,
    IdempotencyConflict,
    Unavailable(String),
}

impl std::fmt::Display for SaveProposalError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(message) | Self::Unavailable(message) => {
                formatter.write_str(message)
            }
            Self::Forbidden => formatter.write_str(
                "the profile is not authorized for this planning draft and Agent Session",
            ),
            Self::DraftNotFound => formatter.write_str("Epic Planning Draft not found"),
            Self::RevisionConflict => formatter.write_str("expected proposal revision is stale"),
            Self::IdempotencyConflict => {
                formatter.write_str("idempotency key was used for different proposal semantics")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InitiateEpicCommand {
    pub(crate) epic_planning_draft_id: EpicPlanningDraftId,
    pub(crate) expected_revision_token: String,
    pub(crate) actor_id: String,
    pub(crate) idempotency_key: String,
    pub(crate) root_branch: Option<String>,
}
impl InitiateEpicCommand {
    pub(crate) fn validate(&self) -> Result<(), InitiateEpicError> {
        if self.expected_revision_token.trim().is_empty()
            || self.expected_revision_token.len() > 200
            || self.actor_id.trim().is_empty()
            || self.actor_id.len() > 200
            || self.idempotency_key.trim().is_empty()
            || self.idempotency_key.len() > 200
        {
            return Err(InitiateEpicError::InvalidInput(
                "initiation identifiers must be non-empty and bounded".into(),
            ));
        }
        if let Some(root_branch) = &self.root_branch {
            if !valid_root_branch(root_branch) {
                return Err(InitiateEpicError::InvalidInput(
                    "Epic root branch is invalid".into(),
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn valid_root_branch(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 240
        && value != "@"
        && !value.starts_with('-')
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.ends_with('.')
        && !value.contains("..")
        && !value.contains("//")
        && !value.contains("@{")
        && value.split('/').all(|component| {
            !component.is_empty() && !component.starts_with('.') && !component.ends_with(".lock")
        })
        && value.chars().all(|character| {
            !character.is_control()
                && !matches!(character, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
        })
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitiateEpicResult {
    pub(crate) initiation_id: EpicInitiationId,
    pub(crate) epic_id: EpicId,
    pub(crate) proposal_revision_id: ProposalRevisionId,
    pub(crate) material_snapshot_hash: String,
    pub(crate) idempotent_replay: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum InitiateEpicError {
    InvalidInput(String),
    DraftNotFound,
    ProposalMissing,
    RevisionConflict,
    Forbidden,
    Canceled,
    AlreadyInitiated,
    IdempotencyConflict,
    Unavailable(String),
}
impl std::fmt::Display for InitiateEpicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(x) | Self::Unavailable(x) => f.write_str(x),
            Self::DraftNotFound => {
                f.write_str("Epic Planning Draft is not active or does not exist")
            }
            Self::ProposalMissing => f.write_str("Epic Planning Draft has no current proposal"),
            Self::RevisionConflict => f.write_str("expected proposal revision is stale"),
            Self::Forbidden => {
                f.write_str("the application actor is not authorized to initiate this draft")
            }
            Self::Canceled => {
                f.write_str("Epic Planning Draft was canceled and cannot be initiated")
            }
            Self::AlreadyInitiated => f.write_str("Epic Planning Draft has already been initiated"),
            Self::IdempotencyConflict => {
                f.write_str("idempotency key was used for different initiation semantics")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::valid_root_branch;

    #[test]
    fn root_branch_uses_git_ref_name_constraints() {
        for name in [
            "main",
            "codex/epic-workflow-ux-test",
            "feature/with.unicode-ø",
        ] {
            assert!(valid_root_branch(name), "{name} should be accepted");
        }
        for name in [
            "",
            "-main",
            "/main",
            "main/",
            "main..next",
            "main//next",
            "main@{1}",
            "@",
            ".hidden",
            "topic/.hidden",
            "topic.lock",
            "topic/name.lock",
            "topic.",
            "topic name",
            "topic~old",
            "topic:old",
            "topic?old",
            "topic*old",
            "topic[old",
            "topic\\old",
        ] {
            assert!(!valid_root_branch(name), "{name} should be rejected");
        }
    }
}
