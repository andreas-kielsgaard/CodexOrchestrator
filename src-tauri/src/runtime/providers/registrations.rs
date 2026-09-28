//! One discoverable registration bundle per Agent provider.
use crate::{
    agent_sessions::{application::ProviderLaunchPreparation, ports::AgentRuntime},
    execution_configuration::ProviderConfigurationSource,
};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone)]
pub(crate) struct ProviderRegistration {
    configuration: Arc<dyn ProviderConfigurationSource>,
    runtime: Arc<dyn AgentRuntime>,
    launch: Option<Arc<dyn ProviderLaunchPreparation>>,
}

impl ProviderRegistration {
    pub(crate) fn new(
        configuration: Arc<dyn ProviderConfigurationSource>,
        runtime: Arc<dyn AgentRuntime>,
        launch: Option<Arc<dyn ProviderLaunchPreparation>>,
    ) -> Self {
        Self {
            configuration,
            runtime,
            launch,
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct ProviderRegistrations {
    entries: HashMap<String, ProviderRegistration>,
}

impl ProviderRegistrations {
    pub(crate) fn register(
        &mut self,
        provider: &str,
        registration: ProviderRegistration,
    ) -> Result<(), String> {
        if provider.trim().is_empty() {
            return Err("Agent provider identity cannot be empty".into());
        }
        if self.entries.insert(provider.into(), registration).is_some() {
            return Err(format!("Agent provider `{provider}` is registered twice"));
        }
        Ok(())
    }

    pub(crate) fn configuration(
        &self,
        provider: &str,
    ) -> Result<Arc<dyn ProviderConfigurationSource>, String> {
        self.entries
            .get(provider)
            .map(|registration| registration.configuration.clone())
            .ok_or_else(|| format!("Agent provider `{provider}` is not registered"))
    }

    pub(crate) fn runtime(&self, provider: &str) -> Result<Arc<dyn AgentRuntime>, String> {
        self.entries
            .get(provider)
            .map(|registration| registration.runtime.clone())
            .ok_or_else(|| format!("Agent provider `{provider}` is not registered for execution"))
    }

    pub(crate) fn launch(&self, provider: &str) -> Option<Arc<dyn ProviderLaunchPreparation>> {
        self.entries
            .get(provider)
            .and_then(|registration| registration.launch.clone())
    }

    pub(crate) fn registrations(&self) -> impl Iterator<Item = &ProviderRegistration> {
        self.entries.values()
    }

    pub(crate) fn configuration_source(
        registration: &ProviderRegistration,
    ) -> &Arc<dyn ProviderConfigurationSource> {
        &registration.configuration
    }

    pub(crate) fn registered_runtimes(&self) -> impl Iterator<Item = &Arc<dyn AgentRuntime>> {
        self.entries
            .values()
            .map(|registration| &registration.runtime)
    }

    #[cfg(test)]
    pub(crate) fn single(
        provider: &str,
        configuration: Arc<dyn ProviderConfigurationSource>,
        runtime: Arc<dyn AgentRuntime>,
    ) -> Self {
        let mut registrations = Self::default();
        registrations
            .register(
                provider,
                ProviderRegistration::new(configuration, runtime, None),
            )
            .expect("one provider registration");
        registrations
    }

    #[cfg(test)]
    pub(crate) fn configuration_only(
        provider: &str,
        configuration: Arc<dyn ProviderConfigurationSource>,
    ) -> Self {
        struct UnavailableRuntime;
        impl AgentRuntime for UnavailableRuntime {
            fn preflight_invocation(
                &self,
                _: crate::agent_sessions::ports::RuntimeInvocationMode,
                _: &crate::agent_sessions::domain::AgentRuntimeOptions,
            ) -> Result<
                crate::agent_sessions::ports::RuntimeInvocationPreflight,
                crate::agent_sessions::ports::RuntimePortError,
            > {
                Err(crate::agent_sessions::ports::RuntimePortError::new(
                    crate::agent_sessions::ports::RuntimePortErrorKind::Unavailable,
                    "Test runtime is unavailable",
                ))
            }
            fn start_invocation(
                &self,
                _: crate::agent_sessions::ports::RuntimeInvocationRequest,
                _: Arc<dyn crate::agent_sessions::ports::AgentRuntimeUpdateSink>,
            ) -> Result<(), crate::agent_sessions::ports::RuntimePortError> {
                unreachable!()
            }
            fn resume_invocation(
                &self,
                _: crate::agent_sessions::ports::RuntimeInvocationRequest,
                _: crate::agent_sessions::domain::ExternalRuntimeContextId,
                _: Arc<dyn crate::agent_sessions::ports::AgentRuntimeUpdateSink>,
            ) -> Result<(), crate::agent_sessions::ports::RuntimePortError> {
                unreachable!()
            }
            fn cancel_invocation(
                &self,
                _: &crate::agent_sessions::domain::AgentInvocationId,
            ) -> Result<(), crate::agent_sessions::ports::RuntimePortError> {
                unreachable!()
            }
        }
        Self::single(provider, configuration, Arc::new(UnavailableRuntime))
    }

    #[cfg(test)]
    pub(crate) fn replace_configuration(
        &mut self,
        provider: &str,
        source: Arc<dyn ProviderConfigurationSource>,
    ) -> Result<(), String> {
        self.entries
            .get_mut(provider)
            .ok_or_else(|| format!("Agent provider `{provider}` is not registered"))?
            .configuration = source;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn replace_runtime(
        &mut self,
        provider: &str,
        runtime: Arc<dyn AgentRuntime>,
    ) -> Result<(), String> {
        self.entries
            .get_mut(provider)
            .ok_or_else(|| format!("Agent provider `{provider}` is not registered"))?
            .runtime = runtime;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn replace_launch(
        &mut self,
        provider: &str,
        launch: Arc<dyn ProviderLaunchPreparation>,
    ) -> Result<(), String> {
        self.entries
            .get_mut(provider)
            .ok_or_else(|| format!("Agent provider `{provider}` is not registered"))?
            .launch = Some(launch);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        agent_sessions::{domain::*, ports::*},
        execution_configuration::{
            NativeCapabilityInventory, ProviderConfigurationSourceError, RuntimeProfileSnapshot,
        },
    };

    struct TestRuntime;
    impl AgentRuntime for TestRuntime {
        fn preflight_invocation(
            &self,
            _: RuntimeInvocationMode,
            options: &AgentRuntimeOptions,
        ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
            Ok(RuntimeInvocationPreflight {
                effective_options: options.clone(),
            })
        }
        fn start_invocation(
            &self,
            _: RuntimeInvocationRequest,
            _: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            Ok(())
        }
        fn resume_invocation(
            &self,
            _: RuntimeInvocationRequest,
            _: ExternalRuntimeContextId,
            _: Arc<dyn AgentRuntimeUpdateSink>,
        ) -> Result<(), RuntimePortError> {
            Ok(())
        }
        fn cancel_invocation(&self, _: &AgentInvocationId) -> Result<(), RuntimePortError> {
            Ok(())
        }
    }

    struct TestConfiguration;
    impl ProviderConfigurationSource for TestConfiguration {
        fn profile_for_configuration(
            &self,
            _: &str,
            _: Option<&str>,
        ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError> {
            Err(ProviderConfigurationSourceError::unavailable("unused"))
        }
        fn inventory_for_configuration(
            &self,
            _: &str,
            _: Option<&str>,
        ) -> Result<NativeCapabilityInventory, ProviderConfigurationSourceError> {
            Ok(Default::default())
        }
    }

    #[test]
    fn dispatches_complete_provider_bundles_without_fallback() {
        let mut registrations = ProviderRegistrations::default();
        let runtime: Arc<dyn AgentRuntime> = Arc::new(TestRuntime);
        registrations
            .register(
                "test-provider",
                ProviderRegistration::new(Arc::new(TestConfiguration), runtime.clone(), None),
            )
            .unwrap();
        assert!(Arc::ptr_eq(
            &registrations.runtime("test-provider").unwrap(),
            &runtime
        ));
        assert!(registrations.runtime("unregistered").is_err());
        assert!(registrations
            .register(
                "test-provider",
                ProviderRegistration::new(Arc::new(TestConfiguration), runtime, None),
            )
            .is_err());
    }
}
