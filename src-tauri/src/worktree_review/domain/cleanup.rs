use super::{
    ArtifactSetId, ArtifactStorageKey, CleanupJobId, CleanupResourceId, ContainmentRoot,
    DomainError, OperationAttemptId, ResourceLocator, ReviewBuildId, WorkspaceId,
    WorkspaceOwnership,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CleanupTrigger {
    RetentionPolicy,
    ManualRequest,
    Reconciliation,
    LegacyMigration,
}

impl CleanupTrigger {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::RetentionPolicy => "retention_policy",
            Self::ManualRequest => "manual_request",
            Self::Reconciliation => "reconciliation",
            Self::LegacyMigration => "legacy_migration",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "retention_policy" => Some(Self::RetentionPolicy),
            "manual_request" => Some(Self::ManualRequest),
            "reconciliation" => Some(Self::Reconciliation),
            "legacy_migration" => Some(Self::LegacyMigration),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CleanupEligibility {
    Eligible,
    BuildRunning,
    RetentionProtected,
    BorrowedResource,
    AlreadyCleaned,
    SourceUnverified,
}

impl CleanupEligibility {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Eligible => "eligible",
            Self::BuildRunning => "build_running",
            Self::RetentionProtected => "retention_protected",
            Self::BorrowedResource => "borrowed_resource",
            Self::AlreadyCleaned => "already_cleaned",
            Self::SourceUnverified => "source_unverified",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "eligible" => Some(Self::Eligible),
            "build_running" => Some(Self::BuildRunning),
            "retention_protected" => Some(Self::RetentionProtected),
            "borrowed_resource" => Some(Self::BorrowedResource),
            "already_cleaned" => Some(Self::AlreadyCleaned),
            "source_unverified" => Some(Self::SourceUnverified),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum CleanupResource {
    Workspace {
        id: CleanupResourceId,
        workspace_id: WorkspaceId,
        ownership: WorkspaceOwnership,
        containment_root: Option<ContainmentRoot>,
    },
    ArtifactSet {
        id: CleanupResourceId,
        artifact_set_id: ArtifactSetId,
        storage_key: ArtifactStorageKey,
        containment_root: ContainmentRoot,
    },
    AttemptLogs {
        id: CleanupResourceId,
        attempt_id: OperationAttemptId,
        storage_key: ArtifactStorageKey,
        containment_root: ContainmentRoot,
    },
    BuildScratch {
        id: CleanupResourceId,
        build_id: ReviewBuildId,
        storage_key: ArtifactStorageKey,
        containment_root: ContainmentRoot,
    },
    RuntimeInstance {
        id: CleanupResourceId,
        build_id: ReviewBuildId,
        locator: ResourceLocator,
    },
    PortLease {
        id: CleanupResourceId,
        build_id: ReviewBuildId,
        locator: ResourceLocator,
    },
}

impl CleanupResource {
    pub(crate) fn id(&self) -> &CleanupResourceId {
        match self {
            Self::Workspace { id, .. }
            | Self::ArtifactSet { id, .. }
            | Self::AttemptLogs { id, .. }
            | Self::BuildScratch { id, .. }
            | Self::RuntimeInstance { id, .. }
            | Self::PortLease { id, .. } => id,
        }
    }

    pub(crate) fn removable_for_build(&self, build_id: &ReviewBuildId) -> bool {
        match self {
            Self::Workspace { ownership, .. } => ownership.is_removable_with_build(build_id),
            Self::BuildScratch {
                build_id: owner, ..
            } => owner == build_id,
            Self::RuntimeInstance {
                build_id: owner, ..
            }
            | Self::PortLease {
                build_id: owner, ..
            } => owner == build_id,
            Self::ArtifactSet { .. } | Self::AttemptLogs { .. } => true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CleanupJobState {
    Planned,
    Running,
    Completed,
    AttentionRequired,
    NotEligible,
}

impl CleanupJobState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::AttentionRequired => "attention_required",
            Self::NotEligible => "not_eligible",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "planned" => Some(Self::Planned),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "attention_required" => Some(Self::AttentionRequired),
            "not_eligible" => Some(Self::NotEligible),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupJob {
    pub(crate) id: CleanupJobId,
    pub(crate) build_id: ReviewBuildId,
    pub(crate) trigger: CleanupTrigger,
    pub(crate) eligibility: CleanupEligibility,
    pub(crate) state: CleanupJobState,
    pub(crate) resources: Vec<CleanupResource>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) settled_at: Option<DateTime<Utc>>,
}

impl CleanupJob {
    pub(crate) fn validate(&self) -> Result<(), DomainError> {
        let unique = self
            .resources
            .iter()
            .map(|resource| resource.id())
            .collect::<HashSet<_>>();
        if unique.len() != self.resources.len() {
            return Err(DomainError::new(
                "cleanup resource IDs must be unique within a job",
            ));
        }
        if self.eligibility == CleanupEligibility::Eligible {
            if self.resources.is_empty() {
                return Err(DomainError::new(
                    "eligible cleanup job must declare at least one owned resource",
                ));
            }
            if self.state == CleanupJobState::NotEligible {
                return Err(DomainError::new(
                    "eligible cleanup job cannot be recorded as not eligible",
                ));
            }
            if self
                .resources
                .iter()
                .any(|resource| !resource.removable_for_build(&self.build_id))
            {
                return Err(DomainError::new(
                    "eligible cleanup job contains a resource the build does not own",
                ));
            }
        } else if self.state != CleanupJobState::NotEligible {
            return Err(DomainError::new(
                "ineligible cleanup evaluation must be recorded as not eligible",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CleanupDisposition {
    Pending,
    Removed,
    AlreadyAbsent,
    Retained,
    Failed,
}

impl CleanupDisposition {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Removed => "removed",
            Self::AlreadyAbsent => "already_absent",
            Self::Retained => "retained",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "removed" => Some(Self::Removed),
            "already_absent" => Some(Self::AlreadyAbsent),
            "retained" => Some(Self::Retained),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }

    pub(crate) fn is_terminal(self) -> bool {
        self != Self::Pending
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupEffect {
    pub(crate) resource_id: CleanupResourceId,
    pub(crate) disposition: CleanupDisposition,
    pub(crate) detail: Option<String>,
    pub(crate) recorded_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupReceipt {
    pub(crate) job_id: CleanupJobId,
    pub(crate) build_id: ReviewBuildId,
    pub(crate) effects: Vec<CleanupEffect>,
    pub(crate) completed_at: DateTime<Utc>,
}

impl CleanupReceipt {
    pub(crate) fn validate_against(&self, job: &CleanupJob) -> Result<(), DomainError> {
        if self.job_id != job.id || self.build_id != job.build_id {
            return Err(DomainError::new(
                "cleanup receipt must identify the job and build it settles",
            ));
        }
        let expected = job
            .resources
            .iter()
            .map(|resource| resource.id())
            .collect::<HashSet<_>>();
        let actual = self
            .effects
            .iter()
            .map(|effect| &effect.resource_id)
            .collect::<HashSet<_>>();
        if expected != actual
            || self
                .effects
                .iter()
                .any(|effect| !effect.disposition.is_terminal())
        {
            return Err(DomainError::new(
                "cleanup receipt requires one terminal effect for every planned resource",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RetentionPolicy {
    KeepNewestSuccessfulPerLogicalSource { count: u32 },
}

impl RetentionPolicy {
    pub(crate) fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::KeepNewestSuccessfulPerLogicalSource { count } if *count > 0 => Ok(()),
            Self::KeepNewestSuccessfulPerLogicalSource { .. } => Err(DomainError::new(
                "retention policy must keep at least one successful build",
            )),
        }
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self::KeepNewestSuccessfulPerLogicalSource { count: 1 }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSettings {
    pub(crate) retention_policy: RetentionPolicy,
    pub(crate) updated_at: DateTime<Utc>,
}

impl ReviewSettings {
    pub(crate) fn validate(&self) -> Result<(), DomainError> {
        self.retention_policy.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_review::domain::WorktreeAssociationId;

    #[test]
    fn borrowed_worktree_cannot_enter_a_build_cleanup_ledger() {
        let build_id = ReviewBuildId::new("build").unwrap();
        let job = CleanupJob {
            id: CleanupJobId::new("job").unwrap(),
            build_id,
            trigger: CleanupTrigger::RetentionPolicy,
            eligibility: CleanupEligibility::Eligible,
            state: CleanupJobState::Planned,
            resources: vec![CleanupResource::Workspace {
                id: CleanupResourceId::new("resource").unwrap(),
                workspace_id: WorkspaceId::new("workspace").unwrap(),
                ownership: WorkspaceOwnership::BorrowedExternal {
                    association_id: WorktreeAssociationId::new("association").unwrap(),
                },
                containment_root: None,
            }],
            created_at: Utc::now(),
            started_at: None,
            settled_at: None,
        };
        assert!(job.validate().is_err());
    }
}
