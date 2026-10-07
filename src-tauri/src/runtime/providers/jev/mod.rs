pub(crate) mod configuration;
pub(crate) use orchid_engine::providers::jev::{contracts, JevRuntime, DEFAULT_MODEL, PROVIDER};

pub(crate) fn register(
    providers: &mut super::registrations::ProviderRegistrations,
) -> Result<(), String> {
    providers.register(
        PROVIDER,
        super::registrations::ProviderRegistration::new(
            std::sync::Arc::new(configuration::JevConfigurationSource),
            std::sync::Arc::new(JevRuntime::system()),
            None,
        ),
    )
}

#[cfg(test)]
mod tests;
