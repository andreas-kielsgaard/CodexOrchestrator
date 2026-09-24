use super::{
    BuildOutputId, BuildOutputStorageKey, CleanupJobId, CleanupResourceId, CleanupStorageKey,
    ContainmentRoot, DomainError, OperationAttemptId, ReviewBuildId,
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
    BuildOutput {
        id: CleanupResourceId,
        output_id: BuildOutputId,
        storage_key: BuildOutputStorageKey,
        containment_root: ContainmentRoot,
    },
    AttemptLogs {
        id: CleanupResourceId,
        attempt_id: OperationAttemptId,
        storage_key: CleanupStorageKey,
        containment_root: ContainmentRoot,
    },
    /// Current code treats the legacy `build_scratch` wire name as the complete attempt directory.
    #[serde(rename = "build_scratch")]
    BuildAttemptStorage {
        id: CleanupResourceId,
        build_id: ReviewBuildId,
        /// Present for current attempt-directory resources; absent only on legacy ledgers.
        #[serde(default)]
        attempt_id: Option<OperationAttemptId>,
        storage_key: CleanupStorageKey,
        containment_root: ContainmentRoot,
    },
    ReviewRuntime {
        id: CleanupResourceId,
        build_id: ReviewBuildId,
        storage_key: CleanupStorageKey,
        containment_root: ContainmentRoot,
    },
}

impl CleanupResource {
    pub(crate) fn id(&self) -> &CleanupResourceId {
        match self {
            Self::BuildOutput { id, .. }
            | Self::AttemptLogs { id, .. }
            | Self::BuildAttemptStorage { id, .. }
            | Self::ReviewRuntime { id, .. } => id,
        }
    }

    pub(crate) fn removable_for_build(&self, build_id: &ReviewBuildId) -> bool {
        match self {
            Self::BuildAttemptStorage {
                build_id: owner, ..
            }
            | Self::ReviewRuntime {
                build_id: owner, ..
            } => owner == build_id,
            Self::BuildOutput { .. } | Self::AttemptLogs { .. } => true,
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
