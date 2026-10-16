//! Claude Code on this device: setups (a configuration folder and its CLI), what each setup
//! offers, and launch preparation. The runtime itself lives in the engine.
pub mod configuration;
pub mod launch;
pub mod setups;

use crate::runtime::providers::registrations::{ProviderRegistration, ProviderRegistrations};
use orchid_engine::providers::claude::{ClaudeRuntime, PROVIDER};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Registers Claude's runtime, configuration discovery, and launch preparation.
pub fn register(
    registrations: &mut ProviderRegistrations,
    setups: Arc<setups::ClaudeSetups>,
    product_tools: BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    let configuration = Arc::new(configuration::ClaudeConfigurationSource::new(
        setups.clone(),
        product_tools,
    ));
    registrations.register(
        PROVIDER,
        ProviderRegistration::new(
            configuration,
            Arc::new(ClaudeRuntime::system("claude")),
            Some(Arc::new(launch::ClaudeLaunchPreparation(setups))),
        ),
    )
}
