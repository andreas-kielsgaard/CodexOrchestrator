//! Claude Code on this device: setups (a configuration folder and its CLI), what each setup
//! offers, and launch preparation. The runtime itself lives in the engine.
pub(crate) mod configuration;
pub(crate) mod launch;
pub(crate) mod setups;

use crate::runtime::providers::registrations::ProviderRegistrations;
use orchid_engine::providers::claude::{ClaudeRuntime, PROVIDER};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Registers Claude's runtime, configuration discovery and launch preparation. Claude has no
/// continuation port: a conversation that moves to another device restarts from the session log.
pub(crate) fn register(
    registrations: &mut ProviderRegistrations,
    setups: Arc<setups::ClaudeSetups>,
    product_tools: BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    registrations
        .runtimes
        .register(PROVIDER, Arc::new(ClaudeRuntime::system("claude")))?;
    registrations.configurations.register(
        PROVIDER,
        Arc::new(configuration::ClaudeConfigurationSource::new(
            setups.clone(),
            product_tools,
        )),
    )?;
    registrations
        .launches
        .register(PROVIDER, Arc::new(launch::ClaudeLaunchPreparation(setups)))
}
