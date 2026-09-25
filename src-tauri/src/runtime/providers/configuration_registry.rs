use crate::execution_configuration::ProviderConfigurationSource;
use std::{collections::HashMap, sync::Arc};

/// Provider-configuration discovery keyed independently from runtime dispatch.
#[derive(Default)]
pub(crate) struct ProviderConfigurationRegistry {
    entries: HashMap<String, Arc<dyn ProviderConfigurationSource>>,
}

impl ProviderConfigurationRegistry {
    pub(crate) fn one(
        provider: &str,
        source: Arc<dyn ProviderConfigurationSource>,
    ) -> Result<Self, String> {
        let mut registry = Self::default();
        registry.register(provider, source)?;
        Ok(registry)
    }

    pub(crate) fn register(
        &mut self,
        provider: &str,
        source: Arc<dyn ProviderConfigurationSource>,
    ) -> Result<(), String> {
        if provider.trim().is_empty() {
            return Err("Agent provider identity cannot be empty".into());
        }
        if self.entries.insert(provider.into(), source).is_some() {
            return Err(format!(
                "Agent provider `{provider}` already has a configuration source"
            ));
        }
        Ok(())
    }

    pub(crate) fn source(
        &self,
        provider: &str,
    ) -> Result<Arc<dyn ProviderConfigurationSource>, String> {
        self.entries.get(provider).cloned().ok_or_else(|| {
            format!("Agent provider `{provider}` is not registered for configuration discovery")
        })
    }
}
