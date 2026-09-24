use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ExecutionConnection {
    Local,
    Ssh {
        target: String,
        host_executable: String,
    },
}

/// Stable design-time reference stored by a Capability Profile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionRouteRef {
    pub(crate) device_id: String,
    pub(crate) provider: String,
    pub(crate) configuration_ref: String,
}

/// Concrete, immutable execution snapshot used by Sessions and runtime operations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedExecutionBinding {
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) provider: String,
    pub(crate) configuration_ref: String,
    pub(crate) connection: ExecutionConnection,
}

pub(crate) type ExecutionBinding = ResolvedExecutionBinding;

impl Default for ExecutionBinding {
    fn default() -> Self {
        Self {
            device_id: "local".into(),
            device_name: "This laptop".into(),
            provider: "codex".into(),
            configuration_ref: "selected".into(),
            connection: ExecutionConnection::Local,
        }
    }
}

impl ExecutionBinding {
    pub(crate) fn route_ref(&self) -> ExecutionRouteRef {
        ExecutionRouteRef {
            device_id: self.device_id.clone(),
            provider: self.provider.clone(),
            configuration_ref: self.configuration_ref.clone(),
        }
    }
    pub(crate) fn is_remote(&self) -> bool {
        matches!(self.connection, ExecutionConnection::Ssh { .. })
    }
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.device_id.trim().is_empty()
            || self.device_name.trim().is_empty()
            || self.configuration_ref.trim().is_empty()
        {
            return Err(
                "Execution binding requires a device, name, and configuration reference".into(),
            );
        }
        if self.provider != "codex" {
            return Err("Remote session execution currently supports Codex only".into());
        }
        if let ExecutionConnection::Ssh {
            target,
            host_executable,
        } = &self.connection
        {
            if target.trim().is_empty() || host_executable.trim().is_empty() {
                return Err("SSH execution requires a target and Orchid host executable".into());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionExecutionTarget {
    pub(crate) capability_profile_id: String,
    pub(crate) capability_profile_revision: u64,
    pub(crate) execution: ExecutionBinding,
    pub(crate) repository_id: String,
    pub(crate) branch_ref: String,
    pub(crate) worktree_id: String,
    pub(crate) path: String,
    pub(crate) head: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetWorktree {
    pub(crate) worktree_id: String,
    pub(crate) path: String,
    pub(crate) head: Option<String>,
    pub(crate) dirty: bool,
    pub(crate) head_committed_at: Option<String>,
    pub(crate) branch_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sister_lock: Option<super::sisters::SisterWorktreeLock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_sister: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileWorktreeTargets {
    #[serde(flatten)]
    pub(crate) profile: ExecutionTargetProfile,
    pub(crate) instances: Vec<TargetWorktree>,
    pub(crate) error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sister_lock: Option<super::sisters::SisterWorktreeLock>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionTargetProfile {
    pub(crate) capability_profile_id: String,
    pub(crate) capability_profile_revision: u64,
    pub(crate) capability_profile_name: String,
    pub(crate) execution: ExecutionBinding,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeviceWorktreeTargets {
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) profiles: Vec<ProfileWorktreeTargets>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfiguredExecutionDevice {
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) profiles: Vec<ExecutionTargetProfile>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum WorktreeChoiceScope {
    Local,
    Device { device_id: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryWorktreeChoices {
    pub(crate) repository_id: String,
    pub(crate) repository_name: String,
    pub(crate) profiles: Vec<ProfileWorktreeTargets>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionTargetRuntime {
    pub(crate) runtime_profile: crate::execution_configuration::RuntimeProfileSnapshot,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionExecutionSelection {
    pub(crate) capability_profile_id: String,
    pub(crate) capability_profile_revision: u64,
    pub(crate) execution: ExecutionBinding,
    pub(crate) workspace: SessionWorkspaceSelection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum SessionWorkspaceSelection {
    Existing {
        target: SessionExecutionTarget,
    },
    Create {
        repository_id: String,
        branch_ref: String,
        commit: String,
        attachment: String,
    },
    Auxiliary,
}
