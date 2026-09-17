//! The selected native source and the saved Capability Profile catalogue share one identity.
use crate::{
    agent_sessions::application::SessionWorkspaces, execution_configuration::*,
    native_profiles::NativeProfileService,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

pub(super) fn compose(
    database: Arc<crate::persistence::ActiveDatabase>,
    profiles: Arc<NativeProfileService>,
    workspaces: &SessionWorkspaces,
    local_runtime: Arc<dyn crate::agent_sessions::ports::AgentRuntime>,
    product_tools: BTreeMap<String, BTreeSet<String>>,
) -> Result<
    (
        Arc<dyn SelectedRuntimeProfileSource>,
        Arc<CapabilityProfileService>,
        Arc<crate::execution_targets::endpoints::ExecutionEndpoints>,
    ),
    String,
> {
    let source: Arc<dyn SelectedRuntimeProfileSource> = Arc::new(
        NativeCodexSelectedRuntimeProfileSource::new(profiles, product_tools.clone())
            .with_skill_roots(vec![workspaces.skills_root()]),
    );
    let endpoints = Arc::new(
        crate::execution_targets::endpoints::ExecutionEndpoints::new(source.clone(), local_runtime),
    );
    let declared = Arc::new(ConfiguredRuntimeProfileSource::new(
        configured_runtime_profile(CapabilitySet {
            mcp_tools: product_tools,
            models: ["gpt-5.6-sol".to_string(), "gpt-5.6-terra".to_string()]
                .into_iter()
                .collect(),
            reasoning_modes: [
                "low".to_string(),
                "medium".to_string(),
                "high".to_string(),
                "xhigh".to_string(),
                "max".to_string(),
                "ultra".to_string(),
            ]
            .into_iter()
            .collect(),
            ..CapabilitySet::default()
        }),
    ));
    let service = Arc::new(CapabilityProfileService::new(
        Arc::new(SqliteCapabilityProfileRepository::from_database(database)),
        declared,
    ));
    Ok((source, service, endpoints))
}
