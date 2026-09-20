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
    let service = Arc::new(
        CapabilityProfileService::new(
            Arc::new(SqliteCapabilityProfileRepository::from_database(database)),
            source.clone(),
        )
        .with_endpoints(endpoints.clone()),
    );
    Ok((source, service, endpoints))
}
