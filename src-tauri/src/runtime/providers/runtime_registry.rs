use crate::agent_sessions::ports::AgentRuntime;
use std::{collections::HashMap, sync::Arc};

/// Runtime implementations keyed by stable Agent provider identity.
///
/// This registry owns dispatch only. Provider options, configuration discovery, history, and
/// continuation use their own contracts and registries.
#[derive(Default)]
pub(crate) struct ProviderRuntimeRegistry {
    entries: HashMap<String, Arc<dyn AgentRuntime>>,
}

impl ProviderRuntimeRegistry {
    pub(crate) fn one(provider: &str, runtime: Arc<dyn AgentRuntime>) -> Result<Self, String> {
        let mut registry = Self::default();
        registry.register(provider, runtime)?;
        Ok(registry)
    }

    pub(crate) fn register(
        &mut self,
        provider: &str,
        runtime: Arc<dyn AgentRuntime>,
    ) -> Result<(), String> {
        validate_provider(provider)?;
        if self.entries.insert(provider.into(), runtime).is_some() {
            return Err(format!("Agent provider `{provider}` already has a runtime"));
        }
        Ok(())
    }

    pub(crate) fn runtime(&self, provider: &str) -> Result<Arc<dyn AgentRuntime>, String> {
        self.entries
            .get(provider)
            .cloned()
            .ok_or_else(|| format!("Agent provider `{provider}` is not registered for execution"))
    }
}

fn validate_provider(provider: &str) -> Result<(), String> {
    if provider.trim().is_empty() {
        Err("Agent provider identity cannot be empty".into())
    } else {
        Ok(())
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
        let mut registry = ProviderRuntimeRegistry::one("codex", codex.clone()).unwrap();
        registry.register("test-provider", other.clone()).unwrap();

        assert!(Arc::ptr_eq(&registry.runtime("codex").unwrap(), &codex));
        assert!(Arc::ptr_eq(&registry.runtime("test-provider").unwrap(), &other));
        assert!(registry.runtime("unregistered").err().unwrap().contains("not registered"));
    }
}
