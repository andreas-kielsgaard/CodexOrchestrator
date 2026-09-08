use super::{
    domain::{
        binding_digest, HarnessBindingRecord, HarnessBindingStage, HarnessMcpExposurePlan,
        HarnessMediationPlan, HarnessToolAccess, ManagedMcpUpstreamDescriptor,
        SidecarBindingRegistration, MEDIATION_PLAN_VERSION,
    },
    proxy::proxy_url,
    repository::{HarnessBindingRepository, SqliteHarnessBindingRepository},
    sidecar::{HarnessSidecarClient, ProcessHarnessSidecar},
};
use crate::agent_sessions::{
    application::SessionHarnessLaunchAuthority,
    domain::{AgentInvocationId, AgentSessionId},
    ports::RuntimeLaunchExtension,
};
use chrono::Utc;
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Default)]
pub(crate) struct ManagedMcpUpstreamRegistry {
    descriptors: Mutex<BTreeMap<String, Vec<ManagedMcpUpstreamRegistration>>>,
}

struct ManagedMcpUpstreamRegistration {
    id: String,
    descriptor: ManagedMcpUpstreamDescriptor,
    owner: Option<Box<dyn ManagedMcpUpstreamOwner>>,
}

pub(crate) trait ManagedMcpUpstreamOwner: Send {
    fn stop(self: Box<Self>);
}

impl ManagedMcpUpstreamRegistry {
    pub(crate) fn register(
        &self,
        descriptor: ManagedMcpUpstreamDescriptor,
    ) -> Result<String, String> {
        let name = descriptor.name.trim();
        if name.is_empty() || descriptor.url.trim().is_empty() {
            return Err("Managed MCP upstream name and URL are required.".to_string());
        }
        let registration_id = format!("managed-mcp-upstream-{}", Uuid::new_v4());
        let mut descriptors = self
            .descriptors
            .lock()
            .map_err(|_| "Managed MCP upstream registry is unavailable.".to_string())?;
        descriptors
            .entry(name.to_string())
            .or_default()
            .push(ManagedMcpUpstreamRegistration {
                id: registration_id.clone(),
                descriptor,
                owner: None,
            });
        Ok(registration_id)
    }

    pub(crate) fn retain_owner(
        &self,
        registration_id: &str,
        owner: Box<dyn ManagedMcpUpstreamOwner>,
    ) -> Result<(), Box<dyn ManagedMcpUpstreamOwner>> {
        let Ok(mut descriptors) = self.descriptors.lock() else {
            return Err(owner);
        };
        for registrations in descriptors.values_mut() {
            if let Some(registration) = registrations
                .iter_mut()
                .find(|registration| registration.id == registration_id)
            {
                registration.owner = Some(owner);
                return Ok(());
            }
        }
        Err(owner)
    }

    pub(crate) fn unregister(&self, registration_id: &str) {
        if let Ok(mut descriptors) = self.descriptors.lock() {
            let mut empty_name = None;
            for (name, registrations) in descriptors.iter_mut() {
                registrations.retain(|registration| registration.id != registration_id);
                if registrations.is_empty() {
                    empty_name = Some(name.clone());
                    break;
                }
            }
            if let Some(name) = empty_name {
                descriptors.remove(&name);
            }
        }
    }

    fn resolve(&self, name: &str) -> Result<ManagedMcpUpstreamDescriptor, String> {
        self.descriptors
            .lock()
            .map_err(|_| "Managed MCP upstream registry is unavailable.".to_string())?
            .get(name)
            .and_then(|registrations| registrations.last())
            .map(|registration| registration.descriptor.clone())
            .ok_or_else(|| {
                format!(
                    "Harness MCP server {name} has no application-owned managed upstream. Ambient Codex MCP configuration is never inherited."
                )
            })
    }

    pub(crate) fn shutdown(&self) {
        let owners = self
            .descriptors
            .lock()
            .map(|mut descriptors| {
                descriptors
                    .values_mut()
                    .flat_map(|registrations| registrations.iter_mut())
                    .filter_map(|registration| registration.owner.take())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for owner in owners {
            owner.stop();
        }
    }
}

pub(crate) struct HarnessEngineService {
    repository: Arc<dyn HarnessBindingRepository>,
    sidecar: Arc<dyn HarnessSidecarClient>,
    upstreams: Arc<ManagedMcpUpstreamRegistry>,
}

impl HarnessEngineService {
    pub(crate) fn bind_session_profile(
        &self,
        session_id: &AgentSessionId,
        profile: &crate::execution_configuration::SessionCreationResolution,
    ) -> Result<(), String> {
        profile.verify_digest().map_err(|error| error.to_string())?;
        let snapshot = serde_json::to_string(profile).map_err(|error| error.to_string())?;
        if let Some(binding) = self.repository.current_for_session(session_id.as_str())? {
            binding.verify_digest()?;
            if binding.harness_snapshot != snapshot {
                return Err("Session MCP binding does not match its pinned profile".into());
            }
            return Ok(());
        }
        let mut exposures = Vec::new();
        for (index, (server, tools)) in profile
            .session_profile()
            .node_capabilities()
            .mcp_tools
            .iter()
            .enumerate()
        {
            if tools.is_empty() {
                continue;
            }
            exposures.push(HarnessMcpExposurePlan {
                configured_server_name: server.clone(),
                proxy_server_name: format!("session_capability_{}", index + 1),
                upstream: self.upstreams.resolve(server)?,
                access: HarnessToolAccess::SelectedTools {
                    tool_names: tools.iter().cloned().collect(),
                },
            });
        }
        let mediation_plan = serde_json::to_string(&HarnessMediationPlan {
            contract_version: MEDIATION_PLAN_VERSION.into(),
            exposures,
        })
        .map_err(|error| error.to_string())?;
        // Keep the existing technical storage/proxy format. Legacy provenance columns are
        // unused here; source addressing is derived from the trusted Session at the receiver.
        let binding = HarnessBindingRecord {
            id: format!("session-capability-binding-{}", Uuid::new_v4()),
            session_id: session_id.as_str().into(),
            runtime_instance_id: profile.session_profile().runtime_profile_ref().into(),
            session_instance_token: Uuid::new_v4().simple().to_string(),
            stage: HarnessBindingStage::Prepared,
            configuration_digest: binding_digest(&snapshot, &mediation_plan),
            harness_snapshot: snapshot,
            mediation_plan,
            harness_token: None,
            source_workflow_instance_id: String::new(),
            source_recipe_id: String::new(),
            source_node_id: String::new(),
            prepared_at: Utc::now().to_rfc3339(),
            bound_at: None,
            retired_at: None,
        };
        self.repository.insert_prepared(&binding)?;
        self.complete_prepared_binding(&binding)?;
        Ok(())
    }

    pub(crate) fn open_system(
        database_path: &Path,
        upstreams: Arc<ManagedMcpUpstreamRegistry>,
    ) -> Result<Arc<Self>, String> {
        let repository = Arc::new(SqliteHarnessBindingRepository::open(database_path)?);
        let sidecar = ProcessHarnessSidecar::start_system()?;
        Self::new(repository, sidecar, upstreams)
    }

    pub(crate) fn new(
        repository: Arc<dyn HarnessBindingRepository>,
        sidecar: Arc<dyn HarnessSidecarClient>,
        upstreams: Arc<ManagedMcpUpstreamRegistry>,
    ) -> Result<Arc<Self>, String> {
        let service = Arc::new(Self {
            repository,
            sidecar,
            upstreams,
        });
        Ok(service)
    }

    fn proxy_extension(
        &self,
        binding: &HarnessBindingRecord,
        mut extension: RuntimeLaunchExtension,
    ) -> Result<RuntimeLaunchExtension, String> {
        reject_caller_mcp_configuration(&extension)?;
        let token = binding
            .harness_token
            .as_deref()
            .ok_or_else(|| "Bound Harness has no Harness token.".to_string())?;
        let plan = binding.parsed_plan()?;
        let address = self.sidecar.proxy_address()?;
        extension
            .additional_args
            .extend(["-c".to_string(), "mcp_servers={}".to_string()]);
        for (index, exposure) in plan.exposures.iter().enumerate() {
            let name = &exposure.proxy_server_name;
            let url = proxy_url(address, token, index);
            for value in [
                format!(
                    "mcp_servers.{name}.url={}",
                    serde_json::to_string(&url).expect("proxy URL serializes")
                ),
                format!("mcp_servers.{name}.required=true"),
                format!("mcp_servers.{name}.default_tools_approval_mode=\"approve\""),
                format!("mcp_servers.{name}.startup_timeout_sec=10"),
                format!("mcp_servers.{name}.tool_timeout_sec=300"),
            ] {
                extension.additional_args.push("-c".to_string());
                extension.additional_args.push(value);
            }
        }
        Ok(extension)
    }

    pub(crate) fn shutdown(&self) -> Result<(), String> {
        let result = self.sidecar.shutdown();
        self.upstreams.shutdown();
        result
    }

    fn complete_prepared_binding(
        &self,
        binding: &HarnessBindingRecord,
    ) -> Result<HarnessBindingRecord, String> {
        let token = self
            .sidecar
            .register_binding(SidecarBindingRegistration::from_record(binding)?)?;
        match self
            .repository
            .mark_bound(&binding.id, &token, &Utc::now().to_rfc3339())
        {
            Ok(bound) => Ok(bound),
            Err(error) => {
                let _ = self.sidecar.retire_binding(&binding.id);
                Err(error)
            }
        }
    }
}

impl SessionHarnessLaunchAuthority for HarnessEngineService {
    fn prepare_launch(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<Option<RuntimeLaunchExtension>, String> {
        let Some(mut binding) = self.repository.current_for_session(session_id.as_str())? else {
            return Ok(extension);
        };
        if binding.stage == HarnessBindingStage::Prepared {
            binding = self.complete_prepared_binding(&binding)?;
        }
        if binding.stage != HarnessBindingStage::Bound {
            return Err("The Session Harness binding is not available for invocation.".to_string());
        }
        binding.verify_digest()?;
        let registration = SidecarBindingRegistration::from_record(&binding)?;
        self.sidecar.ensure_binding(registration)?;
        self.sidecar
            .prepare_invocation(&binding.id, invocation_id.as_str())?;
        self.proxy_extension(&binding, extension.unwrap_or_default())
            .map(Some)
    }
}

fn reject_caller_mcp_configuration(extension: &RuntimeLaunchExtension) -> Result<(), String> {
    if extension
        .additional_args
        .iter()
        .any(|argument| argument.to_ascii_lowercase().contains("mcp_servers"))
    {
        return Err(
            "A Harness-bound Session cannot accept caller-supplied MCP configuration.".to_string(),
        );
    }
    Ok(())
}

pub(crate) struct HarnessEngineTauriState {
    service: Arc<HarnessEngineService>,
}

impl HarnessEngineTauriState {
    pub(crate) fn new(service: Arc<HarnessEngineService>) -> Self {
        Self { service }
    }

    pub(crate) fn service(&self) -> &Arc<HarnessEngineService> {
        &self.service
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[derive(Default)]
    struct FakeSidecar {
        registrations: Mutex<Vec<SidecarBindingRegistration>>,
        failed_registrations: Mutex<usize>,
        retired: Mutex<Vec<String>>,
        prepared_invocations: Mutex<Vec<(String, String)>>,
    }

    impl HarnessSidecarClient for FakeSidecar {
        fn register_binding(
            &self,
            mut registration: SidecarBindingRegistration,
        ) -> Result<String, String> {
            let mut failures = self.failed_registrations.lock().unwrap();
            if *failures > 0 {
                *failures -= 1;
                return Err("sidecar registration failed".to_string());
            }
            drop(failures);
            let token = registration
                .harness_token
                .clone()
                .unwrap_or_else(|| "stable-harness-token".to_string());
            registration.harness_token = Some(token.clone());
            self.registrations.lock().unwrap().push(registration);
            Ok(token)
        }

        fn ensure_binding(&self, registration: SidecarBindingRegistration) -> Result<(), String> {
            self.registrations.lock().unwrap().push(registration);
            Ok(())
        }

        fn retire_binding(&self, binding_id: &str) -> Result<(), String> {
            self.retired.lock().unwrap().push(binding_id.to_string());
            Ok(())
        }

        fn prepare_invocation(&self, binding_id: &str, invocation_id: &str) -> Result<(), String> {
            self.prepared_invocations
                .lock()
                .unwrap()
                .push((binding_id.to_string(), invocation_id.to_string()));
            Ok(())
        }

        fn proxy_address(&self) -> Result<std::net::SocketAddr, String> {
            Ok("127.0.0.1:43123".parse().unwrap())
        }

        fn shutdown(&self) -> Result<(), String> {
            Ok(())
        }
    }

    struct FailingMarkBoundRepository {
        inner: Arc<SqliteHarnessBindingRepository>,
        fail_once: AtomicBool,
    }

    impl HarnessBindingRepository for FailingMarkBoundRepository {
        fn insert_prepared(&self, binding: &HarnessBindingRecord) -> Result<(), String> {
            self.inner.insert_prepared(binding)
        }

        fn mark_bound(
            &self,
            binding_id: &str,
            harness_token: &str,
            bound_at: &str,
        ) -> Result<HarnessBindingRecord, String> {
            if self.fail_once.swap(false, Ordering::SeqCst) {
                return Err("simulated bound persistence failure".to_string());
            }
            self.inner.mark_bound(binding_id, harness_token, bound_at)
        }

        fn current_for_session(
            &self,
            session_id: &str,
        ) -> Result<Option<HarnessBindingRecord>, String> {
            self.inner.current_for_session(session_id)
        }

        fn non_retired(&self) -> Result<Vec<HarnessBindingRecord>, String> {
            self.inner.non_retired()
        }

        fn retire(&self, binding_id: &str, retired_at: &str) -> Result<(), String> {
            self.inner.retire(binding_id, retired_at)
        }
    }

    fn repository() -> (tempfile::TempDir, Arc<SqliteHarnessBindingRepository>) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bindings.sqlite");
        let connection = crate::storage::open_active_database(&path).unwrap();
        connection
            .execute(
                "INSERT INTO agent_sessions(id,title,availability,external_context_id,runtime_version,working_directory,requested_options_json,created_at,updated_at) VALUES('session-1','Session','available',NULL,NULL,NULL,'{}','t','t')",
                [],
            )
            .unwrap();
        drop(connection);
        (
            directory,
            Arc::new(SqliteHarnessBindingRepository::open(&path).unwrap()),
        )
    }

    fn profile(server: Option<&str>) -> crate::execution_configuration::SessionCreationResolution {
        use crate::execution_configuration::*;
        let capabilities = CapabilitySet {
            mcp_tools: server
                .map(|s| {
                    [(
                        s.into(),
                        ["submit_epic_plan_proposal".into()].into_iter().collect(),
                    )]
                    .into_iter()
                    .collect()
                })
                .unwrap_or_default(),
            ..CapabilitySet::default()
        };
        SessionProfileResolver::resolve_snapshot(
            RuntimeProfileSnapshot {
                contract_version: 1,
                profile_ref: "runtime".into(),
                exposure: capabilities.clone(),
                locked: RuntimeSelections::default(),
            },
            SessionCreationRequest {
                contract_version: 1,
                capability_profile: CapabilityProfile {
                    contract_version: 1,
                    capability_profile_id: "profile".into(),
                    name: "Test".into(),
                    revision: 1,
                    allowed_capabilities: capabilities.clone(),
                },
                node_profile: NodeProfile {
                    contract_version: 1,
                    allowed_capabilities: capabilities,
                    pinned_defaults: RuntimeSelections::default(),
                },
            },
        )
        .unwrap()
    }

    #[test]
    fn binding_is_persisted_then_bound_and_launch_receives_only_proxy_configuration() {
        let (_directory, repository) = repository();
        let sidecar = Arc::new(FakeSidecar::default());
        let registry = Arc::new(ManagedMcpUpstreamRegistry::default());
        registry
            .register(ManagedMcpUpstreamDescriptor {
                name: "plan_builder".into(),
                url: "http://127.0.0.1:48000/mcp".into(),
                bearer_token: "upstream-secret".into(),
                caller_context: false,
            })
            .unwrap();
        let service =
            HarnessEngineService::new(repository.clone(), sidecar.clone(), registry).unwrap();
        service
            .bind_session_profile(
                &AgentSessionId::new("session-1").unwrap(),
                &profile(Some("plan_builder")),
            )
            .unwrap();
        let binding = repository
            .current_for_session("session-1")
            .unwrap()
            .unwrap();
        assert_eq!(binding.stage, HarnessBindingStage::Bound);
        assert_eq!(
            binding.harness_token.as_deref(),
            Some("stable-harness-token")
        );
        assert_eq!(binding.source_workflow_instance_id, "");
        let bound: crate::execution_configuration::SessionCreationResolution =
            serde_json::from_str(&binding.harness_snapshot).unwrap();
        bound.verify_digest().unwrap();
        assert_eq!(bound, profile(Some("plan_builder")));
        let extension = service
            .prepare_launch(
                &AgentSessionId::new("session-1").unwrap(),
                &AgentInvocationId::new("invocation-1").unwrap(),
                None,
            )
            .unwrap()
            .unwrap();
        let joined = extension.additional_args.join(" ");
        assert!(joined.contains("mcp_servers={}"));
        assert!(joined.contains("stable-harness-token"));
        assert!(!joined.contains("48000"));
        assert!(!joined.contains("upstream-secret"));
        assert!(extension.environment.is_empty());
        assert_eq!(
            sidecar.prepared_invocations.lock().unwrap()[0].1,
            "invocation-1"
        );
    }

    #[test]
    fn unknown_upstream_fails_visibly_instead_of_using_ambient_codex_configuration() {
        let (_directory, repository) = repository();
        let service = HarnessEngineService::new(
            repository,
            Arc::new(FakeSidecar::default()),
            Arc::new(ManagedMcpUpstreamRegistry::default()),
        )
        .unwrap();
        let error = service
            .bind_session_profile(
                &AgentSessionId::new("session-1").unwrap(),
                &profile(Some("arbitrary_server")),
            )
            .unwrap_err();
        assert!(error.contains("no application-owned managed upstream"));
        assert!(error.contains("never inherited"));
    }

    #[test]
    fn bound_session_rejects_direct_caller_mcp_configuration() {
        let extension = RuntimeLaunchExtension {
            additional_args: vec!["-c".into(), "mcp_servers.attacker.url=\"http://x\"".into()],
            ..RuntimeLaunchExtension::default()
        };
        assert!(reject_caller_mcp_configuration(&extension)
            .unwrap_err()
            .contains("caller-supplied"));
    }

    #[test]
    fn managed_upstream_registrations_are_lifecycle_scoped_without_limiting_concurrency() {
        let registry = ManagedMcpUpstreamRegistry::default();
        let first = registry
            .register(ManagedMcpUpstreamDescriptor {
                name: "plan_builder".into(),
                url: "http://127.0.0.1:41001/mcp".into(),
                bearer_token: "first".into(),
                caller_context: false,
            })
            .unwrap();
        let second = registry
            .register(ManagedMcpUpstreamDescriptor {
                name: "plan_builder".into(),
                url: "http://127.0.0.1:41002/mcp".into(),
                bearer_token: "second".into(),
                caller_context: false,
            })
            .unwrap();

        assert_eq!(
            registry.resolve("plan_builder").unwrap().bearer_token,
            "second"
        );
        registry.unregister(&second);
        assert_eq!(
            registry.resolve("plan_builder").unwrap().bearer_token,
            "first"
        );
        registry.unregister(&first);
        assert!(registry.resolve("plan_builder").is_err());
    }

    struct RecordingUpstreamOwner(Arc<AtomicBool>);

    impl ManagedMcpUpstreamOwner for RecordingUpstreamOwner {
        fn stop(self: Box<Self>) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[test]
    fn managed_upstream_owner_is_retained_until_application_registry_shutdown() {
        let registry = ManagedMcpUpstreamRegistry::default();
        let registration = registry
            .register(ManagedMcpUpstreamDescriptor {
                name: "plan_builder".into(),
                url: "http://127.0.0.1:41001/mcp".into(),
                bearer_token: "secret".into(),
                caller_context: false,
            })
            .unwrap();
        let stopped = Arc::new(AtomicBool::new(false));
        assert!(registry
            .retain_owner(
                &registration,
                Box::new(RecordingUpstreamOwner(stopped.clone())),
            )
            .is_ok());

        assert!(registry.resolve("plan_builder").is_ok());
        assert!(!stopped.load(Ordering::SeqCst));
        registry.shutdown();
        assert!(stopped.load(Ordering::SeqCst));
    }

    #[test]
    fn invocation_reconciles_a_prepared_binding_after_registration_failure() {
        let (_directory, repository) = repository();
        let sidecar = Arc::new(FakeSidecar::default());
        *sidecar.failed_registrations.lock().unwrap() = 1;
        let service = HarnessEngineService::new(
            repository.clone(),
            sidecar,
            Arc::new(ManagedMcpUpstreamRegistry::default()),
        )
        .unwrap();
        assert!(service
            .bind_session_profile(&AgentSessionId::new("session-1").unwrap(), &profile(None))
            .is_err());
        assert_eq!(
            repository
                .current_for_session("session-1")
                .unwrap()
                .unwrap()
                .stage,
            HarnessBindingStage::Prepared
        );

        service
            .prepare_launch(
                &AgentSessionId::new("session-1").unwrap(),
                &AgentInvocationId::new("invocation-2").unwrap(),
                None,
            )
            .unwrap();
        assert_eq!(
            repository
                .current_for_session("session-1")
                .unwrap()
                .unwrap()
                .stage,
            HarnessBindingStage::Bound
        );
    }

    #[test]
    fn bound_persistence_failure_retires_sidecar_registration_and_retries_prepared_bytes() {
        let (_directory, repository) = repository();
        let sidecar = Arc::new(FakeSidecar::default());
        let failing_repository = Arc::new(FailingMarkBoundRepository {
            inner: repository.clone(),
            fail_once: AtomicBool::new(true),
        });
        let service = HarnessEngineService::new(
            failing_repository,
            sidecar.clone(),
            Arc::new(ManagedMcpUpstreamRegistry::default()),
        )
        .unwrap();

        assert!(service
            .bind_session_profile(&AgentSessionId::new("session-1").unwrap(), &profile(None))
            .unwrap_err()
            .contains("persistence failure"));
        assert_eq!(sidecar.retired.lock().unwrap().len(), 1);
        assert_eq!(
            repository
                .current_for_session("session-1")
                .unwrap()
                .unwrap()
                .stage,
            HarnessBindingStage::Prepared
        );

        service
            .prepare_launch(
                &AgentSessionId::new("session-1").unwrap(),
                &AgentInvocationId::new("invocation-2").unwrap(),
                None,
            )
            .unwrap();
        assert_eq!(
            repository
                .current_for_session("session-1")
                .unwrap()
                .unwrap()
                .stage,
            HarnessBindingStage::Bound
        );
    }
}
