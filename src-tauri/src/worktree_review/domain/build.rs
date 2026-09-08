use super::{
    BuildAttentionId, BuildOutputId, BuildOutputStorageKey, DomainError, ExecutableRelativePath,
    OperationAttemptId, RetentionKey, ReviewBuildId, ReviewBuildName, SourceBinding, WorkspaceId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BuildLifecycle {
    Active,
    Superseded,
    CleanupPending,
    Cleaned,
    UnverifiedLegacy,
}

impl BuildLifecycle {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Superseded => "superseded",
            Self::CleanupPending => "cleanup_pending",
            Self::Cleaned => "cleaned",
            Self::UnverifiedLegacy => "unverified_legacy",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "superseded" => Some(Self::Superseded),
            "cleanup_pending" => Some(Self::CleanupPending),
            "cleaned" => Some(Self::Cleaned),
            "unverified_legacy" => Some(Self::UnverifiedLegacy),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewBuild {
    pub(crate) id: ReviewBuildId,
    pub(crate) name: ReviewBuildName,
    pub(crate) source: SourceBinding,
    pub(crate) workspace_id: WorkspaceId,
    pub(crate) retention_key: RetentionKey,
    pub(crate) current_output_id: Option<BuildOutputId>,
    pub(crate) lifecycle: BuildLifecycle,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl ReviewBuild {
    pub(crate) fn validate(&self) -> Result<(), DomainError> {
        if self.workspace_id != self.source.workspace_id {
            return Err(DomainError::new(
                "build workspace must match its recorded source binding",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReviewOperationKind {
    MaterializeSource,
    Build,
}

impl ReviewOperationKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::MaterializeSource => "materialize_source",
            Self::Build => "build",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "materialize_source" => Some(Self::MaterializeSource),
            "build" => Some(Self::Build),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OperationExecutionState {
    Pending,
    Running,
    Completed,
    Interrupted,
}

impl OperationExecutionState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Interrupted => "interrupted",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "completed" => Some(Self::Completed),
            "interrupted" => Some(Self::Interrupted),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OperationVerdict {
    Unknown,
    Passed,
    Failed,
    Cancelled,
}

impl OperationVerdict {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "unknown" => Some(Self::Unknown),
            "passed" => Some(Self::Passed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OperationStage {
    SourceVerification,
    WorktreeProvisioning,
    DependencyProvisioning,
    Compilation,
    OutputPublication,
    InterruptionReconciliation,
}

impl OperationStage {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SourceVerification => "source_verification",
            Self::WorktreeProvisioning => "worktree_provisioning",
            Self::DependencyProvisioning => "dependency_provisioning",
            Self::Compilation => "compilation",
            Self::OutputPublication => "output_publication",
            Self::InterruptionReconciliation => "interruption_reconciliation",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "source_verification" => Some(Self::SourceVerification),
            "worktree_provisioning" => Some(Self::WorktreeProvisioning),
            "dependency_provisioning" => Some(Self::DependencyProvisioning),
            "compilation" => Some(Self::Compilation),
            "output_publication" | "artifact_verification" | "artifact_promotion" => {
                Some(Self::OutputPublication)
            }
            "interruption_reconciliation" | "recovery" => Some(Self::InterruptionReconciliation),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OperationFailureCategory {
    SourceChanged,
    SourceUnavailable,
    ProvisioningFailed,
    ToolchainUnavailable,
    CommandFailed,
    OutputMissing,
    OutputPublicationFailed,
    Interrupted,
    Internal,
}

impl OperationFailureCategory {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SourceChanged => "source_changed",
            Self::SourceUnavailable => "source_unavailable",
            Self::ProvisioningFailed => "provisioning_failed",
            Self::ToolchainUnavailable => "toolchain_unavailable",
            Self::CommandFailed => "command_failed",
            Self::OutputMissing => "output_missing",
            Self::OutputPublicationFailed => "output_publication_failed",
            Self::Interrupted => "interrupted",
            Self::Internal => "internal",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "source_changed" => Some(Self::SourceChanged),
            "source_unavailable" => Some(Self::SourceUnavailable),
            "provisioning_failed" => Some(Self::ProvisioningFailed),
            "toolchain_unavailable" => Some(Self::ToolchainUnavailable),
            "command_failed" => Some(Self::CommandFailed),
            "output_missing" | "artifact_missing" => Some(Self::OutputMissing),
            "output_publication_failed" | "artifact_invalid" => Some(Self::OutputPublicationFailed),
            "interrupted" => Some(Self::Interrupted),
            "internal" => Some(Self::Internal),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OperationFailure {
    pub(crate) stage: OperationStage,
    pub(crate) category: OperationFailureCategory,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewOperationAttempt {
    pub(crate) id: OperationAttemptId,
    pub(crate) build_id: ReviewBuildId,
    pub(crate) kind: ReviewOperationKind,
    pub(crate) execution: OperationExecutionState,
    pub(crate) verdict: OperationVerdict,
    pub(crate) active_stage: Option<OperationStage>,
    pub(crate) failure: Option<OperationFailure>,
    pub(crate) requested_at: DateTime<Utc>,
    pub(crate) started_at: Option<DateTime<Utc>>,
    pub(crate) completed_at: Option<DateTime<Utc>>,
}

impl ReviewOperationAttempt {
    pub(crate) fn validate(&self) -> Result<(), DomainError> {
        match self.execution {
            OperationExecutionState::Pending => {
                if self.verdict != OperationVerdict::Unknown
                    || self.started_at.is_some()
                    || self.completed_at.is_some()
                    || self.failure.is_some()
                {
                    return Err(DomainError::new(
                        "pending operation cannot carry a verdict, timestamps, or failure",
                    ));
                }
            }
            OperationExecutionState::Running => {
                if self.verdict != OperationVerdict::Unknown
                    || self.started_at.is_none()
                    || self.completed_at.is_some()
                    || self.failure.is_some()
                {
                    return Err(DomainError::new(
                        "running operation requires a start and cannot carry terminal evidence",
                    ));
                }
            }
            OperationExecutionState::Completed => {
                if self.verdict == OperationVerdict::Unknown
                    || self.started_at.is_none()
                    || self.completed_at.is_none()
                {
                    return Err(DomainError::new(
                        "completed operation requires start, completion, and an explicit verdict",
                    ));
                }
                if (self.verdict == OperationVerdict::Failed) != self.failure.is_some() {
                    return Err(DomainError::new(
                        "failure evidence must exist exactly when a completed operation failed",
                    ));
                }
            }
            OperationExecutionState::Interrupted => {
                if self.verdict != OperationVerdict::Unknown || self.completed_at.is_none() {
                    return Err(DomainError::new(
                        "interrupted operation remains unknown and records reconciliation time",
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn is_terminal(&self) -> bool {
        matches!(
            self.execution,
            OperationExecutionState::Completed | OperationExecutionState::Interrupted
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetainedBuildOutput {
    pub(crate) id: BuildOutputId,
    pub(crate) build_id: ReviewBuildId,
    pub(crate) attempt_id: OperationAttemptId,
    pub(crate) storage_key: BuildOutputStorageKey,
    pub(crate) executable_relative_path: ExecutableRelativePath,
    pub(crate) published_at: DateTime<Utc>,
}

impl RetainedBuildOutput {
    pub(crate) fn validate(&self) -> Result<(), DomainError> {
        let storage_components = normal_relative_components(self.storage_key.as_str());
        let identifies_attempt = storage_components.as_deref().is_some_and(|components| {
            components.ends_with(&[self.build_id.as_str(), self.attempt_id.as_str(), "output"])
                || matches!(components, ["attempts", token, "output"] if valid_attempt_token(token))
        });
        if !identifies_attempt {
            return Err(DomainError::new(
                "build output storage key must identify its build attempt output",
            ));
        }
        if !is_normal_relative_path(self.executable_relative_path.as_str()) {
            return Err(DomainError::new(
                "build output executable must be a normalized relative path",
            ));
        }
        Ok(())
    }
}

fn valid_attempt_token(value: &str) -> bool {
    value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_normal_relative_path(value: &str) -> bool {
    normal_relative_components(value).is_some()
}

fn normal_relative_components(value: &str) -> Option<Vec<&str>> {
    let path = Path::new(value);
    if path.is_absolute() {
        return None;
    }
    let components = path
        .components()
        .map(|component| match component {
            Component::Normal(value) => value.to_str(),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    (!components.is_empty()).then_some(components)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BuildAttentionCategory {
    CleanupCoordination,
}

impl BuildAttentionCategory {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::CleanupCoordination => "cleanup_coordination",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "cleanup_coordination" => Some(Self::CleanupCoordination),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildAttention {
    pub(crate) id: BuildAttentionId,
    pub(crate) build_id: ReviewBuildId,
    pub(crate) category: BuildAttentionCategory,
    pub(crate) summary: String,
    pub(crate) recorded_at: DateTime<Utc>,
    pub(crate) resolved_at: Option<DateTime<Utc>>,
}

impl BuildAttention {
    pub(crate) fn validate(&self) -> Result<(), DomainError> {
        if self.summary.trim().is_empty() || self.summary != self.summary.trim() {
            return Err(DomainError::new(
                "build attention summary must be nonblank without surrounding whitespace",
            ));
        }
        if self
            .resolved_at
            .is_some_and(|resolved_at| resolved_at < self.recorded_at)
        {
            return Err(DomainError::new(
                "build attention cannot resolve before it was recorded",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attempt(
        execution: OperationExecutionState,
        verdict: OperationVerdict,
    ) -> ReviewOperationAttempt {
        let now = Utc::now();
        ReviewOperationAttempt {
            id: OperationAttemptId::new("attempt").unwrap(),
            build_id: ReviewBuildId::new("build").unwrap(),
            kind: ReviewOperationKind::Build,
            execution,
            verdict,
            active_stage: None,
            failure: None,
            requested_at: now,
            started_at: (execution != OperationExecutionState::Pending).then_some(now),
            completed_at: matches!(
                execution,
                OperationExecutionState::Completed | OperationExecutionState::Interrupted
            )
            .then_some(now),
        }
    }

    #[test]
    fn transport_completion_cannot_stand_in_for_build_verdict() {
        let operation = attempt(
            OperationExecutionState::Completed,
            OperationVerdict::Unknown,
        );
        assert!(operation.validate().is_err());
    }

    #[test]
    fn interrupted_operations_remain_unknown() {
        let operation = attempt(
            OperationExecutionState::Interrupted,
            OperationVerdict::Unknown,
        );
        assert!(operation.validate().is_ok());
        assert!(operation.is_terminal());
    }

    #[test]
    fn build_attention_rejects_ambiguous_summary_or_timeline() {
        let recorded_at = Utc::now();
        let attention = BuildAttention {
            id: BuildAttentionId::new("attention").unwrap(),
            build_id: ReviewBuildId::new("build").unwrap(),
            category: BuildAttentionCategory::CleanupCoordination,
            summary: " cleanup needs coordination".into(),
            recorded_at,
            resolved_at: Some(recorded_at - chrono::Duration::seconds(1)),
        };

        assert!(attention.validate().is_err());
    }

    #[test]
    fn retained_output_paths_are_bounded_relative_values() {
        let now = Utc::now();
        let output = |storage_key: &str, executable: &str| RetainedBuildOutput {
            id: BuildOutputId::new("output").unwrap(),
            build_id: ReviewBuildId::new("build").unwrap(),
            attempt_id: OperationAttemptId::new("attempt").unwrap(),
            storage_key: BuildOutputStorageKey::new(storage_key).unwrap(),
            executable_relative_path: ExecutableRelativePath::new(executable).unwrap(),
            published_at: now,
        };

        assert!(
            output("outputs/build/attempt/output", "cargo-target/debug/app.exe")
                .validate()
                .is_ok()
        );
        assert!(output("../foreign", "app.exe").validate().is_err());
        assert!(output("outputs/build", "../app.exe").validate().is_err());
        assert!(output(
            "attempts/0123456789abcdef0123456789abcdef/output",
            "t/debug/app.exe"
        )
        .validate()
        .is_ok());
        assert!(output("attempts/not-an-attempt/output", "t/debug/app.exe")
            .validate()
            .is_err());
    }
}
