//! Session composition. Startup reconciliation is sequenced by active_app after tool registration.
use crate::{
    agent_sessions::{
        application::{
            AgentSessionApplication, AgentSessionNotifier, SessionWorkspaces,
            SystemAgentSessionProviders,
        },
        repository::SqliteAgentSessionRepository,
    },
    execution_configuration::CapabilityProfileService,
    harness_engine::{catalog_service::HarnessCatalogService, HarnessEngineService},
    runtime::providers::codex::profiles::NativeProfileService,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

pub(super) struct SessionServices {
    pub(super) application: Arc<AgentSessionApplication>,
    pub(super) imports: Arc<crate::agent_sessions::application::import::AgentSessionImportService>,
    pub(super) capability_profiles: Arc<CapabilityProfileService>,
    pub(super) execution_targets: Arc<crate::execution_targets::ExecutionTargetService>,
}

pub(super) fn compose(
    database: Arc<crate::persistence::ActiveDatabase>,
    database_path: &Path,
    native_profiles: Arc<NativeProfileService>,
    repository: Arc<SqliteAgentSessionRepository>,
    harness_catalog: HarnessCatalogService,
    harness_engine: Arc<HarnessEngineService>,
    notifier: Arc<dyn AgentSessionNotifier>,
    product_tools: BTreeMap<String, BTreeSet<String>>,
    otp_skill_roots: BTreeMap<String, Vec<String>>,
) -> Result<SessionServices, String> {
    let workspaces = SessionWorkspaces::system(database_path.to_string_lossy().into_owned())?;
    let (capability_profiles, endpoints, product_skills) =
        super::execution_configuration::compose(
            database.clone(),
            native_profiles.clone(),
            &workspaces,
            product_tools,
            otp_skill_roots,
        )?;
    // Sessions route to their provider through the endpoints; the default route's runtime serves
    // only unprofiled legacy Sessions.
    let default_runtime = endpoints.local_runtime(
        &crate::execution_targets::domain::ExecutionBinding::default().provider,
    )?;
    let execution_targets = Arc::new(crate::execution_targets::ExecutionTargetService::new(
        database,
        endpoints.clone(),
        capability_profiles.clone(),
    ));
    let providers = Arc::new(SystemAgentSessionProviders);
    let application = Arc::new(
        AgentSessionApplication::new(
            repository.clone(),
            default_runtime,
            notifier,
            providers.clone(),
            providers,
            None,
        )
        .with_product_skills(product_skills)
        .with_capability_profiles(capability_profiles.clone())
        .with_execution_endpoints(endpoints.clone())
        .with_execution_target_service(execution_targets.clone())
        .with_workspaces(workspaces)
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
                crate::runtime::providers::codex::app_server::history::CodexHistoryReader("codex".into()),
            ),
            store: repository,
            lane: std::sync::Mutex::new(()),
        },
    );
    Ok(SessionServices {
        imports,
        application,
        capability_profiles,
        execution_targets,
    })
}
