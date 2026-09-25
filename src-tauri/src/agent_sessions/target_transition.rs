//! Durable device/worktree switch intent, separate from accepted prompt preparation.
//!
//! A transition describes the work required to make a selected worktree usable on another
//! device. It deliberately does not own provider continuation or prompt delivery; ordinary
//! [`SessionPreparation`](super::preparation::SessionPreparation) does that after this record is
//! ready.

use super::domain::AgentSessionId;
use crate::execution_targets::domain::{
    SessionExecutionSelection, SessionExecutionTarget, SessionWorkspaceSelection,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionTargetTransition {
    pub(crate) session_id: AgentSessionId,
    pub(crate) source_target: SessionExecutionTarget,
    /// The selected capability profile and physical-worktree choice. `resolved_target` becomes
    /// available only after materialization/migration has completed.
    pub(crate) destination_selection: SessionExecutionSelection,
    /// Owned by execution-target sister tracking. It is optional while a first transition is
    /// being planned, then remains a durable link to the selected siblings.
    pub(crate) sister_group_id: Option<String>,
    pub(crate) phase: TargetTransitionPhase,
    pub(crate) tasks: Vec<TargetTransitionTask>,
    #[serde(default)]
    pub(crate) snapshot: Option<WorktreeSnapshotDescriptor>,
    #[serde(default)]
    pub(crate) transfer_estimate: Option<SnapshotTransferEstimate>,
    /// One user prompt can wait behind an unfinished switch. It remains distinct from an Agent
    /// Session invocation until the normal preparation path accepts it.
    #[serde(default)]
    pub(crate) queued_prompt: Option<QueuedTransitionPrompt>,
    #[serde(default)]
    pub(crate) resolved_target: Option<SessionExecutionTarget>,
    #[serde(default)]
    pub(crate) error: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetTransitionPhase {
    Pending,
    Running,
    Ready,
    Failed,
    Canceled,
}

impl TargetTransitionPhase {
    pub(crate) fn is_unfinished(self) -> bool {
        matches!(self, Self::Pending | Self::Running)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetTransitionTask {
    pub(crate) kind: TargetTransitionTaskKind,
    pub(crate) status: TargetTransitionTaskStatus,
    #[serde(default)]
    pub(crate) detail: Option<String>,
    #[serde(default)]
    pub(crate) error: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetTransitionTaskKind {
    EnsureDestinationDeviceReady,
    InspectSource,
    InspectDestination,
    CaptureSnapshot,
    MaterializeDestination,
    TransferSnapshot,
    ApplySnapshot,
    VerifyDestination,
    ActivateSister,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetTransitionTaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeSnapshotDescriptor {
    pub(crate) source_head: Option<String>,
    pub(crate) destination_head: Option<String>,
    pub(crate) commit_bundle: SnapshotArtifact,
    pub(crate) staged_patch: SnapshotArtifact,
    pub(crate) unstaged_patch: SnapshotArtifact,
    pub(crate) untracked_files: SnapshotArtifact,
    pub(crate) total_bytes: u64,
}

/// Metadata only. Snapshot contents remain short-lived device-local or in the transfer stream.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SnapshotArtifact {
    pub(crate) entry_count: u64,
    pub(crate) bytes: u64,
    #[serde(default)]
    pub(crate) digest: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SnapshotTransferEstimate {
    pub(crate) snapshot_bytes: u64,
    pub(crate) bytes_per_second: u64,
    pub(crate) estimated_seconds: u64,
    pub(crate) measured_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueuedTransitionPrompt {
    pub(crate) text: String,
    pub(crate) client_message_id: String,
}

pub(crate) fn validate_target_transition(
    transition: &SessionTargetTransition,
) -> Result<(), String> {
    validate_target("source", &transition.source_target)?;
    validate_destination(&transition.source_target, &transition.destination_selection)?;

    if transition.tasks.is_empty() {
        return Err("Target transition requires at least one task".into());
    }
    let mut seen = std::collections::HashSet::new();
    if transition.tasks.iter().any(|task| !seen.insert(task.kind)) {
        return Err("Target transition tasks must not repeat a kind".into());
    }
    if transition
        .sister_group_id
        .as_deref()
        .is_some_and(|id| id.trim().is_empty())
    {
        return Err("Target transition sister group cannot be blank when present".into());
    }
    if transition.updated_at < transition.created_at {
        return Err("Target transition update time cannot precede creation time".into());
    }
    if let Some(prompt) = &transition.queued_prompt {
        if prompt.text.trim().is_empty() || prompt.client_message_id.trim().is_empty() {
            return Err(
                "Queued target-transition prompt requires text and a client message ID".into(),
            );
        }
    }
    if let Some(snapshot) = &transition.snapshot {
        validate_snapshot(snapshot)?;
        if let Some(estimate) = &transition.transfer_estimate {
            if estimate.snapshot_bytes != snapshot.total_bytes {
                return Err(
                    "Target transition estimate must describe the captured snapshot".into(),
                );
            }
            if estimate.bytes_per_second == 0 {
                return Err("Target transition transfer rate must be greater than zero".into());
            }
        }
    } else if transition.transfer_estimate.is_some() {
        return Err("Target transition estimate requires a captured snapshot".into());
    }

    match transition.phase {
        TargetTransitionPhase::Ready => {
            let resolved = transition
                .resolved_target
                .as_ref()
                .ok_or("Ready target transition requires a resolved target")?;
            validate_target("resolved", resolved)?;
            if resolved.execution.device_id != transition.destination_selection.execution.device_id
            {
                return Err("Ready target transition resolved a different device".into());
            }
            if transition.error.is_some() {
                return Err("Ready target transition cannot contain an error".into());
            }
        }
        TargetTransitionPhase::Pending => {
            if transition.resolved_target.is_some() {
                return Err("Unfinished target transition cannot resolve its target".into());
            }
        }
        TargetTransitionPhase::Running => {
            if transition.resolved_target.is_some() {
                return Err("Unfinished target transition cannot resolve its target".into());
            }
            if transition.error.is_some() {
                return Err("Running target transition cannot contain an error".into());
            }
        }
        TargetTransitionPhase::Failed => {
            if transition
                .error
                .as_deref()
                .is_none_or(|error| error.trim().is_empty())
            {
                return Err("Failed target transition requires an error".into());
            }
        }
        TargetTransitionPhase::Canceled => {
            if transition.resolved_target.is_some() {
                return Err("Canceled target transition cannot resolve its target".into());
            }
        }
    }
    Ok(())
}

fn validate_target(label: &str, target: &SessionExecutionTarget) -> Result<(), String> {
    target
        .execution
        .validate()
        .map_err(|error| format!("Invalid {label} target: {error}"))?;
    for (field, value) in [
        ("capability profile", target.capability_profile_id.as_str()),
        ("repository", target.repository_id.as_str()),
        ("branch", target.branch_ref.as_str()),
        ("worktree", target.worktree_id.as_str()),
        ("path", target.path.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(format!("Invalid {label} target: {field} cannot be blank"));
        }
    }
    Ok(())
}

fn validate_destination(
    source: &SessionExecutionTarget,
    selection: &SessionExecutionSelection,
) -> Result<(), String> {
    selection
        .execution
        .validate()
        .map_err(|error| format!("Invalid destination selection: {error}"))?;
    if selection.capability_profile_id.trim().is_empty() {
        return Err("Destination capability profile cannot be blank".into());
    }
    if selection.execution.device_id == source.execution.device_id {
        return Err("Destination device must differ from the source device".into());
    }
    let (repository_id, branch_ref) = match &selection.workspace {
        SessionWorkspaceSelection::Existing { target } => {
            validate_target("destination", target)?;
            (&target.repository_id, &target.branch_ref)
        }
        SessionWorkspaceSelection::Create {
            repository_id,
            branch_ref,
            commit,
            attachment,
        } => {
            if repository_id.trim().is_empty()
                || branch_ref.trim().is_empty()
                || commit.trim().is_empty()
                || attachment.trim().is_empty()
            {
                return Err(
                    "New destination worktree requires repository, branch, commit, and attachment"
                        .into(),
                );
            }
            (repository_id, branch_ref)
        }
        SessionWorkspaceSelection::Auxiliary => {
            return Err("Target transition requires a concrete destination worktree".into());
        }
    };
    if repository_id != &source.repository_id || branch_ref != &source.branch_ref {
        return Err(
            "Target transition destination must use the source repository and branch".into(),
        );
    }
    Ok(())
}

fn validate_snapshot(snapshot: &WorktreeSnapshotDescriptor) -> Result<(), String> {
    let content_bytes = snapshot
        .commit_bundle
        .bytes
        .saturating_add(snapshot.staged_patch.bytes)
        .saturating_add(snapshot.unstaged_patch.bytes)
        .saturating_add(snapshot.untracked_files.bytes);
    if snapshot.total_bytes < content_bytes {
        return Err("Target transition snapshot total is smaller than its artifacts".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_targets::domain::{ExecutionBinding, ExecutionConnection};

    fn target(device_id: &str) -> SessionExecutionTarget {
        SessionExecutionTarget {
            capability_profile_id: format!("profile-{device_id}"),
            capability_profile_revision: 1,
            execution: ExecutionBinding {
                device_id: device_id.into(),
                device_name: device_id.into(),
                provider: "codex".into(),
                configuration_ref: "default".into(),
                connection: if device_id == "laptop" {
                    ExecutionConnection::Local
                } else {
                    ExecutionConnection::Ssh {
                        target: "orchid@example.test".into(),
                        host_executable: "orchid-host".into(),
                    }
                },
            },
            repository_id: "orchid".into(),
            branch_ref: "refs/heads/feature/remote".into(),
            worktree_id: format!("{device_id}-worktree"),
            path: format!("/work/{device_id}"),
            head: Some("abc123".into()),
        }
    }

    fn transition() -> SessionTargetTransition {
        let source = target("laptop");
        let destination = target("server");
        SessionTargetTransition {
            session_id: AgentSessionId::new("session").unwrap(),
            source_target: source,
            destination_selection: SessionExecutionSelection {
                capability_profile_id: destination.capability_profile_id.clone(),
                capability_profile_revision: destination.capability_profile_revision,
                execution: destination.execution.clone(),
                workspace: SessionWorkspaceSelection::Existing {
                    target: destination,
                },
            },
            sister_group_id: Some("sister-group".into()),
            phase: TargetTransitionPhase::Pending,
            tasks: vec![TargetTransitionTask {
                kind: TargetTransitionTaskKind::InspectSource,
                status: TargetTransitionTaskStatus::Pending,
                detail: None,
                error: None,
            }],
            snapshot: None,
            transfer_estimate: None,
            queued_prompt: None,
            resolved_target: None,
            error: None,
            created_at: "2026-09-17T12:00:00Z".parse().unwrap(),
            updated_at: "2026-09-17T12:00:00Z".parse().unwrap(),
        }
    }

    #[test]
    fn ready_transition_requires_its_resolved_destination() {
        let mut record = transition();
        record.phase = TargetTransitionPhase::Ready;
        assert_eq!(
            validate_target_transition(&record),
            Err("Ready target transition requires a resolved target".into())
        );
        record.resolved_target = Some(target("laptop"));
        assert_eq!(
            validate_target_transition(&record),
            Err("Ready target transition resolved a different device".into())
        );
        record.resolved_target = match &record.destination_selection.workspace {
            SessionWorkspaceSelection::Existing { target } => Some(target.clone()),
            _ => unreachable!(),
        };
        assert_eq!(validate_target_transition(&record), Ok(()));
    }

    #[test]
    fn target_transition_rejects_a_different_branch_or_device() {
        let mut record = transition();
        record.destination_selection.execution.device_id = "laptop".into();
        assert_eq!(
            validate_target_transition(&record),
            Err("Destination device must differ from the source device".into())
        );
        record.destination_selection.execution.device_id = "server".into();
        if let SessionWorkspaceSelection::Existing { target } =
            &mut record.destination_selection.workspace
        {
            target.branch_ref = "refs/heads/other".into();
        }
        assert_eq!(
            validate_target_transition(&record),
            Err("Target transition destination must use the source repository and branch".into())
        );
    }
}
