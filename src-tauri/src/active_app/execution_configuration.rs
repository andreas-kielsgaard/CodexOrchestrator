//! Provider registration and the Capability Profile catalogue. Product skill roots are composed here
//! once and offered with every provider configuration.
use crate::{
    agent_sessions::application::SessionWorkspaces, execution_configuration::*,
    runtime::providers::{codex::profiles::NativeProfileService, registrations::ProviderRegistrations},
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(super) fn compose(
    database: Arc<crate::persistence::ActiveDatabase>,
    profiles: Arc<NativeProfileService>,
    workspaces: &SessionWorkspaces,
    product_tools: BTreeMap<String, BTreeSet<String>>,
    otp_skill_roots: BTreeMap<String, Vec<String>>,
) -> Result<
    (
        Arc<CapabilityProfileService>,
        Arc<crate::execution_targets::endpoints::ExecutionEndpoints>,
        Arc<ProductSkillRoots>,
    ),
    String,
> {
    let product_skills = Arc::new(ProductSkillRoots::new(
        vec![workspaces.skills_root().into()],
        otp_skill_roots
            .into_iter()
            .map(|(package, paths)| (package, paths.into_iter().map(Into::into).collect()))
            .collect(),
    ));
    let mut providers = ProviderRegistrations::default();
    crate::runtime::providers::codex::register(
        &mut providers,
        profiles,
        product_tools,
        &product_skills,
    )?;
    let endpoints = Arc::new(
        crate::execution_targets::endpoints::ExecutionEndpoints::new(providers)
            .with_local_sessions_directory(workspaces.sessions_directory()),
    );
    let service = Arc::new(
        CapabilityProfileService::new(Arc::new(SqliteCapabilityProfileRepository::from_database(
            database,
        )))
        .with_endpoints(endpoints.clone()),
    );
    Ok((service, endpoints, product_skills))
}
