//! The selected native source and the saved Capability Profile catalogue share one identity.
use crate::{
    agent_sessions::application::SessionWorkspaces, execution_configuration::*,
    native_profiles::NativeProfileService,
};
use std::sync::Arc;

pub(super) fn compose(
    database: Arc<crate::persistence::ActiveDatabase>,
    profiles: Arc<NativeProfileService>,
    workspaces: &SessionWorkspaces,
) -> Result<
    (
        Arc<dyn SelectedRuntimeProfileSource>,
        Arc<CapabilityProfileService>,
    ),
    String,
> {
    let source: Arc<dyn SelectedRuntimeProfileSource> = Arc::new(
        NativeCodexSelectedRuntimeProfileSource::new(
            profiles,
            [(
                crate::workflows::mcp::SERVER_NAME.to_string(),
                [crate::workflows::mcp::TOOL_NAME.to_string()]
                    .into_iter()
                    .collect(),
            )]
            .into_iter()
            .collect(),
        )
        .with_skill_roots(vec![workspaces.skills_root()]),
    );
    let service = Arc::new(CapabilityProfileService::new(
        Arc::new(SqliteCapabilityProfileRepository::from_database(database)),
        source.clone(),
    ));
    Ok((source, service))
}
