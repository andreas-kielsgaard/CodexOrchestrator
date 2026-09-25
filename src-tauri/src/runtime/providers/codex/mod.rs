//! Codex CLI implementation of the provider-neutral Agent Runtime port.
//!
//! Argument/capability mapping, JSONL normalization, and process coordination are intentionally
//! separate. The process supervisor remains the sole owner of child processes.

pub(crate) mod app_server;
pub(crate) mod configuration;
pub(crate) mod continuation;
pub(crate) mod launch;
pub(crate) mod legacy_migration;
pub(crate) mod profiles;
#[cfg(test)]
mod arguments;
mod capabilities;
#[cfg(test)]
mod protocol;
#[cfg(test)]
mod runtime;

pub(crate) use capabilities::resolve_program;
#[cfg(all(test, feature = "live-tests"))]
pub(crate) use capabilities::{CodexCliCapabilities, CodexCliCapabilityProbe};
#[cfg(test)]
#[allow(unused_imports)]
pub(crate) use runtime::CodexCliRuntime;

#[cfg(test)]
mod tests;

use crate::runtime::providers::registrations::ProviderRegistrations;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Registers every Codex responsibility: app-server runtime, Codex home configuration discovery,
/// launch preparation, and native conversation transfer between homes and devices.
pub(crate) fn register(
    registrations: &mut ProviderRegistrations,
    profiles: Arc<profiles::NativeProfileService>,
    product_tools: BTreeMap<String, BTreeSet<String>>,
    product_skills: &crate::execution_configuration::ProductSkillRoots,
) -> Result<(), String> {
    let provider = orchid_engine::providers::codex::options::PROVIDER;
    let source = Arc::new(
        configuration::CodexConfigurationSource::new(profiles.clone(), product_tools)
            .with_product_skill_roots(product_skills),
    );
    registrations.runtimes.register(
        provider,
        Arc::new(app_server::CodexAppServerRuntime::system("codex")),
    )?;
    registrations
        .configurations
        .register(provider, source.clone())?;
    registrations
        .launches
        .register(provider, Arc::new(launch::CodexLaunchPreparation(profiles)))?;
    registrations.continuations.register(
        provider,
        Arc::new(continuation::CodexContinuationPort::new("codex", source)),
    )
}
