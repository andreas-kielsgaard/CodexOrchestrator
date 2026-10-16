use orchid_engine::contracts::ProviderConfigurationRef;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ExecutionConnection {
    Local,
    Device,
    Ssh {
        target: String,
        host_executable: String,
    },
}

/// Stable design-time reference stored by a Capability Profile.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionRouteRef {
    pub device_id: String,
    pub provider: String,
    pub configuration_ref: String,
}

impl ExecutionRouteRef {
    pub fn configuration(&self) -> ProviderConfigurationRef {
        ProviderConfigurationRef::new(&self.provider, &self.configuration_ref)
    }
}

/// Concrete, immutable execution snapshot used by Sessions and runtime operations.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedExecutionBinding {
    pub device_id: String,
    pub device_name: String,
    pub provider: String,
    pub configuration_ref: String,
    pub connection: ExecutionConnection,
}

pub type ExecutionBinding = ResolvedExecutionBinding;

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
    /// The provider configuration this binding executes with.
    pub fn configuration(&self) -> ProviderConfigurationRef {
        ProviderConfigurationRef::new(&self.provider, &self.configuration_ref)
    }
    pub fn route_ref(&self) -> ExecutionRouteRef {
        ExecutionRouteRef {
            device_id: self.device_id.clone(),
            provider: self.provider.clone(),
            configuration_ref: self.configuration_ref.clone(),
        }
    }
    pub fn is_remote(&self) -> bool {
        !matches!(self.connection, ExecutionConnection::Local)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.device_id.trim().is_empty()
            || self.device_name.trim().is_empty()
            || self.configuration_ref.trim().is_empty()
        {
            return Err(
                "Execution binding requires a device, name, and configuration reference".into(),
            );
        }
        if self.provider.trim().is_empty() {
            return Err("Execution binding requires an agent provider".into());
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
pub struct SessionExecutionTarget {
    pub capability_profile_id: String,
    pub capability_profile_revision: u64,
    pub execution: ExecutionBinding,
    pub repository_id: String,
    pub branch_ref: String,
    pub worktree_id: String,
    pub path: String,
    pub head: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetWorktree {
    pub created_at: Option<String>,
    pub worktree_id: String,
    pub path: String,
    pub head: Option<String>,
    pub dirty: bool,
    pub head_committed_at: Option<String>,
    pub branch_ref: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sister_lock: Option<super::sisters::SisterWorktreeLock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_sister: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileWorktreeTargets {
    #[serde(flatten)]
    pub profile: ExecutionTargetProfile,
    pub instances: Vec<TargetWorktree>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sister_lock: Option<super::sisters::SisterWorktreeLock>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionTargetProfile {
    pub capability_profile_id: String,
    pub capability_profile_revision: u64,
    pub capability_profile_name: String,
    pub execution: ExecutionBinding,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceWorktreeTargets {
    pub device_id: String,
    pub device_name: String,
    pub profiles: Vec<ProfileWorktreeTargets>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfiguredExecutionDevice {
    pub device_id: String,
    pub device_name: String,
    pub profiles: Vec<ExecutionTargetProfile>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum WorktreeChoiceScope {
    Local,
    Device { device_id: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryWorktreeChoices {
    pub repository_id: String,
    pub repository_name: String,
    pub profiles: Vec<ProfileWorktreeTargets>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionTargetRuntime {
    pub runtime_profile: crate::execution_configuration::RuntimeProfileSnapshot,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionExecutionSelection {
    pub capability_profile_id: String,
    pub capability_profile_revision: u64,
    pub execution: ExecutionBinding,
    pub workspace: SessionWorkspaceSelection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum SessionWorkspaceSelection {
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
