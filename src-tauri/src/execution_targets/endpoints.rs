use super::{domain::*, remote_runtime::RemoteRuntime, ssh_connection::SshConnection};
use crate::{
    agent_sessions::{application::ProviderLaunchPreparation, ports::AgentRuntime},
    execution_configuration::ProviderConfigurationSource,
    runtime::providers::registrations::ProviderRegistrations,
};
use orchid_engine::protocol::{HostCommand, RuntimeCapabilities, WorktreeInstance};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

/// The app-wide router from an instance's execution binding, or a provider identity, to the
/// provider implementations that serve it. Local bindings use the registered provider parts;
/// SSH bindings use a remote runtime client per binding.
pub(crate) struct ExecutionEndpoints {
    pub(super) providers: ProviderRegistrations,
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
    pub(crate) fn new(providers: ProviderRegistrations) -> Self {
        Self {
            providers,
            remote_runtimes: Default::default(),
            local_sessions_directory: None,
        }
    }
    #[cfg(test)]
    pub(crate) fn providers(&self) -> &ProviderRegistrations {
        &self.providers
    }
    /// The same endpoints with different provider registrations, for test composition.
    #[cfg(test)]
    pub(crate) fn with_providers(&self, providers: ProviderRegistrations) -> Self {
        Self {
            providers,
            remote_runtimes: std::sync::Mutex::new(self.remote_runtimes.lock().unwrap().clone()),
            local_sessions_directory: self.local_sessions_directory.clone(),
        }
    }
    pub(crate) fn with_local_sessions_directory(mut self, directory: std::path::PathBuf) -> Self {
        self.local_sessions_directory = Some(directory);
        self
    }
    pub(crate) fn configuration_source(
        &self,
        provider: &str,
    ) -> Result<Arc<dyn ProviderConfigurationSource>, String> {
        self.providers.configurations.get(provider)
    }
    /// The provider's native launch preparation, if it has one.
    pub(crate) fn launch_preparation(
        &self,
        provider: &str,
    ) -> Option<Arc<dyn ProviderLaunchPreparation>> {
        self.providers.launches.find(provider)
    }
    /// Every registered provider's setups on this device.
    pub(crate) fn provider_setups(
        &self,
    ) -> Result<Vec<crate::execution_configuration::ProviderSetup>, String> {
        let mut setups = Vec::new();
        for source in self.providers.configurations.values() {
            setups.extend(source.setups().map_err(|error| error.to_string())?);
        }
        setups.sort_by(|left, right| {
            (&left.device_id, &left.provider, &left.configuration_id)
                .cmp(&(&right.device_id, &right.provider, &right.configuration_id))
        });
        Ok(setups)
    }
    /// Whether the provider can transfer its native conversations between routes.
    pub(crate) fn supports_continuation(&self, provider: &str) -> bool {
        self.providers.continuations.find(provider).is_some()
    }
    pub(crate) fn local_runtime(&self, provider: &str) -> Result<Arc<dyn AgentRuntime>, String> {
        self.providers.runtimes.get(provider)
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
                    .configuration_source(&binding.provider)?
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
                .configuration_source(&binding.provider)?
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
            ExecutionConnection::Local => self.local_runtime(&binding.provider),
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
        for runtime in self.providers.runtimes.values() {
            let _ = runtime.shutdown();
        }
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
