//! Per-provider implementations registered at composition. Each provider module supplies its parts
//! through one `register` function; shared code looks them up by the provider of the route it holds.
use crate::{
    agent_sessions::{application::ProviderLaunchPreparation, ports::AgentRuntime},
    execution_configuration::ProviderConfigurationSource,
    runtime::providers::continuation::ProviderContinuationPort,
};
use std::{collections::HashMap, sync::Arc};

/// One implementation of a provider responsibility per Agent provider. Lookups never fall back to
/// another provider.
pub(crate) struct ProviderMap<T: ?Sized> {
    entries: HashMap<String, Arc<T>>,
    /// Completes "Agent provider `x` …" when the provider has no entry.
    missing: &'static str,
}

impl<T: ?Sized> Clone for ProviderMap<T> {
    fn clone(&self) -> Self {
        Self {
            entries: self.entries.clone(),
            missing: self.missing,
        }
    }
}

impl<T: ?Sized> ProviderMap<T> {
    fn new(missing: &'static str) -> Self {
        Self {
            entries: HashMap::new(),
            missing,
        }
    }

    pub(crate) fn register(&mut self, provider: &str, entry: Arc<T>) -> Result<(), String> {
        if provider.trim().is_empty() {
            return Err("Agent provider identity cannot be empty".into());
        }
        if self.entries.insert(provider.into(), entry).is_some() {
            return Err(format!("Agent provider `{provider}` is registered twice"));
        }
        Ok(())
    }

    /// Test composition may swap a provider's part after composing it.
    #[cfg(test)]
    pub(crate) fn replace(&mut self, provider: &str, entry: Arc<T>) {
        self.entries.insert(provider.into(), entry);
    }

    pub(crate) fn get(&self, provider: &str) -> Result<Arc<T>, String> {
        self.entries
            .get(provider)
            .cloned()
            .ok_or_else(|| format!("Agent provider `{provider}` {}", self.missing))
    }

    pub(crate) fn find(&self, provider: &str) -> Option<Arc<T>> {
        self.entries.get(provider).cloned()
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &Arc<T>> {
        self.entries.values()
    }
}

#[derive(Clone)]
pub(crate) struct ProviderRegistrations {
    pub(crate) runtimes: ProviderMap<dyn AgentRuntime>,
    pub(crate) configurations: ProviderMap<dyn ProviderConfigurationSource>,
    /// Optional: a provider without native environment preparation launches with the extension
    /// as prepared by Orchid.
    pub(crate) launches: ProviderMap<dyn ProviderLaunchPreparation>,
    /// Optional: a provider without transfer starts a new native conversation from the Session log.
    pub(crate) continuations: ProviderMap<dyn ProviderContinuationPort>,
}

impl Default for ProviderRegistrations {
    fn default() -> Self {
        Self {
            runtimes: ProviderMap::new("is not registered for execution"),
            configurations: ProviderMap::new("is not registered for configuration discovery"),
            launches: ProviderMap::new("has no launch preparation"),
            continuations: ProviderMap::new("does not support native continuation transfer"),
        }
    }
}

impl ProviderRegistrations {
    /// A provider with only a runtime and a configuration source, as used by tests and fakes.
    #[cfg(test)]
    pub(crate) fn single(
        provider: &str,
        configuration: Arc<dyn ProviderConfigurationSource>,
        runtime: Arc<dyn AgentRuntime>,
    ) -> Self {
        let mut registrations = Self::default();
        registrations
            .configurations
            .register(provider, configuration)
            .expect("one configuration source");
        registrations
            .runtimes
            .register(provider, runtime)
            .expect("one runtime");
        registrations
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_sessions::{domain::*, ports::*};

    struct TestRuntime;
    impl AgentRuntime for TestRuntime {
        fn preflight_invocation(&self, _: RuntimeInvocationMode, options: &AgentRuntimeOptions) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
            Ok(RuntimeInvocationPreflight { effective_options: options.clone() })
        }
        fn start_invocation(&self, _: RuntimeInvocationRequest, _: Arc<dyn AgentRuntimeUpdateSink>) -> Result<(), RuntimePortError> { Ok(()) }
        fn resume_invocation(&self, _: RuntimeInvocationRequest, _: ExternalRuntimeContextId, _: Arc<dyn AgentRuntimeUpdateSink>) -> Result<(), RuntimePortError> { Ok(()) }
        fn cancel_invocation(&self, _: &AgentInvocationId) -> Result<(), RuntimePortError> { Ok(()) }
    }

    #[test]
    fn dispatches_distinct_providers_without_a_codex_fallback() {
        let codex: Arc<dyn AgentRuntime> = Arc::new(TestRuntime);
        let other: Arc<dyn AgentRuntime> = Arc::new(TestRuntime);
        let mut registrations = ProviderRegistrations::default();
        registrations.runtimes.register("codex", codex.clone()).unwrap();
        registrations.runtimes.register("test-provider", other.clone()).unwrap();

        assert!(Arc::ptr_eq(&registrations.runtimes.get("codex").unwrap(), &codex));
        assert!(Arc::ptr_eq(&registrations.runtimes.get("test-provider").unwrap(), &other));
        assert!(registrations.runtimes.get("unregistered").err().unwrap().contains("not registered"));
        assert!(registrations.runtimes.register("codex", codex).is_err());
    }
}
