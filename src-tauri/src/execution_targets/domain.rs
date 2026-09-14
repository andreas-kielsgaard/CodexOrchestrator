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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionBinding {
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) provider: String,
    pub(crate) configuration_ref: String,
    pub(crate) connection: ExecutionConnection,
}

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
    pub(crate) branch_ref: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileWorktreeTargets {
    pub(crate) capability_profile_id: String,
    pub(crate) capability_profile_revision: u64,
    pub(crate) capability_profile_name: String,
    pub(crate) execution: ExecutionBinding,
    pub(crate) instances: Vec<TargetWorktree>,
    pub(crate) error: Option<String>,
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
pub(crate) struct ExecutionTargetRuntime {
    pub(crate) runtime_profile: crate::execution_configuration::RuntimeProfileSnapshot,
    pub(crate) native_inventory: crate::execution_configuration::NativeCapabilityInventory,
}
