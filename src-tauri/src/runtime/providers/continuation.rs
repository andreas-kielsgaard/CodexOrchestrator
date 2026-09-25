use crate::agent_sessions::domain::ExternalRuntimeContextId;
use orchid_engine::contracts::provider::ProviderContinuationPayload;
use std::{collections::HashMap, sync::Arc};

pub(crate) trait ProviderContinuationPort: Send + Sync {
    fn export(
        &self,
        configuration_ref: &str,
        external_context_id: &ExternalRuntimeContextId,
    ) -> Result<ProviderContinuationPayload, String>;

    fn install(
        &self,
        configuration_ref: &str,
        continuation: &ProviderContinuationPayload,
    ) -> Result<(), String>;
}

#[derive(Default)]
pub(crate) struct ProviderContinuationRegistry {
    entries: HashMap<String, Arc<dyn ProviderContinuationPort>>,
}

impl ProviderContinuationRegistry {
    pub(crate) fn register(
        &mut self,
        provider: &str,
        port: Arc<dyn ProviderContinuationPort>,
    ) -> Result<(), String> {
        if provider.trim().is_empty() {
            return Err("Agent provider identity cannot be empty".into());
        }
        if self.entries.insert(provider.into(), port).is_some() {
            return Err(format!(
                "Agent provider `{provider}` already has a continuation implementation"
            ));
        }
        Ok(())
    }

    pub(crate) fn port(&self, provider: &str) -> Result<Arc<dyn ProviderContinuationPort>, String> {
        self.entries.get(provider).cloned().ok_or_else(|| {
            format!("Agent provider `{provider}` does not support native continuation transfer")
        })
    }
}
