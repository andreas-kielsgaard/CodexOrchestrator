//! Session composition. Startup reconciliation is sequenced by active_app after tool registration.
use crate::{
    agent_sessions::{
        application::{
            AgentSessionApplication, AgentSessionNotifier, SessionWorkspaces,
            SystemAgentSessionProviders,
        },
        repository::SqliteAgentSessionRepository,
    },
    execution_configuration::{CapabilityProfileService, SelectedRuntimeProfileSource},
    harness_engine::{catalog_service::HarnessCatalogService, HarnessEngineService},
    native_profiles::NativeProfileService,
};
use std::{path::Path, sync::Arc};

pub(super) struct SessionServices {
    pub(super) application: Arc<AgentSessionApplication>,
    pub(super) imports: Arc<crate::agent_sessions::application::import::AgentSessionImportService>,
    pub(super) selected_runtime_profile: Arc<dyn SelectedRuntimeProfileSource>,
    pub(super) capability_profiles: Arc<CapabilityProfileService>,
}

pub(super) fn compose(
    database: Arc<crate::persistence::ActiveDatabase>,
    database_path: &Path,
    native_profiles: Arc<NativeProfileService>,
    repository: Arc<SqliteAgentSessionRepository>,
    harness_catalog: HarnessCatalogService,
    harness_engine: Arc<HarnessEngineService>,
    notifier: Arc<dyn AgentSessionNotifier>,
) -> Result<SessionServices, String> {
    let workspaces = SessionWorkspaces::system(database_path.to_string_lossy().into_owned())?;
    let (selected_runtime_profile, capability_profiles) =
        super::execution_configuration::compose(database, native_profiles.clone(), &workspaces)?;
    let providers = Arc::new(SystemAgentSessionProviders);
    let application = Arc::new(
        AgentSessionApplication::new(
            repository.clone(),
            Arc::new(crate::runtime::codex::app_server::CodexAppServerRuntime::system("codex")),
            notifier,
            providers.clone(),
            providers,
            None,
        )
        .with_profile_source(selected_runtime_profile.clone())
        .with_capability_profiles(capability_profiles.clone())
        .with_workspaces(workspaces)
        .with_native_profile_launch_authority(native_profiles.clone())
        .with_session_harness_version_resolver(Arc::new(harness_catalog))
        .with_session_harness_launch_authority(Arc::new(
            crate::harness_engine::session_binding::SessionProfileHarnessAuthority {
                engine: harness_engine,
                sessions: repository.clone(),
            },
        )),
    );
    let imports = Arc::new(
        crate::agent_sessions::application::import::AgentSessionImportService {
            application: application.clone(),
            homes: native_profiles,
            history: Arc::new(
                crate::runtime::codex::app_server::history::CodexHistoryReader("codex".into()),
            ),
            store: repository,
            lane: std::sync::Mutex::new(()),
        },
    );
    Ok(SessionServices {
        imports,
        application,
        selected_runtime_profile,
        capability_profiles,
    })
}
