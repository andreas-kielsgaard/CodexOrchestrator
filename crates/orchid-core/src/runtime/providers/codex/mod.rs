//! Codex CLI implementation of the provider-neutral Agent Runtime port.
//!
//! Argument/capability mapping, JSONL normalization, and process coordination are intentionally
//! separate. The process supervisor remains the sole owner of child processes.

pub mod app_server;
#[cfg(any(test, feature = "test-support"))]
mod arguments;
mod capabilities;
pub mod configuration;
pub mod launch;
pub mod legacy_migration;
pub mod profiles;
#[cfg(any(test, feature = "test-support"))]
mod protocol;
#[cfg(any(test, feature = "test-support"))]
mod runtime;

pub use capabilities::resolve_program;
#[cfg(all(test, feature = "live-tests"))]
pub use capabilities::{CodexCliCapabilities, CodexCliCapabilityProbe};
#[cfg(any(test, feature = "test-support"))]
#[allow(unused_imports)]
pub use runtime::CodexCliRuntime;

#[cfg(any(test, feature = "test-support"))]
mod tests;

use crate::runtime::providers::registrations::{ProviderRegistration, ProviderRegistrations};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

/// Registers every Codex responsibility: app-server runtime, Codex home configuration discovery,
/// and launch preparation.
pub fn register(
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
    registrations.register(
        provider,
        ProviderRegistration::new(
            source,
            Arc::new(app_server::CodexAppServerRuntime::system("codex")),
            Some(Arc::new(launch::CodexLaunchPreparation(profiles))),
        ),
    )
}
