//! Destination preparation through the same local/SSH boundary as execution.
use super::{
    domain::*, endpoints::ExecutionEndpoints, ssh_connection::SshConnection, ExecutionTargetService,
};
use orchid_engine::protocol::{HostCommand, WorktreeInspection, WorktreeInstance, WorktreeSnapshot};

impl ExecutionEndpoints {
    /// Runs the same Git inspection locally or through the Orchid host. The result carries only
    /// device-local facts; Orchid decides whether those facts are eligible for a move.
    pub(crate) fn inspect_worktree(
        &self,
        binding: &ExecutionBinding,
        worktree_root: &str,
        compare_to_head: Option<&str>,
    ) -> Result<WorktreeInspection, String> {
        match &binding.connection {
            ExecutionConnection::Local => orchid_engine::workspaces::inspect_worktree(
                worktree_root,
                compare_to_head,
            )
            .map_err(|error| error.to_string()),
            ExecutionConnection::Ssh {
                target,
                host_executable,
            } => SshConnection::connect(target, host_executable)
                .map_err(|error| error.to_string())?
                .request(HostCommand::InspectWorktree {
                    worktree_root: worktree_root.into(),
                    compare_to_head: compare_to_head.map(Into::into),
                })
                .map_err(|error| error.to_string()),
        }
    }

    /// Captures a product-owned virtual Git snapshot. Its bytes can cross the existing JSON SSH
    /// protocol in this first slice; the descriptor remains usable when that transport streams.
    pub(crate) fn capture_worktree_snapshot(
        &self,
        binding: &ExecutionBinding,
        worktree_root: &str,
        destination_head: Option<&str>,
        snapshot_id: &str,
    ) -> Result<WorktreeSnapshot, String> {
        match &binding.connection {
            ExecutionConnection::Local => orchid_engine::workspaces::capture_worktree_snapshot(
                worktree_root,
                destination_head,
                snapshot_id,
            )
            .map_err(|error| error.to_string()),
            ExecutionConnection::Ssh {
                target,
                host_executable,
            } => SshConnection::connect(target, host_executable)
                .map_err(|error| error.to_string())?
                .request(HostCommand::CaptureWorktreeSnapshot {
                    worktree_root: worktree_root.into(),
                    destination_head: destination_head.map(Into::into),
                    snapshot_id: snapshot_id.into(),
                })
                .map_err(|error| error.to_string()),
        }
    }

    pub(crate) fn apply_worktree_snapshot(
        &self,
        binding: &ExecutionBinding,
        worktree_root: &str,
        snapshot: &WorktreeSnapshot,
    ) -> Result<WorktreeInspection, String> {
        match &binding.connection {
            ExecutionConnection::Local => orchid_engine::workspaces::apply_worktree_snapshot(
                worktree_root,
                snapshot,
            )
            .map_err(|error| error.to_string()),
            ExecutionConnection::Ssh {
                target,
                host_executable,
            } => SshConnection::connect(target, host_executable)
                .map_err(|error| error.to_string())?
                .request(HostCommand::ApplyWorktreeSnapshot {
                    worktree_root: worktree_root.into(),
                    snapshot: snapshot.clone(),
                })
                .map_err(|error| error.to_string()),
        }
    }

    pub(crate) fn transfer_continuation(
        &self,
        source: &ExecutionBinding,
        destination: &ExecutionBinding,
        external_id: &crate::agent_sessions::domain::ExternalRuntimeContextId,
    ) -> Result<(), String> {
        use orchid_engine::codex::app_server::continuation;
        let source = self.freeze_binding(source.clone())?;
        let destination = self.freeze_binding(destination.clone())?;
        if source.device_id == destination.device_id
            && source.configuration_ref == destination.configuration_ref
            && source.connection == destination.connection
        {
            return Ok(());
        }
        let payload: continuation::CodexContinuation = match &source.connection {
            ExecutionConnection::Local => {
                let home = self
                    .local_source
                    .configuration_home(&source.configuration_ref)
                    .map_err(|e| e.to_string())?;
                continuation::export("codex", &home, external_id.as_str())
                    .map_err(|e| e.to_string())?
            }
            ExecutionConnection::Ssh {
                target,
                host_executable,
            } => SshConnection::connect(target, host_executable)
                .map_err(|e| e.to_string())?
                .request(HostCommand::ExportContinuation {
                    configuration_ref: source.configuration_ref.clone(),
                    external_context_id: external_id.clone(),
                })
                .map_err(|e| e.to_string())?,
        };
        match &destination.connection {
            ExecutionConnection::Local => {
                let home = self
                    .local_source
                    .configuration_home(&destination.configuration_ref)
                    .map_err(|e| e.to_string())?;
                continuation::install("codex", &home, &payload).map_err(|e| e.to_string())
            }
            ExecutionConnection::Ssh {
                target,
                host_executable,
            } => SshConnection::connect(target, host_executable)
                .map_err(|e| e.to_string())?
                .request(HostCommand::InstallContinuation {
                    configuration_ref: destination.configuration_ref.clone(),
                    continuation: payload,
                })
                .map_err(|e| e.to_string()),
        }
    }
    pub(crate) fn published_commit(
        &self,
        binding: &ExecutionBinding,
        root: &str,
        branch: &str,
    ) -> Result<String, String> {
        match &binding.connection {
            ExecutionConnection::Local => {
                orchid_engine::workspaces::published_commit(root, branch).map_err(|e| e.to_string())
            }
            ExecutionConnection::Ssh {
                target,
                host_executable,
            } => {
                let value: serde_json::Value = SshConnection::connect(target, host_executable)
                    .map_err(|e| e.to_string())?
                    .request(HostCommand::PublishedCommit {
                        repository_root: root.into(),
                        branch_ref: branch.into(),
                    })
                    .map_err(|e| e.to_string())?;
                value["commit"]
                    .as_str()
                    .map(str::to_owned)
                    .ok_or_else(|| "Remote host did not return a published commit".into())
            }
        }
    }

    pub(crate) fn materialize_worktree(
        &self,
        binding: &ExecutionBinding,
        root: &str,
        branch: &str,
        commit: &str,
        instance_id: &str,
    ) -> Result<WorktreeInstance, String> {
        match &binding.connection {
            ExecutionConnection::Local => {
                orchid_engine::workspaces::materialize_worktree(root, branch, commit, instance_id)
                    .map_err(|e| e.to_string())
            }
            ExecutionConnection::Ssh {
                target,
                host_executable,
            } => SshConnection::connect(target, host_executable)
                .map_err(|e| e.to_string())?
                .request(HostCommand::MaterializeWorktree {
                    repository_root: root.into(),
                    branch_ref: branch.into(),
                    commit: commit.into(),
                    instance_id: instance_id.into(),
                })
                .map_err(|e| e.to_string()),
        }
    }

    pub(crate) fn auxiliary_workspace(
        &self,
        binding: &ExecutionBinding,
        session_id: &str,
    ) -> Result<String, String> {
        match &binding.connection {
            ExecutionConnection::Local => {
                let home = self
                    .local_source
                    .configuration_home(&binding.configuration_ref)
                    .map_err(|e| e.to_string())?;
                orchid_engine::workspaces::auxiliary_workspace(&home.join("orchid"), session_id)
                    .map_err(|e| e.to_string())
            }
            ExecutionConnection::Ssh {
                target,
                host_executable,
            } => SshConnection::connect(target, host_executable)
                .map_err(|e| e.to_string())?
                .request(HostCommand::AuxiliaryWorkspace {
                    session_id: session_id.into(),
                })
                .map_err(|e| e.to_string()),
        }
    }
}

impl ExecutionTargetService {
    pub(crate) fn repository_root(
        &self,
        repository_id: &str,
        execution: &ExecutionBinding,
    ) -> Result<String, String> {
        let repository = self
            .repositories
            .find(repository_id)?
            .ok_or("Registered repository was not found")?;
        if execution.is_remote() {
            self.locations
                .list()?
                .into_iter()
                .find(|l| l.repository_id == repository_id && l.device_id == execution.device_id)
                .map(|l| l.repository_root)
                .ok_or_else(|| "No repository path is configured on this device".into())
        } else {
            Ok(repository.anchor_root.to_string_lossy().into_owned())
        }
    }

    pub(crate) fn materialize_selection(
        &self,
        selection: &SessionExecutionSelection,
        workspace_id: &str,
    ) -> Result<SessionExecutionTarget, String> {
        let execution = self.endpoints.freeze_binding(selection.execution.clone())?;
        let (repository_id, instance) = match &selection.workspace {
            SessionWorkspaceSelection::Existing { target } => {
                if target.repository_id.is_empty() {
                    return Ok(SessionExecutionTarget {
                        capability_profile_id: selection.capability_profile_id.clone(),
                        capability_profile_revision: selection.capability_profile_revision,
                        execution,
                        ..target.clone()
                    });
                }
                let root = self.repository_root(&target.repository_id, &execution)?;
                let instance = self
                    .endpoints
                    .worktrees(&execution, &root, Some(&target.branch_ref))?
                    .into_iter()
                    .find(|i| i.handle == target.worktree_id && i.path == target.path)
                    .ok_or("Selected worktree is no longer available on this device")?;
                (target.repository_id.clone(), instance)
            }
            SessionWorkspaceSelection::Create {
                repository_id,
                branch_ref,
                commit,
                attachment,
            } => {
                if attachment != "branch" {
                    return Err("Unsupported worktree attachment".into());
                }
                let root = self.repository_root(repository_id, &execution)?;
                (
                    repository_id.clone(),
                    self.endpoints.materialize_worktree(
                        &execution,
                        &root,
                        branch_ref,
                        commit,
                        workspace_id,
                    )?,
                )
            }
            SessionWorkspaceSelection::Auxiliary => {
                let path = self
                    .endpoints
                    .auxiliary_workspace(&execution, workspace_id)?;
                (
                    String::new(),
                    WorktreeInstance {
                        handle: workspace_id.into(),
                        path,
                        branch_ref: None,
                        head: None,
                        dirty: false,
                        head_committed_at: None,
                    },
                )
            }
        };
        Ok(SessionExecutionTarget {
            capability_profile_id: selection.capability_profile_id.clone(),
            capability_profile_revision: selection.capability_profile_revision,
            execution,
            repository_id,
            branch_ref: instance.branch_ref.unwrap_or_default(),
            worktree_id: instance.handle,
            path: instance.path,
            head: instance.head,
        })
    }
}
