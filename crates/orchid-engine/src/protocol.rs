//! Newline-delimited JSON carried by an Orchid SSH stdio connection.
use crate::{
    configuration::{NativeCapabilityInventory, RuntimeProfileSnapshot},
    contracts::*,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const HOST_PROTOCOL_VERSION: u32 = 3;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HostRequest {
    pub id: String,
    #[serde(flatten)]
    pub command: HostCommand,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "method",
    content = "params",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum HostCommand {
    Describe,
    PublishedCommit {
        repository_root: String,
        branch_ref: String,
    },
    MaterializeWorktree {
        repository_root: String,
        branch_ref: String,
        commit: String,
        instance_id: String,
    },
    InspectWorktree {
        worktree_root: String,
        compare_to_head: Option<String>,
    },
    CaptureWorktreeSnapshot {
        worktree_root: String,
        destination_head: Option<String>,
        snapshot_id: String,
    },
    ApplyWorktreeSnapshot {
        worktree_root: String,
        snapshot: WorktreeSnapshot,
    },
    AuxiliaryWorkspace {
        session_id: String,
    },
    ExportContinuation {
        provider: String,
        configuration_ref: String,
        external_context_id: ExternalRuntimeContextId,
    },
    InstallContinuation {
        provider: String,
        configuration_ref: String,
        continuation: crate::contracts::provider::ProviderContinuationPayload,
    },
    /// Copies a stored native conversation into a new native context for a destination Session.
    ForkContinuation {
        provider: String,
        configuration_ref: String,
        external_context_id: ExternalRuntimeContextId,
        working_directory: String,
    },
    PrepareInvocation {
        provider: String,
        configuration_ref: String,
        request: RuntimeInvocationRequest,
        external_context_id: Option<ExternalRuntimeContextId>,
    },
    DeliverPreparedInvocation {
        invocation_id: AgentInvocationId,
    },
    ListWorktrees {
        repository_root: String,
        branch_ref: Option<String>,
    },
    Capabilities {
        provider: String,
        configuration_ref: String,
        working_directory: Option<String>,
    },
    Preflight {
        provider: String,
        configuration_ref: String,
        mode: RuntimeInvocationMode,
        options: AgentRuntimeOptions,
    },
    Invoke {
        provider: String,
        configuration_ref: String,
        request: RuntimeInvocationRequest,
        external_context_id: Option<ExternalRuntimeContextId>,
    },
    Respond {
        invocation_id: AgentInvocationId,
        request_id: String,
        response: RuntimeInteractionResponse,
    },
    Cancel {
        invocation_id: AgentInvocationId,
    },
    ActiveTurn {
        invocation_id: AgentInvocationId,
    },
    Steer {
        invocation_id: AgentInvocationId,
        target: RuntimeTurnTarget,
        input_id: String,
        text: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum HostFrame {
    Response {
        id: String,
        result: Option<Value>,
        error: Option<RuntimePortError>,
    },
    Update {
        invocation_id: AgentInvocationId,
        update: RuntimeUpdate,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostDescription {
    pub contract_version: u32,
    pub device_id: String,
    pub device_name: String,
    pub configurations: Vec<HostConfigurationDescription>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostConfigurationDescription {
    pub id: String,
    pub provider_kind: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCapabilities {
    pub profile: RuntimeProfileSnapshot,
    pub inventory: NativeCapabilityInventory,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInstance {
    pub handle: String,
    pub path: String,
    pub branch_ref: Option<String>,
    pub head: Option<String>,
    #[serde(default)]
    pub dirty: bool,
    pub head_committed_at: Option<String>,
}

/// Counts and byte estimates for one layer of a Git working tree.
///
/// `staged` describes the index relative to `HEAD`; `unstaged` describes the
/// working tree relative to the index; `untracked` includes only nonignored
/// files.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeChangeSummary {
    pub files: u32,
    pub bytes: u64,
}

/// The current worktree `HEAD` relative to an explicitly supplied commit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeHeadRelation {
    Equal,
    Ahead,
    Behind,
    Diverged,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeHeadComparison {
    pub reference_head: String,
    pub relation: WorktreeHeadRelation,
}

/// Device-local Git facts used by an Orchid worktree transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInspection {
    pub path: String,
    pub branch_ref: Option<String>,
    pub head: String,
    pub detached: bool,
    pub staged: WorktreeChangeSummary,
    pub unstaged: WorktreeChangeSummary,
    pub untracked: WorktreeChangeSummary,
    pub comparison: Option<WorktreeHeadComparison>,
}

/// Durable metadata for a portable, product-owned working-tree snapshot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeSnapshotDescriptor {
    pub id: String,
    pub source_branch_ref: Option<String>,
    pub source_head: String,
    pub destination_head: Option<String>,
    pub virtual_ref: String,
    pub commit_bundle_bytes: u64,
    pub staged_files: u32,
    pub staged_patch_bytes: u64,
    pub unstaged_files: u32,
    pub unstaged_patch_bytes: u64,
    pub untracked_files: u32,
    pub untracked_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeSnapshotFile {
    pub path: String,
    pub content: Vec<u8>,
    pub executable: bool,
}

/// Snapshot payload suitable for local calls and the Orchid host protocol.
///
/// The payload has no product persistence semantics. Callers may retain only
/// its descriptor after the bytes have crossed their transport boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeSnapshot {
    pub descriptor: WorktreeSnapshotDescriptor,
    pub commit_bundle: Vec<u8>,
    pub staged_patch: Vec<u8>,
    pub unstaged_patch: Vec<u8>,
    pub untracked_files: Vec<WorktreeSnapshotFile>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frames_preserve_request_and_invocation_identity() {
        let request: HostRequest = serde_json::from_str(
            r#"{"id":"request-1","method":"cancel","params":{"invocationId":"invocation-2"}}"#,
        )
        .unwrap();
        assert_eq!(request.id, "request-1");
        assert!(
            matches!(request.command, HostCommand::Cancel { invocation_id } if invocation_id.as_str() == "invocation-2")
        );
        let frame = HostFrame::Update {
            invocation_id: AgentInvocationId::new("invocation-2").unwrap(),
            update: RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                status: AgentInvocationTerminalStatus::Canceled,
                exit_code: None,
                signal: None,
                runtime_error: None,
            }),
        };
        let value = serde_json::to_value(&frame).unwrap();
        assert_eq!(value["kind"], "update");
        assert_eq!(value["invocationId"], "invocation-2");
        assert!(serde_json::from_value::<HostFrame>(value).is_ok());
    }

    #[test]
    fn worktree_snapshot_requests_round_trip_portable_payloads() {
        let snapshot = WorktreeSnapshot {
            descriptor: WorktreeSnapshotDescriptor {
                id: "snapshot-1".into(),
                source_branch_ref: Some("refs/heads/main".into()),
                source_head: "a".repeat(40),
                destination_head: Some("b".repeat(40)),
                virtual_ref: "refs/orchid/snapshots/snapshot-1".into(),
                commit_bundle_bytes: 2,
                staged_files: 1,
                staged_patch_bytes: 1,
                unstaged_files: 0,
                unstaged_patch_bytes: 0,
                untracked_files: 1,
                untracked_bytes: 1,
                total_bytes: 4,
            },
            commit_bundle: vec![1, 2],
            staged_patch: vec![3],
            unstaged_patch: vec![],
            untracked_files: vec![WorktreeSnapshotFile {
                path: "notes.txt".into(),
                content: vec![4],
                executable: false,
            }],
        };
        let request = HostRequest {
            id: "request-1".into(),
            command: HostCommand::ApplyWorktreeSnapshot {
                worktree_root: "/workspace".into(),
                snapshot,
            },
        };
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["method"], "apply_worktree_snapshot");
        assert_eq!(
            value["params"]["snapshot"]["commitBundle"],
            serde_json::json!([1, 2])
        );
        assert!(matches!(
            serde_json::from_value::<HostRequest>(value).unwrap().command,
            HostCommand::ApplyWorktreeSnapshot { snapshot, .. }
                if snapshot.untracked_files[0].content == vec![4]
        ));
    }
}
