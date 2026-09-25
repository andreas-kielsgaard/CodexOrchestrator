//! The selected native source and the saved Capability Profile catalogue share one identity.
//! Product skill roots are composed here once and offered with every provider configuration.
use crate::{
    agent_sessions::application::SessionWorkspaces, execution_configuration::*,
    runtime::providers::codex::profiles::NativeProfileService,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(super) fn compose(
    database: Arc<crate::persistence::ActiveDatabase>,
    profiles: Arc<NativeProfileService>,
    workspaces: &SessionWorkspaces,
    local_runtime: Arc<dyn crate::agent_sessions::ports::AgentRuntime>,
    product_tools: BTreeMap<String, BTreeSet<String>>,
    otp_skill_roots: BTreeMap<String, Vec<String>>,
) -> Result<
    (
        Arc<dyn ProviderConfigurationSource>,
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
    let codex = Arc::new(
        CodexConfigurationSource::new(profiles, product_tools.clone())
            .with_product_skill_roots(&product_skills),
    );
    let source: Arc<dyn ProviderConfigurationSource> = codex.clone();
    let endpoints = Arc::new(
        crate::execution_targets::endpoints::ExecutionEndpoints::new(
            "codex",
            source.clone(),
            local_runtime,
        )?
        .with_local_sessions_directory(workspaces.sessions_directory())
        .with_continuation(
            "codex",
            Arc::new(
                crate::runtime::providers::codex::continuation::CodexContinuationPort::new(
                    "codex", codex,
                ),
            ),
        )?,
    );
    let service = Arc::new(
        CapabilityProfileService::new(
            Arc::new(SqliteCapabilityProfileRepository::from_database(database)),
            source.clone(),
        )
        .with_endpoints(endpoints.clone()),
    );
    Ok((source, service, endpoints, product_skills))
}
