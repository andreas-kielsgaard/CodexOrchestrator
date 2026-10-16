//! Persistent execution-device composition. No network connection owns this object's lifetime.
use super::sessions::{self, SessionServices};
use crate::{
    agent_sessions::{
        application::{AgentSessionNotification, AgentSessionNotifier},
        repository::SqliteAgentSessionRepository,
    },
    persistence::ActiveDatabase,
};
use std::{
    path::Path,
    sync::{Arc, Mutex, Weak},
};

#[derive(Default)]
struct HostNotifications(
    Mutex<Option<Weak<crate::workflows::execution::WorkflowExecutionService>>>,
);
impl AgentSessionNotifier for HostNotifications {
    fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
        let workflow = self
            .0
            .lock()
            .map_err(|_| "Workflow observer unavailable")?
            .as_ref()
            .and_then(Weak::upgrade);
        if let Some(workflow) = workflow {
            workflow
                .on_agent_notification(&notification)
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

pub struct DeviceHost {
    _lease: crate::instance::InstanceLease,
    profiles: Arc<crate::runtime::providers::codex::profiles::NativeProfileService>,
    claude: Arc<crate::runtime::providers::claude::setups::ClaudeSetups>,
    pub database: Arc<ActiveDatabase>,
    pub sessions: SessionServices,
    pub repository: Arc<SqliteAgentSessionRepository>,
    pub workflows: Arc<crate::workflows::execution::WorkflowExecutionService>,
    pub harness: Arc<crate::harness_engine::HarnessEngineService>,
}
impl DeviceHost {
    pub fn open(root: &Path) -> Result<Self, String> {
        let lease = crate::instance::InstanceLease::acquire(root, "host")?;
        std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let database = crate::product_database::open(&root.join("orchid.sqlite"))?;
        let profiles = Arc::new(
            crate::runtime::providers::codex::profiles::NativeProfileService::new(
                database.clone(),
                root.into(),
            ),
        );
        let claude = Arc::new(
            crate::runtime::providers::claude::setups::ClaudeSetups::system(database.clone()),
        );
        let repository = Arc::new(SqliteAgentSessionRepository::from_database(
            database.clone(),
        ));
        let upstreams = Arc::new(crate::harness_engine::ManagedMcpUpstreamRegistry::default());
        let otp = crate::otp_host::OtpRegistry::import(&["workflow", "job_agent"])?;
        let installations =
            crate::otp_host::installations::OtpInstallationService::from_database(database.clone());
        let harness = crate::harness_engine::HarnessEngineService::open_system(
            database.clone(),
            upstreams.clone(),
        )?;
        harness.attach_agent_mcp_provisioner(
            crate::otp_host::job_agent::JobAgentMcpProvisioner::new(otp.clone(), installations),
        )?;
        harness.attach_agent_mcp_provisioner(
            crate::execution_configuration::SessionSkillReaderProvisioner::start()?,
        )?;
        let observer = Arc::new(HostNotifications::default());
        let updates = crate::agent_cli_updates::AgentCliUpdateService::system();
        // Service installations manage their CLI versions outside invocation preparation.
        updates.mark_controller_managed();
        let sessions = sessions::compose(
            database.clone(),
            crate::agent_sessions::workspace::SessionWorkspaces::new(
                root.join("content"),
                root.to_string_lossy().into_owned(),
            )?,
            profiles.clone(),
            claude.clone(),
            repository.clone(),
            crate::harness_engine::catalog_service::HarnessCatalogService::from_database(
                database.clone(),
            ),
            harness.clone(),
            observer.clone(),
            {
                let mut tools = otp.mcp_tools();
                tools.extend(crate::orchid_control::tool_map());
                tools
            },
            otp.catalogue()
                .into_iter()
                .filter(|package| !package.skill_roots.is_empty())
                .map(|package| (package.id, package.skill_roots))
                .collect(),
            updates,
        )?;
        let identities =
            crate::identities::service::IdentityService::from_database(database.clone());
        let adapter = Arc::new(
            crate::agent_sessions::session_event_adapter::AgentSessionEventAdapter::new(
                sessions.application.clone(),
                repository.clone(),
                identities,
            )
            .with_capability_profiles(sessions.capability_profiles.clone()),
        );
        let events = Arc::new(crate::session_events::SessionEventApplication::new(
            adapter.clone(),
            adapter.clone(),
            Arc::new(
                crate::session_events::SqliteSessionEventStore::from_database(database.clone()),
            ),
        ));
        let authoring=Arc::new(crate::workflows::authoring_service::WorkflowAuthoringService::new(Arc::new(crate::workflows::authoring_repository::SqliteWorkflowAuthoringRepository::from_database(database.clone())),sessions.capability_profiles.clone(),otp.clone()));
        let workflows = Arc::new(
            crate::workflows::execution::WorkflowExecutionService::new(
                authoring,
                events,
                Arc::new(
                    crate::workflows::instances::WorkflowInstanceStore::from_database(
                        database.clone(),
                    ),
                ),
                adapter,
                repository.clone(),
            )
            .with_session_control(Arc::new(
                crate::otp_host::session_control::AgentSessionControl {
                    application: sessions.application.clone(),
                    repository: repository.clone(),
                },
            )),
        );
        *observer
            .0
            .lock()
            .map_err(|_| "Workflow observer unavailable")? = Some(Arc::downgrade(&workflows));
        let (descriptors, owner) =
            crate::otp_host::mcp::start_server(otp, Arc::downgrade(&workflows))?;
        let mut descriptors = descriptors.into_iter();
        let first = descriptors.next().ok_or("No OTP endpoints")?;
        let registration = upstreams.register(first)?;
        if let Err(owner) = upstreams.retain_owner(&registration, owner) {
            owner.stop();
            return Err("Unable to retain OTP endpoint".into());
        }
        for descriptor in descriptors {
            upstreams.register(descriptor)?;
        }
        let control = Arc::new(crate::orchid_control::OrchidControl {
            agents: sessions.application.clone(),
            profiles: sessions.capability_profiles.clone(),
            repositories: Arc::new(crate::repository_catalog::RepositoryCatalog::new(
                database.clone(),
            )),
        });
        let (descriptors, owner) = crate::orchid_control::mcp::start(control, None)?;
        let mut descriptors = descriptors.into_iter();
        let registration =
            upstreams.register(descriptors.next().ok_or("No Orchid control endpoint")?)?;
        if let Err(owner) = upstreams.retain_owner(&registration, owner) {
            owner.stop();
            return Err("Unable to retain Orchid control endpoint".into());
        }
        for descriptor in descriptors {
            upstreams.register(descriptor)?;
        }
        sessions
            .application
            .reconcile_startup()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            _lease: lease,
            database,
            sessions,
            repository,
            workflows,
            harness,
            profiles,
            claude,
        })
    }
    /// Machine-local provider setup. Credentials remain in the provider's own home.
    pub fn register_provider(
        &self,
        provider: &str,
        home: &str,
        executable: Option<String>,
    ) -> Result<serde_json::Value, String> {
        let id = match provider {
            "codex" => serde_json::to_value(self.profiles.register_existing(home)?)
                .map_err(|e| e.to_string())?["id"]
                .as_str()
                .ok_or("Provider registration did not return an identity")?
                .to_owned(),
            "claude" => self.claude.add(Path::new(home), executable)?.id,
            _ => {
                return Err(
                    "This provider uses environment configuration rather than a native home".into(),
                )
            }
        };
        Ok(serde_json::json!({"provider":provider,"configurationRef":id}))
    }
    pub fn shutdown(&self) -> Result<(), String> {
        self.sessions
            .application
            .shutdown_runtime()
            .map_err(|e| e.to_string())?;
        self.harness.shutdown()
    }
}
