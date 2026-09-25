use super::{domain::*, remote_runtime::RemoteRuntime, ssh_connection::SshConnection};
use crate::{
    agent_sessions::ports::AgentRuntime,
    execution_configuration::ProviderConfigurationSource,
    runtime::providers::{
        configuration_registry::ProviderConfigurationRegistry,
        continuation::ProviderContinuationRegistry,
        runtime_registry::ProviderRuntimeRegistry,
    },
};
use orchid_engine::protocol::{HostCommand, RuntimeCapabilities, WorktreeInstance};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub(crate) struct ExecutionEndpoints {
    pub(super) configurations: ProviderConfigurationRegistry,
    pub(super) continuations: ProviderContinuationRegistry,
    local_runtimes: ProviderRuntimeRegistry,
    remote_runtimes: Mutex<HashMap<String, Arc<dyn AgentRuntime>>>,
    /// Orchid-owned folder under which local auxiliary Session workspaces are created.
    pub(super) local_sessions_directory: Option<std::path::PathBuf>,
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
        provider: &str,
        local_source: Arc<dyn ProviderConfigurationSource>,
        local_runtime: Arc<dyn AgentRuntime>,
    ) -> Result<Self, String> {
        Ok(Self {
            configurations: ProviderConfigurationRegistry::one(provider, local_source)?,
            continuations: ProviderContinuationRegistry::default(),
            local_runtimes: ProviderRuntimeRegistry::one(provider, local_runtime)?,
            remote_runtimes: Default::default(),
            local_sessions_directory: None,
        })
    }
    pub(crate) fn with_local_sessions_directory(mut self, directory: std::path::PathBuf) -> Self {
        self.local_sessions_directory = Some(directory);
        self
    }
    pub(crate) fn with_continuation(
        mut self,
        provider: &str,
        port: Arc<dyn crate::runtime::providers::continuation::ProviderContinuationPort>,
    ) -> Result<Self, String> {
        self.continuations.register(provider, port)?;
        Ok(self)
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
                    .configurations
                    .source(&binding.provider)?
                    .profile_for_configuration(&binding.configuration_ref, cwd)
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
                        provider: binding.provider.clone(),
                        configuration_ref: binding.configuration_ref.clone(),
                        working_directory: cwd.map(str::to_owned),
                    })
                    .map_err(|e| e.to_string())?;
                Ok(ExecutionTargetRuntime {
                    runtime_profile: capabilities.profile,
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
                .configurations
                .source(&binding.provider)?
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
            ExecutionConnection::Local => self.local_runtimes.runtime(&binding.provider),
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
