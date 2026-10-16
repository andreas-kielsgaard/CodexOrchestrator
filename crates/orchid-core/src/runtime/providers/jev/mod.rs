pub mod configuration;
mod credentials;
pub use credentials::{
    remove as remove_credential, save as save_credential, status as credential_status,
    CredentialStatus,
};
pub use orchid_engine::providers::jev::{contracts, JevRuntime, DEFAULT_MODEL, PROVIDER};

pub fn register(providers: &mut super::registrations::ProviderRegistrations) -> Result<(), String> {
    providers.register(
        PROVIDER,
        super::registrations::ProviderRegistration::new(
            std::sync::Arc::new(configuration::JevConfigurationSource),
            std::sync::Arc::new(JevRuntime::system_with_credential(std::sync::Arc::new(
                credentials::resolve,
            ))),
            None,
        ),
    )
}

#[cfg(test)]
mod tests;
