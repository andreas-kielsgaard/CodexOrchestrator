use super::contracts::validate_model;
use super::PROVIDER;
use crate::execution_configuration::{
    CapabilitySet, ProviderConfigurationSource, ProviderConfigurationSourceError,
    RuntimeProfileSnapshot, RuntimeSelections,
};
use orchid_engine::contracts::ProviderConfigurationRef;

pub struct JevConfigurationSource;

impl ProviderConfigurationSource for JevConfigurationSource {
    fn profile_for_configuration(
        &self,
        reference: &str,
        _: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError> {
        validate_model(reference).map_err(ProviderConfigurationSourceError::unavailable)?;
        Ok(RuntimeProfileSnapshot {
            contract_version: 1,
            configuration: ProviderConfigurationRef::new(PROVIDER, reference),
            exposure: CapabilitySet {
                models: [reference.into()].into_iter().collect(),
                ..Default::default()
            },
            locked: RuntimeSelections {
                model: Some(reference.into()),
                ..Default::default()
            },
            provider_options: None,
        })
    }
}
