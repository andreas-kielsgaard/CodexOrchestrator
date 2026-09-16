use super::{domain::*, remote_runtime::RemoteRuntime, ssh_connection::SshConnection};
use crate::{
    agent_sessions::ports::AgentRuntime, execution_configuration::SelectedRuntimeProfileSource,
};
use orchid_engine::protocol::{HostCommand, RuntimeCapabilities, WorktreeInstance};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub(crate) struct ExecutionEndpoints {
    pub(super) local_source: Arc<dyn SelectedRuntimeProfileSource>,
    local_runtime: Arc<dyn AgentRuntime>,
    remote_runtimes: Mutex<HashMap<String, Arc<dyn AgentRuntime>>>,
}

impl ExecutionEndpoints {
    #[cfg(test)]
    pub(crate) fn with_runtime(
        self,
        binding: &ExecutionBinding,
        runtime: Arc<dyn AgentRuntime>,
    ) -> Self {
        self.remote_runtimes
            .lock()
            .unwrap()
            .insert(serde_json::to_string(binding).unwrap(), runtime);
        self
    }
    pub(crate) fn new(
        local_source: Arc<dyn SelectedRuntimeProfileSource>,
        local_runtime: Arc<dyn AgentRuntime>,
    ) -> Self {
        Self {
            local_source,
            local_runtime,
            remote_runtimes: Default::default(),
        }
    }
    pub(crate) fn describe_runtime(
        &self,
        binding: &ExecutionBinding,
        cwd: Option<&str>,
    ) -> Result<ExecutionTargetRuntime, String> {
        binding.validate()?;
        match &binding.connection {
            ExecutionConnection::Local => Ok(ExecutionTargetRuntime {
                runtime_profile: self
                    .local_source
                    .profile_for_configuration(&binding.configuration_ref, cwd)
                    .map_err(|e| e.to_string())?,
                native_inventory: self
                    .local_source
                    .inventory_for_configuration(&binding.configuration_ref, cwd)
                    .map_err(|e| e.to_string())?,
            }),
            ExecutionConnection::Ssh {
                target,
                host_executable,
            } => {
                let connection =
                    SshConnection::connect(target, host_executable).map_err(|e| e.to_string())?;
                let capabilities: RuntimeCapabilities = connection
                    .request(HostCommand::Capabilities {
                        configuration_ref: binding.configuration_ref.clone(),
                        working_directory: cwd.map(str::to_owned),
                    })
                    .map_err(|e| e.to_string())?;
                Ok(ExecutionTargetRuntime {
                    runtime_profile: capabilities.profile,
                    native_inventory: capabilities.inventory,
                })
            }
        }
    }
    pub(crate) fn freeze_binding(
        &self,
        mut binding: ExecutionBinding,
    ) -> Result<ExecutionBinding, String> {
        if !binding.is_remote() {
            binding.configuration_ref = self
                .local_source
                .resolve_configuration_ref(&binding.configuration_ref)
                .map_err(|e| e.to_string())?;
        }
        Ok(binding)
    }
    pub(crate) fn worktrees(
        &self,
        binding: &ExecutionBinding,
        root: &str,
        branch: Option<&str>,
    ) -> Result<Vec<WorktreeInstance>, String> {
        Ok(match &binding.connection {
            ExecutionConnection::Local => {
                orchid_engine::host::list_worktrees(root, branch).map_err(|e| e.to_string())?
            }
            ExecutionConnection::Ssh {
                target,
                host_executable,
            } => SshConnection::connect(target, host_executable)
                .map_err(|e| e.to_string())?
                .request(HostCommand::ListWorktrees {
                    repository_root: root.into(),
                    branch_ref: branch.map(str::to_owned),
                })
                .map_err(|e| e.to_string())?,
        })
    }
    pub(crate) fn runtime(
        &self,
        binding: &ExecutionBinding,
    ) -> Result<Arc<dyn AgentRuntime>, String> {
        match &binding.connection {
            ExecutionConnection::Local => Ok(self.local_runtime.clone()),
            ExecutionConnection::Ssh { .. } => {
                let key = serde_json::to_string(binding).map_err(|e| e.to_string())?;
                let mut runtimes = self
                    .remote_runtimes
                    .lock()
                    .map_err(|_| "Execution endpoints are unavailable")?;
                if let Some(runtime) = runtimes.get(&key) {
                    return Ok(runtime.clone());
                }
                let runtime =
                    Arc::new(RemoteRuntime::connect(binding.clone()).map_err(|e| e.to_string())?);
                runtimes.insert(key, runtime.clone());
                Ok(runtime)
            }
        }
    }
    pub(crate) fn shutdown(&self) -> Result<(), String> {
        for runtime in self
            .remote_runtimes
            .lock()
            .map_err(|_| "Execution endpoints are unavailable")?
            .values()
        {
            let _ = runtime.shutdown();
        }
        Ok(())
    }
}
