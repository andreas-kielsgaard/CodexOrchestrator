use crate::agent_sessions::domain::ExternalRuntimeContextId;
use orchid_engine::contracts::provider::ProviderContinuationPayload;

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
