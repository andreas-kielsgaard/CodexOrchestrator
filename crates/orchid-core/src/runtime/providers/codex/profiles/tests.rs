use super::*;
use crate::execution_configuration::{ProviderConfigurationSource, SandboxMode};
use crate::runtime::providers::codex::configuration::CodexConfigurationSource;
use std::sync::Barrier;
use std::thread;

struct FakeChild {
    result: Option<NativeCliReceipt>,
    terminated: Arc<Mutex<usize>>,
}
impl NativeCliChild for FakeChild {
    fn try_wait(&mut self) -> Result<Option<NativeCliReceipt>, String> {
        Ok(self.result.take())
    }
    fn terminate(&mut self) -> Result<(), String> {
        self.result = Some(NativeCliReceipt {
            succeeded: false,
            exit_code: None,
            sandbox_receipt_observed: false,
        });
        *self
            .terminated
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) += 1;
        Ok(())
    }
}
struct FakeCli {
    receipt: NativeCliReceipt,
    calls: Mutex<Vec<NativeCliInvocation>>,
    starts: Mutex<usize>,
    terminated: Arc<Mutex<usize>>,
    next_child_result: Mutex<Option<NativeCliReceipt>>,
    start_error: bool,
    surface_supported: bool,
    workspace_sandbox_supported: bool,
    windows_sandbox_setup_supported: bool,
    workspace_launch_flags_supported: bool,
    workspace_launch_project_config_isolated: bool,
    danger_unrestricted_network_supported: bool,
}
impl FakeCli {
    fn succeeding() -> Self {
        Self {
            receipt: NativeCliReceipt {
                succeeded: true,
                exit_code: Some(0),
                sandbox_receipt_observed: true,
            },
            calls: Mutex::new(vec![]),
            starts: Mutex::new(0),
            terminated: Arc::new(Mutex::new(0)),
            next_child_result: Mutex::new(None),
            start_error: false,
            surface_supported: true,
            workspace_sandbox_supported: true,
            windows_sandbox_setup_supported: true,
            workspace_launch_flags_supported: true,
            workspace_launch_project_config_isolated: true,
            danger_unrestricted_network_supported: true,
        }
    }

    fn unsupported_surface() -> Self {
        Self {
            surface_supported: false,
            ..Self::succeeding()
        }
    }

    fn unsupported_workspace_policy() -> Self {
        Self {
            workspace_sandbox_supported: false,
            ..Self::succeeding()
        }
    }

    fn unsupported_windows_sandbox_setup() -> Self {
        Self {
            windows_sandbox_setup_supported: false,
            ..Self::succeeding()
        }
    }

    fn unsupported_danger_launch_surface() -> Self {
        Self {
            danger_unrestricted_network_supported: false,
            ..Self::succeeding()
        }
    }

    fn failing_start() -> Self {
        Self {
            start_error: true,
            ..Self::succeeding()
        }
    }
}
impl NativeCliPort for FakeCli {
    fn run(&self, invocation: &NativeCliInvocation) -> Result<NativeCliReceipt, String> {
        self.calls.lock().unwrap().push(invocation.clone());
        Ok(self.receipt)
    }
    fn start(&self, invocation: &NativeCliInvocation) -> Result<Box<dyn NativeCliChild>, String> {
        self.calls.lock().unwrap().push(invocation.clone());
        *self.starts.lock().unwrap() += 1;
        if self.start_error {
            return Err("launch failed".into());
        }
        Ok(Box::new(FakeChild {
            result: self.next_child_result.lock().unwrap().take(),
            terminated: self.terminated.clone(),
        }))
    }
    fn surface(&self) -> Result<NativeCliSurface, String> {
        if !self.surface_supported {
            return Err("unsupported".into());
        }
        Ok(NativeCliSurface {
            provenance: NativeCliProvenance {
                executable: "C:/application-owned/codex.exe".into(),
                version: "codex-cli test".into(),
                workspace_sandbox_supported: self.workspace_sandbox_supported,
                danger_full_access_supported: true,
                danger_unrestricted_network_supported: self.danger_unrestricted_network_supported,
                non_interactive_approval_supported: true,
            },
            windows_sandbox_setup_supported: self.windows_sandbox_setup_supported,
            workspace_launch_flags_supported: self.workspace_launch_flags_supported,
            workspace_launch_project_config_isolated: self.workspace_launch_project_config_isolated,
        })
    }
}

fn service() -> (tempfile::TempDir, NativeProfileService) {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    service.cli = Arc::new(FakeCli::succeeding());
    (directory, service)
}

struct ReportingCli {
    calls: Mutex<Vec<NativeCliInvocation>>,
}

impl ReportingCli {
    fn new() -> Self {
        Self {
            calls: Mutex::new(vec![]),
        }
    }

    fn call_reporting_tool(invocation: &NativeCliInvocation) -> Result<(), String> {
        let configured = |suffix: &str| {
            invocation.args.windows(2).find_map(|pair| {
                (pair[0] == "-c")
                    .then(|| pair[1].strip_prefix(suffix).map(str::to_owned))
                    .flatten()
            })
        };
        let endpoint = configured(&format!("mcp_servers.{MCP_REPORTING_SERVER}.url="))
            .and_then(|value| serde_json::from_str::<String>(&value).ok())
            .ok_or("missing MCP endpoint")?;
        let variable = configured(&format!(
            "mcp_servers.{MCP_REPORTING_SERVER}.bearer_token_env_var="
        ))
        .and_then(|value| serde_json::from_str::<String>(&value).ok())
        .ok_or("missing MCP bearer variable")?;
        let bearer = invocation
            .environment
            .iter()
            .find_map(|(key, value)| (key == &variable).then_some(value.clone()))
            .ok_or("missing MCP bearer")?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .map_err(|_| "test MCP runtime unavailable")?;
        runtime.block_on(async move {
                let client = reqwest::Client::new();
                let initialize = client
                    .post(&endpoint)
                    .header("content-type", "application/json")
                    .header("accept", "application/json, text/event-stream")
                    .header("authorization", format!("Bearer {bearer}"))
                    .body(serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"native-profile-test","version":"1"}}}).to_string())
                    .send()
                    .await
                    .map_err(|_| "test MCP initialize unavailable")?;
                let session = initialize
                    .headers()
                    .get("mcp-session-id")
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned)
                    .ok_or("test MCP session missing")?;
                if !initialize.status().is_success() {
                    return Err("test MCP initialize rejected".into());
                }
                let notification = client
                    .post(&endpoint)
                    .header("content-type", "application/json")
                    .header("accept", "application/json, text/event-stream")
                    .header("authorization", format!("Bearer {bearer}"))
                    .header("mcp-session-id", &session)
                    .body("{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}")
                    .send()
                    .await
                    .map_err(|_| "test MCP notification unavailable")?;
                if !notification.status().is_success() {
                    return Err("test MCP notification rejected".into());
                }
                let call = client
                    .post(&endpoint)
                    .header("content-type", "application/json")
                    .header("accept", "application/json, text/event-stream")
                    .header("authorization", format!("Bearer {bearer}"))
                    .header("mcp-session-id", session)
                    .body(serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":MCP_REPORTING_TOOL,"arguments":{}}}).to_string())
                    .send()
                    .await
                    .map_err(|_| "test MCP tool unavailable")?;
                call.status()
                    .is_success()
                    .then_some(())
                    .ok_or_else(|| "test MCP tool rejected".into())
            })
    }
}

impl NativeCliPort for ReportingCli {
    fn run(&self, invocation: &NativeCliInvocation) -> Result<NativeCliReceipt, String> {
        self.calls.lock().unwrap().push(invocation.clone());
        Self::call_reporting_tool(invocation)?;
        Ok(NativeCliReceipt {
            succeeded: true,
            exit_code: Some(0),
            sandbox_receipt_observed: false,
        })
    }

    fn start(&self, _: &NativeCliInvocation) -> Result<Box<dyn NativeCliChild>, String> {
        Err("not used by MCP reporting".into())
    }

    fn surface(&self) -> Result<NativeCliSurface, String> {
        FakeCli::succeeding().surface()
    }
}

fn selected_profile_ready_except_mcp(service: &NativeProfileService) -> NativeProfileDto {
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .update_readiness(
            &profile.id,
            Some("authenticated"),
            Some("initialized"),
            Some("passed"),
            Some("not_assessed"),
            None,
        )
        .unwrap();
    profile
}

fn mark_mcp_ready(service: &NativeProfileService, profile_id: &str) {
    service
        .connection()
        .unwrap()
        .execute(
            "UPDATE native_codex_profile_readiness SET mcp_reporting='ready' WHERE profile_id=?1",
            params![profile_id],
        )
        .unwrap();
}

#[test]
fn selected_profile_adapter_uses_native_exposure_without_product_sandbox_locks() {
    struct Environment;
    impl crate::runtime::providers::codex::app_server::environment::CodexEnvironmentSource
        for Environment
    {
        fn read(
            &self,
            _: PathBuf,
            _: Option<PathBuf>,
        ) -> Result<
            crate::runtime::providers::codex::app_server::environment::CodexEnvironment,
            crate::agent_sessions::ports::RuntimePortError,
        > {
            Ok(
                crate::runtime::providers::codex::app_server::environment::CodexEnvironment {
                    models: serde_json::json!([{"model":"native-model","supportedReasoningEfforts":[{"reasoningEffort":"high"}]}]),
                    skills: serde_json::json!({"data":[]}),
                    config: serde_json::json!({}),
                    requirements: serde_json::json!({}),
                },
            )
        }
    }
    let (_directory, service) = service();
    let profile = selected_profile_ready_except_mcp(&service);
    mark_mcp_ready(&service, &profile.id);
    let source = CodexConfigurationSource::new(Arc::new(service), Default::default())
        .with_reader(Arc::new(Environment));

    let snapshot = source.profile_for_configuration("selected", None).unwrap();
    assert_eq!(
        snapshot.configuration,
        orchid_engine::contracts::ProviderConfigurationRef::new("codex", profile.id.clone())
    );
    assert_eq!(
        snapshot.exposure.sandbox_modes,
        [SandboxMode::DangerFullAccess].into_iter().collect()
    );
    assert_eq!(
        snapshot.locked.sandbox_mode,
        Some(SandboxMode::DangerFullAccess)
    );
    assert!(snapshot.exposure.models.contains("native-model"));
    let encoded = serde_json::to_value(snapshot).unwrap();
    assert!(encoded.get("home").is_none());
    assert!(encoded.get("filesystemIdentity").is_none());
}

#[test]
fn skill_discovery_uses_each_registered_home() {
    struct Environment(Arc<Mutex<Vec<PathBuf>>>);
    impl crate::runtime::providers::codex::app_server::environment::CodexEnvironmentSource
        for Environment
    {
        fn read(
            &self,
            _: PathBuf,
            _: Option<PathBuf>,
        ) -> Result<
            crate::runtime::providers::codex::app_server::environment::CodexEnvironment,
            crate::agent_sessions::ports::RuntimePortError,
        > {
            unreachable!("Skill discovery must not load the full runtime environment")
        }
        fn discover_skills(
            &self,
            home: PathBuf,
            _: Option<PathBuf>,
        ) -> Result<
            orchid_engine::contracts::ProviderSkillCatalogue,
            crate::agent_sessions::ports::RuntimePortError,
        > {
            self.0.lock().unwrap().push(home);
            Ok(Default::default())
        }
    }
    let (_directory, service) = service();
    let first = selected_profile_ready_except_mcp(&service);
    let second = service.create_dedicated().unwrap();
    let observed = Arc::new(Mutex::new(Vec::new()));
    let source = CodexConfigurationSource::new(Arc::new(service), Default::default())
        .with_reader(Arc::new(Environment(observed.clone())));
    source
        .native_skills_for_configuration(&first.id, None)
        .unwrap();
    source
        .native_skills_for_configuration(&second.id, None)
        .unwrap();
    let homes = observed.lock().unwrap();
    assert_eq!(homes.len(), 2);
    assert_ne!(homes[0], homes[1]);
}

#[test]
fn quick_features_cache_by_profile_and_context_until_explicit_refresh() {
    struct Environment(Arc<Mutex<usize>>);
    impl crate::runtime::providers::codex::app_server::environment::CodexEnvironmentSource
        for Environment
    {
        fn read(
            &self,
            _: PathBuf,
            _: Option<PathBuf>,
        ) -> Result<
            crate::runtime::providers::codex::app_server::environment::CodexEnvironment,
            crate::agent_sessions::ports::RuntimePortError,
        > {
            *self.0.lock().unwrap() += 1;
            Ok(
                crate::runtime::providers::codex::app_server::environment::CodexEnvironment {
                    models: serde_json::json!([{"model":"native-model","supportedReasoningEfforts":[]}]),
                    skills: serde_json::json!({"data":[]}),
                    config: serde_json::json!({}),
                    requirements: serde_json::json!({}),
                },
            )
        }
    }
    let (_directory, service) = service();
    let profile = selected_profile_ready_except_mcp(&service);
    let reads = Arc::new(Mutex::new(0));
    let source = CodexConfigurationSource::new(Arc::new(service), Default::default())
        .with_reader(Arc::new(Environment(reads.clone())));

    source
        .quick_features_for_configuration(&profile.id, Some("C:/repo"))
        .unwrap();
    source
        .quick_features_for_configuration(&profile.id, Some("C:\\repo"))
        .unwrap();
    assert_eq!(*reads.lock().unwrap(), 1);
    source
        .refresh_quick_features_for_configuration(&profile.id, Some("C:/repo"))
        .unwrap();
    assert_eq!(*reads.lock().unwrap(), 2);
}

fn claim_mcp_reporting_for_current_reconciliation(
    service: &NativeProfileService,
    id: &str,
) -> NativeMcpReportingProbeAuthority {
    let claimed = service
        .claim_pending_mcp_reporting_probe(id)
        .unwrap()
        .expect("pending reporting request is claimed");
    service
        .mcp_dispatch_claims
        .lock()
        .unwrap()
        .insert(id.into(), claimed.claim_id);
    claimed.authority
}

#[test]
fn creates_selects_and_reopens_a_dedicated_profile_without_provider_state() {
    let (directory, service) = service();
    let created = service.create_dedicated().unwrap();
    assert_eq!(created.ownership, Ownership::ApplicationDedicated);
    let selected = service.select(&created.id).unwrap();
    assert!(selected.selected);
    drop(service);
    let reopened = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let query = reopened.query().unwrap();
    assert_eq!(query.contract, PROFILE_QUERY_CONTRACT);
    assert_eq!(query.profiles.len(), 1);
    assert!(query.profiles[0].selected);
    assert!(reopened.resolve_selected_home().is_err());
}

#[test]
fn managed_agent_session_binding_revalidates_selection_and_records_no_raw_home() {
    let (directory, service) = service();
    let first = selected_profile_ready_except_mcp(&service);
    mark_mcp_ready(&service, &first.id);
    let first_home = first.home_path.clone();
    let prepared = service
        .prepare_configured_agent_session_launch(
            "selected",
            "session-1",
            "invocation-1",
            false,
            Some(crate::agent_sessions::ports::RuntimeLaunchExtension {
                environment: vec![("ROLE_ENV".into(), "preserved".into())],
                ..Default::default()
            }),
        )
        .expect("fresh launch binding");
    assert!(prepared
        .environment
        .iter()
        .any(|(key, value)| key == "CODEX_HOME" && value == &first_home));
    assert!(prepared
        .environment
        .iter()
        .any(|(key, value)| key == "ROLE_ENV" && value == "preserved"));
    let provenance: String = service.connection().unwrap().query_row(
            "SELECT printf('%s:%s:%s',profile_id,environment_key,invocation_mode) FROM agent_session_native_profile_launch_provenance WHERE invocation_id='invocation-1'",
            [], |row| row.get(0),
        ).unwrap();
    assert_eq!(provenance, format!("{}:CODEX_HOME:start", first.id));
    let stored_path_count: i64 = service.connection().unwrap().query_row(
            "SELECT COUNT(*) FROM agent_session_native_profile_launch_provenance WHERE profile_id=?1 AND filesystem_identity=?2 AND instr(profile_id,?3)=0",
            params![first.id, service.resolve_selected_home().unwrap().filesystem_identity, first_home], |row| row.get(0),
        ).unwrap();
    assert_eq!(stored_path_count, 1);
    assert!(service
        .prepare_configured_agent_session_launch(
            "selected",
            "session-1",
            "invocation-1",
            false,
            None
        )
        .is_ok());

    drop(service);
    let reopened = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .expect("reopen profile service");
    assert!(reopened
        .prepare_configured_agent_session_launch(
            "selected",
            "session-1",
            "invocation-2",
            true,
            None
        )
        .is_ok());

    let second = selected_profile_ready_except_mcp(&reopened);
    mark_mcp_ready(&reopened, &second.id);
    assert!(reopened
        .prepare_configured_agent_session_launch(
            "selected",
            "session-1",
            "invocation-3",
            true,
            None
        )
        .is_err());
    assert!(reopened
        .prepare_configured_agent_session_launch(
            "selected",
            "session-2",
            "invocation-4",
            false,
            Some(crate::agent_sessions::ports::RuntimeLaunchExtension {
                environment: vec![("CODEX_HOME".into(), "foreign".into())],
                ..Default::default()
            }),
        )
        .is_err());
}

#[test]
fn registration_rejects_relative_and_application_owned_homes() {
    let (_directory, service) = service();
    assert!(service.register_existing("relative").is_err());
    let dedicated = service.create_dedicated().unwrap();
    assert!(service.register_existing(&dedicated.home_path).is_err());
}

#[test]
fn codex_personality_preference_persists_and_inherit_remains_absent() {
    let (directory, service) = service();
    let profile = service.create_dedicated().unwrap();
    assert_eq!(service.profile(&profile.id).unwrap().personality, None);
    service
        .set_personality(&profile.id, Some(CodexPersonality::Friendly))
        .unwrap();
    assert_eq!(
        service.profile(&profile.id).unwrap().personality,
        Some(CodexPersonality::Friendly)
    );
    drop(service);

    let reopened = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    assert_eq!(
        reopened.profile(&profile.id).unwrap().personality,
        Some(CodexPersonality::Friendly)
    );
    reopened.set_personality(&profile.id, None).unwrap();
    assert_eq!(reopened.profile(&profile.id).unwrap().personality, None);
}

#[test]
fn replacement_and_malformed_marker_fail_closed() {
    let (_directory, service) = service();
    let dedicated = service.create_dedicated().unwrap();
    service.select(&dedicated.id).unwrap();
    let before = service.profile(&dedicated.id).unwrap();
    service
        .update_readiness(
            &dedicated.id,
            Some("authenticated"),
            Some("initialized"),
            Some("passed"),
            Some("ready"),
            None,
        )
        .unwrap();
    let now = Utc::now().to_rfc3339();
    let connection = service.connection().unwrap();
    connection.execute(
            "INSERT INTO native_codex_profile_mode_authorizations (profile_id,mode,filesystem_identity,authority_scope,authority_version,authorization_correlation_id,authorized_at) VALUES (?1,'danger_full_access',?2,?3,?4,?5,?6)",
            params![dedicated.id, before.identity, DANGER_AUTHORITY_SCOPE, DANGER_AUTHORITY_VERSION, "native-danger-test", now],
        ).unwrap();
    connection.execute(
            "INSERT INTO native_codex_profile_sandbox_adoptions (profile_id,filesystem_identity,executable,version,workspace_sandbox_supported,windows_sandbox_setup_supported,correlation_id,observed_at,state,elevated_mode_observed) VALUES (?1,?2,'C:/codex.exe','codex-cli test',1,1,'native-adoption-observation',?3,'verified',1)",
            params![dedicated.id, before.identity, now],
        ).unwrap();
    connection.execute(
            "INSERT INTO native_codex_profile_sandbox_adoption_confirmations (profile_id,filesystem_identity,adoption_correlation_id,confirmation_correlation_id,confirmed_at,state,invalidated_at) VALUES (?1,?2,'native-adoption-observation','native-adoption-confirmation',?3,'confirmed',NULL)",
            params![dedicated.id, before.identity, now],
        ).unwrap();
    fs::write(
        Path::new(&dedicated.home_path).join(MARKER_FILE),
        b"malformed",
    )
    .unwrap();
    assert!(service.select(&dedicated.id).is_err());
    let query = service.query().unwrap();
    let profile = &query.profiles[0];
    assert_eq!(profile.lifecycle, Lifecycle::Malformed);
    assert!(!profile.selected);
    assert!(!profile.execution.danger_full_access_authorized);
    assert_eq!(profile.readiness.authentication, "unknown");
    assert_eq!(profile.readiness.sandbox_initialization, "unknown");
    assert_eq!(profile.readiness.workspace_write_canary, "not_run");
    assert_eq!(profile.sandbox_adoption.disposition, "invalidated");
    assert_eq!(
        profile.sandbox_adoption_confirmation.disposition,
        "invalidated"
    );
}

#[test]
fn readiness_facts_do_not_imply_consumer_resolution() {
    let (_directory, service) = service();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .update_readiness(
            &profile.id,
            Some("authenticated"),
            Some("attention_required"),
            Some("blocked"),
            Some("probe_failed"),
            Some((
                "sandbox",
                Some("sandbox_probe_failed_or_uac_attention_required"),
            )),
        )
        .unwrap();
    assert!(service.resolve_selected_home().is_err());
}

#[test]
fn readiness_requires_every_fact_before_the_resolver_exposes_a_home() {
    let (_directory, service) = service();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .update_readiness(
            &profile.id,
            Some("authenticated"),
            Some("initialized"),
            Some("passed"),
            Some("ready"),
            None,
        )
        .unwrap();
    assert_eq!(
        service.resolve_selected_home().unwrap().home,
        PathBuf::from(profile.home_path)
    );
}

#[test]
fn setup_retries_are_idempotent_and_migration_adds_the_profile_tables() {
    let (directory, service) = service();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    let first = service.request_sandbox_initialization(&profile.id).unwrap();
    let second = service.request_sandbox_initialization(&profile.id).unwrap();
    assert_eq!(first.readiness, second.readiness);
    assert_eq!(
        service
            .run_workspace_write_canary(&profile.id)
            .unwrap()
            .readiness
            .workspace_write_canary,
        "blocked"
    );
}

#[test]
fn sandbox_setup_requires_explicit_uac_confirmation_before_the_canary_can_start() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::unsupported_danger_launch_surface());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
        succeeded: true,
        exit_code: Some(0),
        sandbox_receipt_observed: false,
    });
    service.request_sandbox_initialization(&profile.id).unwrap();
    service.reconcile_setup_attempts(&profile.id).unwrap();
    assert!(!service.probe_root(&profile.id).exists());
    let mut query = service.query().unwrap();
    let awaiting_confirmation = query.profiles.remove(0);
    assert_eq!(
        awaiting_confirmation.readiness.sandbox_initialization,
        "attention_required"
    );
    assert_eq!(
        awaiting_confirmation.readiness.attentions.sandbox,
        Some("native_sandbox_setup_completed_explicit_uac_confirmation_required".into())
    );
    assert_eq!(
        service
            .run_workspace_write_canary(&profile.id)
            .unwrap()
            .readiness
            .workspace_write_canary,
        "blocked"
    );
    service.confirm_sandbox_initialization(&profile.id).unwrap();
    *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
        succeeded: true,
        exit_code: Some(0),
        sandbox_receipt_observed: true,
    });
    service.run_workspace_write_canary(&profile.id).unwrap();
    service.reconcile_setup_attempts(&profile.id).unwrap();
    let mut query = service.query().unwrap();
    let canaried = query.profiles.remove(0);
    assert_eq!(canaried.readiness.workspace_write_canary, "passed");
    assert_eq!(canaried.setup_attempt.phase, "workspace_write_canary");
    assert_eq!(canaried.setup_attempt.disposition, "terminal_succeeded");
    assert_eq!(canaried.setup_attempt.terminal_exit_code, Some(0));

    let calls = fake.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    assert_eq!(
        calls[0].args,
        vec![
            "sandbox",
            "setup",
            "--elevated",
            "--current-user",
            "--codex-home",
            profile.home_path.as_str(),
        ]
    );
    assert_eq!(calls[0].cwd, PathBuf::from(&profile.home_path));
    assert_eq!(calls[1].args[0], "sandbox");
    assert_eq!(calls[1].args[1], "-P");
    assert_eq!(calls[1].args[2], ":workspace");
    assert_eq!(calls[1].args[3], "-C");
    assert_eq!(
        calls[1].args[4],
        service.probe_root(&profile.id).to_string_lossy()
    );
    assert_eq!(
        calls[1].args,
        vec![
            "sandbox".into(),
            "-P".into(),
            ":workspace".into(),
            "-C".into(),
            service
                .probe_root(&profile.id)
                .to_string_lossy()
                .into_owned(),
            "--".into(),
            "cmd.exe".into(),
            "/d".into(),
            "/c".into(),
            ".\\native-codex-profile-canary.cmd".into(),
        ]
    );
    assert_eq!(calls[1].cwd, service.probe_root(&profile.id));
    for call in calls.iter() {
        assert_eq!(
            call.environment,
            native_windows_cli_environment(Path::new(&profile.home_path))
        );
        assert!(!call
            .args
            .iter()
            .any(|argument| argument.contains("state-json")));
        assert!(!call
            .args
            .iter()
            .any(|argument| argument == "--include-managed-config" || argument == "--add-dir"));
        assert!(!call
            .args
            .iter()
            .any(|argument| argument.contains("dangerously")));
    }
    assert!(calls[0].sandbox_receipt.is_none());
    assert!(calls[1].sandbox_receipt.is_some());
    assert!(!service
        .probe_root(&profile.id)
        .join(WORKSPACE_WRITE_CANARY_COMMAND_FILE)
        .exists());
}

#[test]
fn workspace_canary_receipt_absence_is_durable_and_separate_from_exit_evidence() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::unsupported_danger_launch_surface());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .update_readiness(
            &profile.id,
            None,
            Some("initialized"),
            Some("not_run"),
            None,
            None,
        )
        .unwrap();
    *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
        succeeded: false,
        exit_code: Some(1),
        sandbox_receipt_observed: false,
    });
    let stale_receipt = service
        .probe_root(&profile.id)
        .join("workspace_write_canary.txt");
    fs::create_dir_all(stale_receipt.parent().unwrap()).unwrap();
    fs::write(&stale_receipt, "native-codex-profile-canary").unwrap();

    service.run_workspace_write_canary(&profile.id).unwrap();
    service.reconcile_setup_attempts(&profile.id).unwrap();

    let profile = service.query().unwrap().profiles.remove(0);
    assert_eq!(profile.readiness.workspace_write_canary, "blocked");
    assert_eq!(profile.setup_attempt.phase, "workspace_write_canary");
    assert_eq!(profile.setup_attempt.disposition, "terminal_failed");
    assert_eq!(
        profile.setup_attempt.terminal_classification,
        "receipt_missing"
    );
    assert_eq!(profile.setup_attempt.terminal_exit_code, Some(1));
    assert!(!stale_receipt.exists());
}

#[test]
fn contradictory_receipt_missing_attempt_fails_closed_before_profile_reconciliation() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .update_readiness(
            &profile.id,
            None,
            Some("initialized"),
            Some("not_run"),
            None,
            None,
        )
        .unwrap();
    *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
        succeeded: false,
        exit_code: Some(1),
        sandbox_receipt_observed: false,
    });
    service.run_workspace_write_canary(&profile.id).unwrap();
    service.reconcile_setup_attempts(&profile.id).unwrap();
    assert_eq!(
        service.query().unwrap().profiles[0]
            .setup_attempt
            .terminal_classification,
        "receipt_missing"
    );
    let connection = service.connection().unwrap();
    connection
        .execute_batch("PRAGMA ignore_check_constraints=ON;")
        .unwrap();
    assert_eq!(
            connection
                .execute(
                    "UPDATE native_codex_profile_setup_attempts SET phase='sandbox_initialization' WHERE profile_id=?1",
                    params![profile.id],
                )
                .unwrap(),
            1
        );
    connection
        .execute_batch("PRAGMA ignore_check_constraints=OFF;")
        .unwrap();
    assert!(load_profiles(&mut service.connection().unwrap()).is_err());
}

#[test]
fn windows_semantic_capability_detection_accepts_combined_setup_help_and_rejects_drift() {
    assert_eq!(
        windows_semantic_sandbox_capabilities(
            "Options: --permission-profile <NAME> --cd <DIR>",
            "Error: --current-user required\nOptions: --elevated --current-user --codex-home <DIR>",
        ),
        (true, true)
    );
    assert_eq!(
        windows_semantic_sandbox_capabilities(
            "Options: --permission-profile <NAME>",
            "Options: --elevated --current-user --codex-home <DIR>",
        ),
        (false, true)
    );
    assert_eq!(
        windows_semantic_sandbox_capabilities(
            "Options: --permission-profile <NAME> --cd <DIR>",
            "Options: --elevated --current-user",
        ),
        (true, false)
    );
}

#[test]
fn workspace_launch_capability_requires_the_exact_version_and_every_known_flag() {
    assert_eq!(
            workspace_launch_semantic_capabilities(
                "codex-cli 0.144.0",
                "--json --strict-config --ignore-user-config --ignore-rules --config <KEY=VALUE> --sandbox <MODE> --cd <DIR> --skip-git-repo-check workspace-write",
            ),
            (true, true)
        );
    assert_eq!(
            workspace_launch_semantic_capabilities(
                "codex-cli 0.147.0-alpha.1.2",
                "--json --strict-config --ignore-user-config --ignore-rules --config <KEY=VALUE> --sandbox <MODE> --cd <DIR> --skip-git-repo-check workspace-write",
            ),
            (true, false)
        );
    assert_eq!(
            workspace_launch_semantic_capabilities(
                "codex-cli 0.144.0",
                "--json --ignore-user-config --ignore-rules --config <KEY=VALUE> --sandbox <MODE> --cd <DIR> --skip-git-repo-check workspace-write",
            ),
            (false, false)
        );
}

#[test]
fn workspace_project_trust_override_is_one_toml_value_for_a_dot_bearing_windows_path() {
    assert_eq!(
        workspace_project_trust_override(Path::new(r"C:\application\.runtime\assigned root"))
            .unwrap(),
        r#"projects={"C:\\application\\.runtime\\assigned root"={trust_level="untrusted"}}"#,
    );
}

#[test]
fn unsupported_workspace_policy_is_durable_and_blocks_children_and_uac() {
    let (directory, mut service) = service();
    let fake = Arc::new(FakeCli::unsupported_workspace_policy());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();

    let requested = service.request_sandbox_initialization(&profile.id).unwrap();
    assert_eq!(requested.setup_attempt.disposition, "policy_unsupported");
    assert_eq!(
        requested.setup_attempt.terminal_classification,
        "policy_unsupported"
    );
    assert_eq!(
        requested.setup_attempt.workspace_sandbox_supported,
        Some(false)
    );
    assert_eq!(
        requested.setup_attempt.executable.as_deref(),
        Some("C:/application-owned/codex.exe")
    );
    assert_eq!(
        requested.setup_attempt.version.as_deref(),
        Some("codex-cli test")
    );
    assert!(requested.setup_attempt.correlation_id.is_some());
    assert!(requested.setup_attempt.requested_at.is_some());
    assert!(requested.setup_attempt.deadline_at.is_some());
    assert!(requested.setup_attempt.launch_accepted_at.is_none());
    assert!(requested.setup_attempt.settled_at.is_some());
    assert!(requested.setup_attempt.terminal_exit_code.is_none());
    assert_eq!(*fake.starts.lock().unwrap(), 0);
    assert_eq!(
        requested.readiness.attentions.sandbox,
        Some("native_sandbox_semantic_policy_unsupported".into())
    );
    assert!(service.confirm_sandbox_initialization(&profile.id).is_err());
    assert_eq!(
        service
            .run_workspace_write_canary(&profile.id)
            .unwrap()
            .readiness
            .workspace_write_canary,
        "blocked"
    );
    assert_eq!(*fake.starts.lock().unwrap(), 0);
    assert!(!directory.path().join("app").join("probes").exists());
}

#[test]
fn missing_windows_setup_capability_fails_closed_before_a_child_or_probe_root() {
    let (directory, mut service) = service();
    let fake = Arc::new(FakeCli::unsupported_windows_sandbox_setup());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();

    let requested = service.request_sandbox_initialization(&profile.id).unwrap();
    assert_eq!(requested.setup_attempt.disposition, "policy_unsupported");
    assert_eq!(
        requested.setup_attempt.workspace_sandbox_supported,
        Some(false)
    );
    assert_eq!(*fake.starts.lock().unwrap(), 0);
    assert!(!directory.path().join("app").join("probes").exists());
}

#[test]
fn policy_unsupported_attempts_reject_contradictory_storage_and_query_facts() {
    let (_directory, mut service) = service();
    service.cli = Arc::new(FakeCli::unsupported_workspace_policy());
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service.request_sandbox_initialization(&profile.id).unwrap();
    let connection = service.connection().unwrap();
    for mutation in [
        "phase='not_requested'",
        "launch_accepted_at='2026-08-07T12:00:01Z'",
        "workspace_sandbox_supported=1",
        "terminal_exit_code=1",
        "terminal_classification='exit_code'",
        "executable=NULL",
        "executable=''",
        "version=NULL",
        "version=''",
        "correlation_id=''",
        "requested_at=''",
        "deadline_at=''",
        "settled_at=NULL",
        "settled_at=''",
    ] {
        assert!(connection
            .execute(
                &format!(
                    "UPDATE native_codex_profile_setup_attempts SET {mutation} WHERE profile_id=?1"
                ),
                params![profile.id],
            )
            .is_err());
    }
    connection
        .execute_batch("PRAGMA ignore_check_constraints=ON;")
        .unwrap();
    connection
            .execute(
                "UPDATE native_codex_profile_setup_attempts SET launch_accepted_at='2026-08-07T12:00:01Z' WHERE profile_id=?1",
                params![profile.id],
            )
            .unwrap();
    connection
        .execute_batch("PRAGMA ignore_check_constraints=OFF;")
        .unwrap();
    assert!(service.query().is_err());
}

#[test]
fn setup_attempt_persists_safe_provenance_then_classifies_terminal_failure() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
        succeeded: false,
        exit_code: Some(7),
        sandbox_receipt_observed: false,
    });
    service.cli = fake;
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    let requested = service.request_sandbox_initialization(&profile.id).unwrap();
    assert_eq!(requested.setup_attempt.phase, "sandbox_initialization");
    assert_eq!(requested.setup_attempt.disposition, "pending");
    assert_eq!(
        requested.setup_attempt.executable.as_deref(),
        Some("C:/application-owned/codex.exe")
    );
    assert_eq!(
        requested.setup_attempt.version.as_deref(),
        Some("codex-cli test")
    );
    assert_eq!(
        requested.setup_attempt.workspace_sandbox_supported,
        Some(true)
    );
    assert!(requested.setup_attempt.correlation_id.is_some());
    assert!(requested.setup_attempt.requested_at.is_some());
    assert!(requested.setup_attempt.launch_accepted_at.is_some());
    assert!(requested.setup_attempt.settled_at.is_none());

    service.reconcile_setup_attempts(&profile.id).unwrap();
    let settled = service.profile(&profile.id).unwrap();
    assert_eq!(settled.setup_attempt.disposition, "terminal_failed");
    assert_eq!(settled.setup_attempt.terminal_classification, "exit_code");
    assert_eq!(settled.setup_attempt.terminal_exit_code, Some(7));
    assert_eq!(
        settled.readiness.sandbox_initialization,
        "attention_required"
    );
    assert!(service.confirm_sandbox_initialization(&profile.id).is_err());
}

#[test]
fn setup_launch_failure_is_durable_without_uac_or_initialization_claims() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    service.cli = Arc::new(FakeCli::failing_start());
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    let result = service.request_sandbox_initialization(&profile.id).unwrap();
    assert_eq!(result.setup_attempt.disposition, "launch_failed");
    assert_eq!(
        result.setup_attempt.terminal_classification,
        "launch_failed"
    );
    assert!(result.setup_attempt.launch_accepted_at.is_none());
    assert_eq!(
        result.readiness.sandbox_initialization,
        "attention_required"
    );
    assert_ne!(
        result.readiness.attentions.sandbox.as_deref(),
        Some("native_sandbox_setup_completed_explicit_uac_confirmation_required")
    );
    assert!(service.confirm_sandbox_initialization(&profile.id).is_err());
}

#[test]
fn pending_sandbox_attempts_are_reused_then_timeout_without_launch_success() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service.request_sandbox_initialization(&profile.id).unwrap();
    service.request_sandbox_initialization(&profile.id).unwrap();
    assert_eq!(*fake.starts.lock().unwrap(), 1);
    let connection = service.connection().unwrap();
    connection
            .execute(
                "UPDATE native_codex_profile_setup_attempts SET deadline_at='2000-01-01T00:00:00+00:00' WHERE profile_id=?1",
                params![profile.id],
            )
            .unwrap();
    service.reconcile_setup_attempts(&profile.id).unwrap();
    let result = service.query().unwrap();
    assert_eq!(
        result.profiles[0].readiness.sandbox_initialization,
        "attention_required"
    );
    assert_eq!(
        result.profiles[0].readiness.attentions.sandbox,
        Some("native_sandbox_attempt_timed_out_human_or_uac_attention".into())
    );
    assert_eq!(*fake.terminated.lock().unwrap(), 1);
    assert_eq!(result.profiles[0].setup_attempt.disposition, "timed_out");
    assert_eq!(
        result.profiles[0].setup_attempt.terminal_classification,
        "timed_out"
    );
}

#[test]
fn reopened_pending_setup_is_recovered_without_an_owned_child_to_observe() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let mut service =
        NativeProfileService::open(database.clone(), directory.path().join("app")).unwrap();
    service.cli = Arc::new(FakeCli::succeeding());
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service.request_sandbox_initialization(&profile.id).unwrap();
    drop(service);
    let reopened = NativeProfileService::open(database, directory.path().join("app")).unwrap();
    reopened.reconcile_setup_attempts(&profile.id).unwrap();
    let query = reopened.query().unwrap();
    assert_eq!(
        query.profiles[0].readiness.sandbox_initialization,
        "attention_required"
    );
    assert_eq!(
        query.profiles[0].readiness.attentions.sandbox,
        Some("native_sandbox_attempt_recovered_without_owned_process".into())
    );
    assert_eq!(
        query.profiles[0].setup_attempt.disposition,
        "recovered_unobserved"
    );
}

#[test]
fn canary_before_observed_initialization_never_starts_a_child() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    assert!(service.confirm_sandbox_initialization(&profile.id).is_err());
    let blocked = service.run_workspace_write_canary(&profile.id).unwrap();
    assert_eq!(blocked.readiness.workspace_write_canary, "blocked");
    assert_eq!(*fake.starts.lock().unwrap(), 0);
    assert_eq!(
        blocked.readiness.attentions.canary,
        Some("workspace_write_canary_requires_observed_sandbox_initialization".into())
    );
}

#[test]
fn setup_confirmation_and_canary_require_the_currently_selected_profile() {
    let (directory, mut service) = service();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let selected = service.create_dedicated().unwrap();
    let unselected = service.create_dedicated().unwrap();
    service.select(&selected.id).unwrap();

    assert!(service
        .request_sandbox_initialization(&unselected.id)
        .is_err());
    assert!(service
        .confirm_sandbox_initialization(&unselected.id)
        .is_err());
    assert!(service.run_workspace_write_canary(&unselected.id).is_err());
    assert_eq!(*fake.starts.lock().unwrap(), 0);
    assert!(!service.probe_root(&unselected.id).exists());
    assert!(!directory.path().join("app").join("probes").exists());
}

#[test]
fn switching_selection_cancels_only_the_previous_profile_owned_setup_attempt() {
    let (_directory, mut service) = service();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let first = service.create_dedicated().unwrap();
    let second = service.create_dedicated().unwrap();
    service.select(&first.id).unwrap();
    service.request_sandbox_initialization(&first.id).unwrap();

    service.select(&second.id).unwrap();
    service.reconcile_setup_attempts(&first.id).unwrap();
    assert_eq!(*fake.terminated.lock().unwrap(), 1);
    assert_eq!(
        service
            .profile(&first.id)
            .unwrap()
            .setup_attempt
            .disposition,
        "cancelled"
    );
    assert!(service.confirm_sandbox_initialization(&first.id).is_err());

    service.select(&first.id).unwrap();
    assert!(service.confirm_sandbox_initialization(&first.id).is_err());
    assert_eq!(
        service
            .profile(&first.id)
            .unwrap()
            .setup_attempt
            .disposition,
        "cancelled"
    );
}

#[test]
fn switching_selection_cancels_the_previous_profile_owned_workspace_canary() {
    let (_directory, mut service) = service();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let first = service.create_dedicated().unwrap();
    let second = service.create_dedicated().unwrap();
    service.select(&first.id).unwrap();
    service
        .update_readiness(&first.id, None, Some("initialized"), None, None, None)
        .unwrap();
    service.run_workspace_write_canary(&first.id).unwrap();

    service.select(&second.id).unwrap();
    service.reconcile_setup_attempts(&first.id).unwrap();
    assert_eq!(*fake.terminated.lock().unwrap(), 1);
    assert_eq!(
        service
            .profile(&first.id)
            .unwrap()
            .setup_attempt
            .disposition,
        "cancelled"
    );
    assert_eq!(
        service
            .profile(&first.id)
            .unwrap()
            .readiness
            .workspace_write_canary,
        "blocked"
    );
}

#[test]
fn externally_provisioned_sandbox_is_adopted_without_a_setup_attempt_or_uac_claim() {
    let (directory, mut service) = service();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    fs::write(
        Path::new(&profile.home_path).join("config.toml"),
        "[windows]\nsandbox = \"elevated\"\n",
    )
    .unwrap();

    let verified = service.verify_preprovisioned_sandbox(&profile.id).unwrap();
    assert_eq!(verified.sandbox_adoption.disposition, "verified");
    assert_eq!(verified.setup_attempt.disposition, "not_requested");
    assert!(verified.sandbox_adoption.observed_at.is_some());
    assert!(verified.sandbox_adoption.correlation_id.is_some());
    assert_eq!(
        verified.readiness.sandbox_initialization,
        "attention_required"
    );
    assert_eq!(*fake.starts.lock().unwrap(), 0);
    assert!(service.connection().unwrap().execute(
            "UPDATE native_codex_profile_sandbox_adoptions SET elevated_mode_observed=0 WHERE profile_id=?1",
            params![profile.id],
        ).is_err());

    let confirmed = service
        .confirm_preprovisioned_sandbox_adoption(&profile.id)
        .unwrap();
    assert_eq!(
        confirmed.sandbox_adoption_confirmation.disposition,
        "confirmed"
    );
    assert!(confirmed
        .sandbox_adoption_confirmation
        .confirmed_at
        .is_some());
    assert!(confirmed
        .sandbox_adoption_confirmation
        .correlation_id
        .is_some());
    assert_eq!(confirmed.readiness.sandbox_initialization, "initialized");
    assert_eq!(
        confirmed.readiness.attentions.sandbox,
        Some("external_sandbox_adoption_confirmed_product_uac_unobserved".into())
    );
    assert_eq!(*fake.starts.lock().unwrap(), 0);
    drop(service);
    let mut reopened = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    reopened.cli = fake;
    let mut reopened_query = reopened.query().unwrap();
    let reopened_profile = reopened_query.profiles.remove(0);
    assert_eq!(reopened_profile.sandbox_adoption.disposition, "verified");
    assert_eq!(
        reopened_profile.sandbox_adoption_confirmation.disposition,
        "confirmed"
    );
}

#[test]
fn external_sandbox_adoption_rejects_unselected_and_invalidates_on_continuity_loss() {
    let (_directory, mut service) = service();
    service.cli = Arc::new(FakeCli::succeeding());
    let first = service.create_dedicated().unwrap();
    let second = service.create_dedicated().unwrap();
    service.select(&first.id).unwrap();
    fs::write(
        Path::new(&first.home_path).join("config.toml"),
        "[windows]\nsandbox = \"elevated\"\n",
    )
    .unwrap();
    assert!(service.verify_preprovisioned_sandbox(&second.id).is_err());
    service.verify_preprovisioned_sandbox(&first.id).unwrap();
    fs::remove_file(Path::new(&first.home_path).join(MARKER_FILE)).unwrap();
    service.query().unwrap();
    assert_eq!(
        service
            .profile(&first.id)
            .unwrap()
            .sandbox_adoption
            .disposition,
        "invalidated"
    );
}

#[test]
fn external_sandbox_adoption_confirmation_is_invalidated_by_selection_and_observation_drift() {
    let (_directory, mut service) = service();
    service.cli = Arc::new(FakeCli::succeeding());
    let first = service.create_dedicated().unwrap();
    let second = service.create_dedicated().unwrap();
    service.select(&first.id).unwrap();
    let config = Path::new(&first.home_path).join("config.toml");
    fs::write(&config, "[windows]\nsandbox = \"elevated\"\n").unwrap();
    service.verify_preprovisioned_sandbox(&first.id).unwrap();
    service
        .confirm_preprovisioned_sandbox_adoption(&first.id)
        .unwrap();

    service.select(&second.id).unwrap();
    service.reconcile_sandbox_adoption(&first.id).unwrap();
    let invalidated = service.profile(&first.id).unwrap();
    assert_eq!(invalidated.sandbox_adoption.disposition, "invalidated");
    assert_eq!(
        invalidated.sandbox_adoption_confirmation.disposition,
        "invalidated"
    );
    assert_eq!(
        invalidated.readiness.sandbox_initialization,
        "attention_required"
    );
    assert_eq!(invalidated.readiness.workspace_write_canary, "blocked");

    service.select(&first.id).unwrap();
    service.verify_preprovisioned_sandbox(&first.id).unwrap();
    service
        .confirm_preprovisioned_sandbox_adoption(&first.id)
        .unwrap();
    fs::remove_file(config).unwrap();
    service.reconcile_sandbox_adoption(&first.id).unwrap();
    let drifted = service
        .query()
        .unwrap()
        .profiles
        .into_iter()
        .find(|profile| profile.id == first.id)
        .unwrap();
    assert_eq!(drifted.sandbox_adoption.disposition, "invalidated");
    assert_eq!(
        drifted.sandbox_adoption_confirmation.disposition,
        "invalidated"
    );
    assert_eq!(
        drifted.readiness.sandbox_initialization,
        "attention_required"
    );
}

#[test]
fn elevated_sandbox_observation_rejects_duplicate_windows_assignments() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("config.toml"),
        "[windows]\nsandbox = \"elevated\"\nsandbox = \"elevated\"\n",
    )
    .unwrap();
    assert!(observe_elevated_windows_sandbox_mode(directory.path()).is_err());
    fs::write(
        directory.path().join("config.toml"),
        "[windows]\nsandbox = \"elevated\"\n[windows]\n",
    )
    .unwrap();
    assert!(observe_elevated_windows_sandbox_mode(directory.path()).is_err());
}

#[test]
fn browser_login_is_idempotently_supervised_and_owned_children_are_reaped_on_shutdown() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service.request_login(&profile.id).unwrap();
    service.request_login(&profile.id).unwrap();
    assert_eq!(*fake.starts.lock().unwrap(), 1);
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .attentions
            .authentication,
        Some("browser_login_attempt_pending".into())
    );
    drop(service);
    assert_eq!(*fake.terminated.lock().unwrap(), 1);
}

#[test]
fn windows_login_environment_is_allowlisted_and_keeps_the_product_selected_home() {
    let environment =
        native_windows_cli_environment_from(Path::new("C:/product-owned-home"), &|key| match key {
            "PATH" => Some("C:/Windows/System32;C:/Windows".into()),
            "SYSTEMROOT" => Some("C:/Windows".into()),
            "USERPROFILE" => Some("C:/Users/launching-user".into()),
            "CODEX_HOME" => Some("C:/foreign-home".into()),
            "UNRELATED_SECRET" => Some("must-not-pass".into()),
            _ => None,
        });
    assert!(environment.contains(&("CODEX_HOME".into(), "C:/product-owned-home".into())));
    assert!(environment.contains(&("PATH".into(), "C:/Windows/System32;C:/Windows".into())));
    assert!(environment.contains(&("SYSTEMROOT".into(), "C:/Windows".into())));
    assert!(environment.contains(&("USERPROFILE".into(), "C:/Users/launching-user".into())));
    assert!(!environment.iter().any(|(key, _)| key == "UNRELATED_SECRET"));
    assert!(!environment
        .iter()
        .any(|(_, value)| value == "C:/foreign-home"));
}

#[cfg(windows)]
#[test]
fn system_cli_port_drains_windows_stdio_and_observes_only_its_bound_receipt() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().join("selected-home");
    let root = directory.path().join("application-owned-probe-root");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&root).unwrap();
    let receipt = root.join("workspace_write_canary.txt");
    let other = root.join("not-the-designated-receipt.txt");
    let script = root.join("bound-system-cli-port.cmd");
    let other_script = root.join("unbound-system-cli-port.cmd");
    fs::write(
            &script,
            format!(
                "@echo off\r\nif /i not \"%CODEX_HOME%\"==\"{}\" exit /b 2\r\nif /i not \"%CD%\"==\"{}\" exit /b 3\r\necho native-codex-profile-canary>\"{}\"\r\necho discarded-stderr>&2\r\n",
                home.to_string_lossy(),
                root.to_string_lossy(),
                receipt.to_string_lossy(),
            ),
        )
        .unwrap();
    fs::write(
        &other_script,
        format!(
            "@echo off\r\necho native-codex-profile-canary>\"{}\"\r\nexit /b 0\r\n",
            other.to_string_lossy(),
        ),
    )
    .unwrap();
    let command = std::env::var("COMSPEC").unwrap();
    let port = SystemNativeCliPort {
        program: Ok(command),
    };
    let invocation = NativeCliInvocation {
        args: vec![
            "/d".into(),
            "/s".into(),
            "/c".into(),
            script.to_string_lossy().into_owned(),
        ],
        cwd: root.clone(),
        codex_home: home.clone(),
        environment: native_windows_cli_environment(&home),
        sandbox_receipt: Some(receipt.clone()),
        sandbox_command_file: None,
    };
    let settled = port.run(&invocation).unwrap();
    assert_eq!(settled.exit_code, Some(0));
    assert!(settled.succeeded);
    assert!(settled.sandbox_receipt_observed);

    let unbound = NativeCliInvocation {
        args: vec![
            "/d".into(),
            "/s".into(),
            "/c".into(),
            other_script.to_string_lossy().into_owned(),
        ],
        sandbox_receipt: Some(root.join("different-designated-receipt.txt")),
        ..invocation
    };
    let settled = port.run(&unbound).unwrap();
    assert!(settled.succeeded);
    assert!(!settled.sandbox_receipt_observed);
    assert!(other.exists());
}

#[cfg(windows)]
#[test]
fn system_cli_port_forwards_a_workspace_command_file_without_receipt_path_quoting() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().join("selected-home");
    let root = directory.path().join("application owned probe root");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&root).unwrap();
    let receipt = root.join("workspace_write_canary.txt");
    let command = root.join("native-codex-profile-canary.cmd");
    fs::write(
        &command,
        "@echo off\r\necho native-codex-profile-canary>workspace_write_canary.txt\r\n",
    )
    .unwrap();
    let port = SystemNativeCliPort {
        program: Ok(std::env::var("COMSPEC").unwrap()),
    };
    let settled = port
        .run(&NativeCliInvocation {
            args: vec![
                "/d".into(),
                "/c".into(),
                ".\\native-codex-profile-canary.cmd".into(),
            ],
            cwd: root,
            codex_home: home.clone(),
            environment: native_windows_cli_environment(&home),
            sandbox_receipt: Some(receipt),
            sandbox_command_file: Some(command.clone()),
        })
        .unwrap();

    assert!(settled.succeeded);
    assert!(settled.sandbox_receipt_observed);
    assert!(!command.exists());
}

#[cfg(windows)]
#[test]
fn system_cli_port_observes_the_allowlisted_full_access_receipt_value() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().join("selected-home");
    let parent = directory
        .path()
        .join("application-owned-full-access-canary");
    let work = parent.join("work");
    let receipt_root = parent.join("receipt");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&work).unwrap();
    fs::create_dir_all(&receipt_root).unwrap();
    let receipt = receipt_root.join(FULL_ACCESS_CANARY_RECEIPT_FILE);
    let command = work.join("write-full-access-receipt.cmd");
    fs::write(
            &command,
            format!(
                "@echo off\r\necho {FULL_ACCESS_CANARY_RECEIPT}>{FULL_ACCESS_CANARY_RECEIPT_RELATIVE_PATH}\r\n"
            ),
        )
        .unwrap();
    let port = SystemNativeCliPort {
        program: Ok(std::env::var("COMSPEC").unwrap()),
    };
    let settled = port
        .run(&NativeCliInvocation {
            args: vec![
                "/d".into(),
                "/c".into(),
                ".\\write-full-access-receipt.cmd".into(),
            ],
            cwd: work.clone(),
            codex_home: home.clone(),
            environment: native_windows_cli_environment(&home),
            sandbox_receipt: Some(receipt.clone()),
            sandbox_command_file: None,
        })
        .unwrap();

    assert!(settled.succeeded);
    assert!(settled.sandbox_receipt_observed);
    assert!(!receipt.starts_with(&work));
    assert!(receipt.starts_with(&parent));
}

#[cfg(windows)]
#[test]
fn system_cli_port_quote_bearing_workspace_payload_fails_when_the_probe_path_has_spaces() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().join("selected-home");
    let root = directory.path().join("application owned probe root");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&root).unwrap();
    let receipt = root.join("workspace_write_canary.txt");
    let port = SystemNativeCliPort {
        program: Ok(std::env::var("COMSPEC").unwrap()),
    };

    let settled = port
        .run(&NativeCliInvocation {
            args: vec![
                "/d".into(),
                "/s".into(),
                "/c".into(),
                format!("echo native-codex-profile-canary>\"{}\"", receipt.display()),
            ],
            cwd: root,
            codex_home: home.clone(),
            environment: native_windows_cli_environment(&home),
            sandbox_receipt: Some(receipt),
            sandbox_command_file: None,
        })
        .unwrap();

    assert!(!settled.succeeded);
    assert!(!settled.sandbox_receipt_observed);
}

#[test]
fn login_and_status_use_the_allowlisted_windows_environment_without_a_real_browser() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service.request_login(&profile.id).unwrap();
    service.refresh_readiness(&profile.id).unwrap();
    let calls = fake.calls.lock().unwrap();
    for call in calls
        .iter()
        .filter(|call| call.args.first().is_some_and(|arg| arg == "login"))
    {
        assert_eq!(
            call.environment,
            native_windows_cli_environment(Path::new(&profile.home_path))
        );
        assert!(call.environment.iter().any(|(key, _)| key == "CODEX_HOME"));
    }
}

#[test]
fn browser_login_exit_requires_a_separate_status_observation() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
        succeeded: true,
        exit_code: Some(0),
        sandbox_receipt_observed: false,
    });
    service.cli = fake;
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service.request_login(&profile.id).unwrap();
    let mut query = service.query().unwrap();
    let after_exit = query.profiles.remove(0);
    assert_eq!(after_exit.readiness.authentication, "unknown");
    assert_eq!(
        after_exit.readiness.attentions.authentication,
        Some("browser_login_terminal_succeeded_browser_handoff_unobserved".into())
    );
    assert_eq!(after_exit.login_attempt.disposition, "terminal_succeeded");
    assert_eq!(after_exit.login_attempt.browser_handoff, "unobserved");
}

#[test]
fn browser_login_launch_failure_is_durable_without_authentication_or_handoff_claims() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    service.cli = Arc::new(FakeCli::failing_start());
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    let result = service.request_login(&profile.id).unwrap();
    assert_eq!(result.login_attempt.disposition, "launch_failed");
    assert_eq!(result.login_attempt.browser_handoff, "unobserved");
    assert_eq!(result.readiness.authentication, "unknown");
    let persisted: (String, String, Option<String>) = service.connection().unwrap().query_row(
            "SELECT executable,version,launch_accepted_at FROM native_codex_profile_login_attempts WHERE profile_id=?1",
            params![profile.id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).unwrap();
    assert_eq!(persisted.0, "C:/application-owned/codex.exe");
    assert_eq!(persisted.1, "codex-cli test");
    assert!(persisted.2.is_none());
}

#[test]
fn browser_login_terminal_failure_is_durable_and_does_not_claim_unauthenticated() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
        succeeded: false,
        exit_code: Some(1),
        sandbox_receipt_observed: false,
    });
    service.cli = fake;
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service.request_login(&profile.id).unwrap();
    let result = service.query().unwrap().profiles.remove(0);
    assert_eq!(result.login_attempt.disposition, "terminal_failed");
    assert_eq!(result.login_attempt.browser_handoff, "unobserved");
    assert_eq!(result.readiness.authentication, "unknown");
}

#[test]
fn login_refresh_preserves_pending_attempt_creates_probe_root_and_reconciles_cli_attention() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .set_attention(&profile.id, "cli", Some("codex_cli_unavailable"), false)
        .unwrap();
    let refreshed = service.refresh_readiness(&profile.id).unwrap();
    assert!(service.probe_root(&profile.id).is_dir());
    assert_eq!(refreshed.login_attempt.disposition, "not_requested");
    assert_eq!(refreshed.readiness.attentions.cli, None);
    service.request_login(&profile.id).unwrap();
    let refreshed = service.refresh_readiness(&profile.id).unwrap();
    assert_eq!(refreshed.login_attempt.disposition, "pending");
    assert_eq!(refreshed.login_attempt.browser_handoff, "unobserved");
    assert_eq!(refreshed.readiness.authentication, "authenticated");
    assert_eq!(
        refreshed.readiness.attentions.authentication,
        Some("browser_login_attempt_pending".into())
    );
    assert_eq!(
        fake.calls.lock().unwrap().last().unwrap().cwd,
        fs::canonicalize(service.probe_root(&profile.id)).unwrap()
    );
}

#[test]
fn pending_login_reopen_recovers_without_claiming_process_or_browser_activity() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let mut service =
        NativeProfileService::open(database.clone(), directory.path().join("app")).unwrap();
    service.cli = Arc::new(FakeCli::succeeding());
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service.request_login(&profile.id).unwrap();
    drop(service);
    let reopened = NativeProfileService::open(database, directory.path().join("app")).unwrap();
    let profile = reopened.query().unwrap().profiles.remove(0);
    assert_eq!(profile.login_attempt.disposition, "recovered_unobserved");
    assert_eq!(profile.login_attempt.browser_handoff, "unobserved");
    assert_eq!(profile.readiness.authentication, "unknown");
}

#[test]
fn login_operations_require_the_selected_validated_profile() {
    let (_directory, mut service) = service();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    assert!(service.request_login(&profile.id).is_err());
    assert!(service.refresh_readiness(&profile.id).is_ok());
    assert_eq!(*fake.starts.lock().unwrap(), 0);
    assert_eq!(fake.calls.lock().unwrap().len(), 1);
    service.select(&profile.id).unwrap();
    fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
    assert!(service.request_login(&profile.id).is_err());
}

#[test]
fn query_revalidation_clears_selection_and_invalidates_readiness() {
    let (_directory, service) = service();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .update_readiness(
            &profile.id,
            Some("authenticated"),
            Some("initialized"),
            Some("passed"),
            Some("ready"),
            None,
        )
        .unwrap();
    fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
    let query = service.query().unwrap();
    assert!(!query.profiles[0].selected);
    assert_eq!(query.profiles[0].lifecycle, Lifecycle::Malformed);
    assert_eq!(query.profiles[0].readiness.authentication, "unknown");
    assert_eq!(
        query.profiles[0].readiness.workspace_write_canary,
        "not_run"
    );
}

#[test]
fn danger_full_access_is_explicit_durable_and_revocable_per_profile_identity() {
    let (directory, service) = service();
    let first = service.create_dedicated().unwrap();
    let second = service.create_dedicated().unwrap();
    service.select(&first.id).unwrap();
    service
        .select_execution_mode(&first.id, ExecutionMode::DangerFullAccess)
        .unwrap();
    assert!(
        !service
            .profile(&first.id)
            .unwrap()
            .execution
            .danger_full_access_authorized
    );
    service.authorize_danger_full_access(&first.id).unwrap();
    assert!(
        service
            .profile(&first.id)
            .unwrap()
            .execution
            .danger_full_access_authorized
    );
    assert!(
        !service
            .profile(&second.id)
            .unwrap()
            .execution
            .danger_full_access_authorized
    );
    drop(service);
    let reopened = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    assert!(
        reopened
            .profile(&first.id)
            .unwrap()
            .execution
            .danger_full_access_authorized
    );
    reopened.revoke_danger_full_access(&first.id).unwrap();
    assert!(
        !reopened
            .profile(&first.id)
            .unwrap()
            .execution
            .danger_full_access_authorized
    );
}

#[test]
fn danger_authority_fails_closed_when_identity_is_stale_or_continuity_is_lost() {
    let (_directory, service) = service();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
        .unwrap();
    service.authorize_danger_full_access(&profile.id).unwrap();
    service
            .connection()
            .unwrap()
            .execute(
                "UPDATE native_codex_profile_mode_authorizations SET filesystem_identity='stale' WHERE profile_id=?1",
                params![profile.id],
            )
            .unwrap();
    assert!(
        !service
            .profile(&profile.id)
            .unwrap()
            .execution
            .danger_full_access_authorized
    );
    service.authorize_danger_full_access(&profile.id).unwrap();
    fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
    service.query().unwrap();
    let revoked: Option<String> = service.connection().unwrap().query_row(
            "SELECT revoked_at FROM native_codex_profile_mode_authorizations WHERE profile_id=?1 AND mode='danger_full_access'",
            params![profile.id], |row| row.get(0),
        ).unwrap();
    assert!(revoked.is_some());
}

#[test]
fn workspace_launch_isolated_from_project_authority_after_readiness() {
    let (directory, mut service) = service();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .select_execution_mode(&profile.id, ExecutionMode::WorkspaceWrite)
        .unwrap();
    let root = directory.path().join("assigned-application-root");
    fs::create_dir_all(&root).unwrap();
    let workspace_target = NativeLaunchTarget::application_owned(root.clone(), true).unwrap();
    assert!(service
        .project_launch(&profile.id, &workspace_target)
        .is_err());
    service
        .update_readiness(
            &profile.id,
            None,
            Some("initialized"),
            Some("passed"),
            None,
            None,
        )
        .unwrap();
    let workspace = service
        .project_launch(&profile.id, &workspace_target)
        .unwrap();
    assert!(workspace
        .arguments
        .windows(2)
        .any(|arguments| arguments == ["--sandbox", "workspace-write"]));
    assert!(workspace
        .arguments
        .iter()
        .any(|argument| argument == "--strict-config"));
    assert!(workspace.arguments.windows(2).any(|arguments| {
        arguments[0] == "--config"
            && arguments[1].starts_with("projects={\"")
            && arguments[1].ends_with("\"={trust_level=\"untrusted\"}}")
    }));
    for expected in [
        "project_root_markers=[]",
        "project_doc_max_bytes=0",
        "mcp_servers={}",
        "features.hooks=false",
        "features.plugins=false",
        "features.apps=false",
        "sandbox_workspace_write.network_access=false",
        "sandbox_workspace_write.writable_roots=[]",
    ] {
        assert!(workspace
            .arguments
            .iter()
            .any(|argument| argument == expected));
    }
    assert!(!workspace
        .arguments
        .iter()
        .any(|argument| argument.contains("state-json")));
    service
        .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
        .unwrap();
    assert!(service
        .project_launch(&profile.id, &workspace_target)
        .is_err());
    service.authorize_danger_full_access(&profile.id).unwrap();
    service.cli = Arc::new(FakeCli::succeeding());
    let danger_target = NativeLaunchTarget::application_owned(root.clone(), false).unwrap();
    let danger = service.project_launch(&profile.id, &danger_target).unwrap();
    assert!(danger
        .arguments
        .windows(2)
        .any(|pair| pair == ["--sandbox", "danger-full-access"]));
    assert!(danger
        .arguments
        .iter()
        .any(|argument| argument == "--dangerously-bypass-approvals-and-sandbox"));
    for (index, argument) in danger.arguments.iter().enumerate() {
        if argument == "--config" {
            let value = danger.arguments.get(index + 1).unwrap();
            assert!(!value.starts_with('-'));
        }
    }
    let bypass = danger
        .arguments
        .iter()
        .position(|argument| argument == "--dangerously-bypass-approvals-and-sandbox")
        .unwrap();
    assert!(bypass == 0 || danger.arguments[bypass - 1] != "--config");
    assert!(!danger
        .arguments
        .iter()
        .any(|argument| argument.contains("state-json")));
    assert!(danger.non_interactive_approval);
    assert!(!danger.requested_network_disabled);
    assert!(!danger.effective_network_enforced);
    assert_eq!(danger.windows_uac_authority, "not_granted");
    let canary = service.project_full_access_canary(&profile.id).unwrap();
    assert_eq!(canary.evidence_state, "not_run");
    assert!(canary
        .sentinel_path
        .contains("native-full-access-canary.txt"));
    assert_ne!(canary.launch.working_root, root.to_string_lossy());
    let work_root = PathBuf::from(&canary.launch.working_root);
    let receipt = PathBuf::from(&canary.sentinel_path);
    let canary_parent = service.full_access_canary_parent(&profile.id).unwrap();
    assert!(!receipt.starts_with(&work_root));
    assert!(receipt.starts_with(&canary_parent));
}

#[test]
fn unsupported_cli_surface_fails_closed_before_launch_projection() {
    let (directory, mut service) = service();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
        .unwrap();
    service.authorize_danger_full_access(&profile.id).unwrap();
    service.cli = Arc::new(FakeCli::unsupported_surface());
    let root = directory.path().join("assigned-application-root");
    fs::create_dir_all(&root).unwrap();
    let target = NativeLaunchTarget::application_owned(root, true).unwrap();
    assert!(service.project_launch(&profile.id, &target).is_err());
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .attentions
            .cli,
        Some("codex_cli_surface_unsupported".into())
    );
}

#[test]
fn full_access_canary_persists_then_settles_only_the_owned_sentinel_receipt() {
    let (directory, mut service) = service();
    let fake = Arc::new(FakeCli::succeeding());
    *fake.next_child_result.lock().unwrap() = Some(NativeCliReceipt {
        succeeded: true,
        exit_code: Some(0),
        sandbox_receipt_observed: true,
    });
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
        .unwrap();
    service.authorize_danger_full_access(&profile.id).unwrap();
    service.run_danger_full_access_canary(&profile.id).unwrap();
    let call = fake.calls.lock().unwrap()[0].clone();
    assert_eq!(
        call.environment,
        native_windows_cli_environment(Path::new(&profile.home_path))
    );
    assert!(call.args.last().is_some_and(|prompt| {
        prompt.contains(FULL_ACCESS_CANARY_RECEIPT_RELATIVE_PATH)
            && prompt.contains(FULL_ACCESS_CANARY_RECEIPT)
            && !prompt.contains(&profile.home_path)
    }));
    let work_root = PathBuf::from(&call.cwd);
    let receipt = call.sandbox_receipt.unwrap();
    assert!(!receipt.starts_with(&work_root));
    assert!(receipt.starts_with(service.full_access_canary_parent(&profile.id).unwrap()));
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .danger_full_access_canary,
        "not_run"
    );
    service.reconcile_full_access_canary(&profile.id).unwrap();
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .danger_full_access_canary,
        "passed"
    );
    let persisted: String = service
        .connection()
        .unwrap()
        .query_row(
            "SELECT state FROM native_codex_profile_full_access_canaries WHERE profile_id=?1",
            params![profile.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(persisted, "passed");
    let durable = service
        .profile(&profile.id)
        .unwrap()
        .full_access_canary_attempt;
    assert_eq!(durable.process_activity, "terminal_observed");
    assert_eq!(durable.provider_activity, "unobserved");
    assert!(durable.receipt_observed);
    assert_eq!(durable.cleanup_disposition, "removed");
    assert_eq!(
        durable.authorization_version.as_deref(),
        Some(DANGER_AUTHORITY_VERSION)
    );
    assert!(directory.path().join("app").exists());
}

#[test]
fn unsupported_danger_network_policy_blocks_canary_before_a_child_starts() {
    let (_directory, mut service) = service();
    let fake = Arc::new(FakeCli::unsupported_danger_launch_surface());
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
        .unwrap();
    service.cli = Arc::new(FakeCli::succeeding());
    service.authorize_danger_full_access(&profile.id).unwrap();
    service.cli = fake.clone();
    assert!(service.run_danger_full_access_canary(&profile.id).is_err());
    assert_eq!(*fake.starts.lock().unwrap(), 0);
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .attentions
            .cli,
        Some("codex_cli_danger_launch_surface_unsupported".into())
    );
}

#[test]
fn malformed_full_access_canary_timestamps_and_terminal_shapes_fail_closed() {
    let (_directory, mut service) = service();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake;
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
        .unwrap();
    service.authorize_danger_full_access(&profile.id).unwrap();
    service.run_danger_full_access_canary(&profile.id).unwrap();
    service
            .connection()
            .unwrap()
            .execute(
                "UPDATE native_codex_profile_full_access_canaries SET launch_accepted_at='2000-01-01T00:00:00Z' WHERE profile_id=?1",
                params![profile.id],
            )
            .unwrap();
    assert!(service.query().is_err());
}

#[test]
fn current_full_access_canary_without_a_deadline_fails_closed() {
    let (_directory, mut service) = service();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake;
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
        .unwrap();
    service.authorize_danger_full_access(&profile.id).unwrap();
    service.run_danger_full_access_canary(&profile.id).unwrap();
    service
            .connection()
            .unwrap()
            .execute(
                "UPDATE native_codex_profile_full_access_canaries SET deadline_at=NULL WHERE profile_id=?1",
                params![profile.id],
            )
            .unwrap();
    assert!(service.query().is_err());
}

#[test]
fn continuity_loss_cancels_owned_login_and_sandbox_children() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service.request_login(&profile.id).unwrap();
    service.request_sandbox_initialization(&profile.id).unwrap();

    fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
    service.query().unwrap();

    assert_eq!(*fake.terminated.lock().unwrap(), 2);
    assert_eq!(
        service.profile(&profile.id).unwrap().lifecycle,
        Lifecycle::Malformed
    );
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .login_attempt
            .disposition,
        "cancelled"
    );
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .setup_attempt
            .disposition,
        "cancelled"
    );
}

#[test]
fn continuity_cancellation_cleans_a_reopened_workspace_canary_command_without_an_owned_child() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake;
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .update_readiness(&profile.id, None, Some("initialized"), None, None, None)
        .unwrap();
    service.run_workspace_write_canary(&profile.id).unwrap();
    let command_file = service
        .probe_root(&profile.id)
        .join(WORKSPACE_WRITE_CANARY_COMMAND_FILE);
    assert!(command_file.exists());

    // A cold reopen retains the durable pending attempt but not its in-memory child.
    drop(service);
    let reopened = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
    let query = reopened.query().unwrap();
    let invalidated = &query.profiles[0];

    assert_eq!(invalidated.lifecycle, Lifecycle::Malformed);
    assert!(!invalidated.selected);
    assert_eq!(invalidated.setup_attempt.disposition, "cancelled");
    assert_eq!(
        invalidated.setup_attempt.terminal_classification,
        "cancelled"
    );
    assert_eq!(invalidated.setup_attempt.terminal_exit_code, None);
    assert_eq!(invalidated.readiness.workspace_write_canary, "not_run");
    assert!(!command_file.exists());

    write_marker(Path::new(&profile.home_path), &profile.id).unwrap();
    assert!(reopened.select(&profile.id).is_err());
    assert_eq!(
        reopened
            .profile(&profile.id)
            .unwrap()
            .setup_attempt
            .disposition,
        "cancelled"
    );
}

#[test]
fn continuity_cancellation_cleans_a_reopened_full_access_receipt_without_an_owned_child() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
        .unwrap();
    service.authorize_danger_full_access(&profile.id).unwrap();
    service.run_danger_full_access_canary(&profile.id).unwrap();
    let receipt = fake.calls.lock().unwrap()[0]
        .sandbox_receipt
        .clone()
        .unwrap();
    fs::write(&receipt, FULL_ACCESS_CANARY_RECEIPT).unwrap();

    drop(service);
    let reopened = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
    let query = reopened.query().unwrap();
    let invalidated = &query.profiles[0];

    assert_eq!(
        invalidated.full_access_canary_attempt.disposition,
        "cancelled"
    );
    assert_eq!(
        invalidated.full_access_canary_attempt.cleanup_disposition,
        "removed"
    );
    assert!(!invalidated.full_access_canary_attempt.receipt_observed);
    assert_eq!(invalidated.readiness.danger_full_access_canary, "blocked");
    assert!(!receipt.exists());
    write_marker(Path::new(&profile.home_path), &profile.id).unwrap();
    assert!(reopened.select(&profile.id).is_err());
    assert_eq!(
        reopened
            .profile(&profile.id)
            .unwrap()
            .full_access_canary_attempt
            .disposition,
        "cancelled"
    );
}

#[test]
fn continuity_cancellation_records_full_access_receipt_cleanup_failure_without_deleting_broadly() {
    let directory = tempfile::tempdir().unwrap();
    let mut service = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let fake = Arc::new(FakeCli::succeeding());
    service.cli = fake.clone();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service
        .select_execution_mode(&profile.id, ExecutionMode::DangerFullAccess)
        .unwrap();
    service.authorize_danger_full_access(&profile.id).unwrap();
    service.run_danger_full_access_canary(&profile.id).unwrap();
    let receipt = fake.calls.lock().unwrap()[0]
        .sandbox_receipt
        .clone()
        .unwrap();
    fs::create_dir(&receipt).unwrap();

    drop(service);
    let reopened = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    fs::remove_file(Path::new(&profile.home_path).join(MARKER_FILE)).unwrap();
    let query = reopened.query().unwrap();
    let invalidated = &query.profiles[0];

    assert_eq!(
        invalidated.full_access_canary_attempt.disposition,
        "cancelled"
    );
    assert_eq!(
        invalidated.full_access_canary_attempt.cleanup_disposition,
        "failed"
    );
    assert_eq!(invalidated.readiness.danger_full_access_canary, "blocked");
    assert!(receipt.is_dir());
}

#[test]
fn continuity_loss_is_terminal_until_the_home_is_registered_again() {
    let (_directory, service) = service();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    let home = PathBuf::from(&profile.home_path);
    fs::remove_file(home.join(MARKER_FILE)).unwrap();
    service.query().unwrap();
    write_marker(&home, &profile.id).unwrap();
    let query = service.query().unwrap();
    assert_eq!(query.profiles[0].lifecycle, Lifecycle::Malformed);
    assert!(service.select(&profile.id).is_err());
    assert!(service.request_sandbox_initialization(&profile.id).is_err());
}

#[test]
fn one_attention_can_be_cleared_without_erasing_another_concern() {
    let (_directory, service) = service();
    let profile = service.create_dedicated().unwrap();
    service
        .set_attention(
            &profile.id,
            "sandbox",
            Some("sandbox_setup_failed_or_uac_attention_required"),
            false,
        )
        .unwrap();
    service
        .set_attention(
            &profile.id,
            "authentication",
            Some("browser_login_in_progress"),
            false,
        )
        .unwrap();
    service
        .set_attention(&profile.id, "authentication", None, false)
        .unwrap();
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .attentions
            .authentication,
        None
    );
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .attentions
            .sandbox,
        Some("sandbox_setup_failed_or_uac_attention_required".into())
    );
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .authentication,
        "unknown"
    );
}

#[test]
fn mcp_reporting_probe_changes_only_its_own_readiness_fact() {
    let (_directory, service) = service();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    let result = service.probe_mcp_reporting(&profile.id).unwrap();
    assert_eq!(result.readiness.authentication, "unknown");
    assert_eq!(result.readiness.sandbox_initialization, "unknown");
    assert_eq!(result.readiness.workspace_write_canary, "not_run");
    assert_eq!(result.readiness.mcp_reporting, "not_assessed");
    assert_eq!(
        result.readiness.attentions.mcp_reporting,
        Some("mcp_reporting_probe_pending_application_receipt".into())
    );
}

#[test]
fn reporting_dispatch_calls_the_exact_mcp_tool_then_settles_the_real_receipt() {
    let (_directory, mut service) = service();
    let profile = selected_profile_ready_except_mcp(&service);
    service.begin_mcp_reporting_probe(&profile.id).unwrap();
    let reporting = Arc::new(ReportingCli::new());
    service.cli = reporting.clone();

    let ready = service
        .reconcile_pending_mcp_reporting(&profile.id)
        .unwrap();

    assert_eq!(ready.readiness.mcp_reporting, "ready");
    assert!(service.resolve_selected_home().is_ok());
    let calls = reporting.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    let call = &calls[0];
    assert_eq!(call.codex_home, PathBuf::from(&profile.home_path));
    assert!(call.args.iter().any(|argument| {
        argument
            == &format!("mcp_servers.{MCP_REPORTING_SERVER}.enabled_tools=[{MCP_REPORTING_TOOL:?}]")
    }));
    assert!(call
        .args
        .last()
        .is_some_and(|prompt| prompt.contains(MCP_REPORTING_CAPABILITY)));
}

#[test]
fn reporting_dispatch_terminalizes_no_receipt_without_reopen_or_duplicate_retry() {
    let (directory, service) = service();
    let profile = selected_profile_ready_except_mcp(&service);
    service.begin_mcp_reporting_probe(&profile.id).unwrap();
    assert!(service
        .reconcile_pending_mcp_reporting(&profile.id)
        .is_err());
    let incomplete = service.profile(&profile.id).unwrap();
    assert_eq!(incomplete.readiness.mcp_reporting, "probe_failed");
    assert_eq!(
        incomplete.readiness.attentions.mcp_reporting.as_deref(),
        Some("mcp_reporting_dispatch_completed_without_receipt")
    );
    assert_eq!(
        service
            .connection()
            .unwrap()
            .query_row(
                "SELECT state FROM native_codex_profile_mcp_probes WHERE profile_id=?1",
                params![profile.id],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "cancelled"
    );
    drop(service);

    let mut reopened = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let reporting = Arc::new(ReportingCli::new());
    reopened.cli = reporting.clone();
    reopened
        .reconcile_pending_mcp_reporting(&profile.id)
        .unwrap();
    reopened
        .reconcile_pending_mcp_reporting(&profile.id)
        .unwrap();
    assert!(reporting.calls.lock().unwrap().is_empty());
    assert!(reopened.resolve_selected_home().is_err());
}

#[test]
fn reporting_dispatch_requires_the_current_selected_profile_before_mcp_exposure() {
    let (_directory, mut service) = service();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    service.begin_mcp_reporting_probe(&profile.id).unwrap();
    let other = service.create_dedicated().unwrap();
    service.select(&other.id).unwrap();
    let reporting = Arc::new(ReportingCli::new());
    service.cli = reporting.clone();

    assert!(service
        .reconcile_pending_mcp_reporting(&profile.id)
        .is_err());
    assert!(reporting.calls.lock().unwrap().is_empty());
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .mcp_reporting,
        "not_assessed"
    );
}

#[test]
fn reporting_reconciliation_without_a_request_is_a_noop() {
    let (_directory, mut service) = service();
    let profile = selected_profile_ready_except_mcp(&service);
    let reporting = Arc::new(ReportingCli::new());
    service.cli = reporting.clone();

    service
        .reconcile_pending_mcp_reporting(&profile.id)
        .unwrap();

    assert!(reporting.calls.lock().unwrap().is_empty());
    assert_eq!(
        service
            .connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM native_codex_profile_mcp_probes WHERE profile_id=?1",
                params![profile.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        0
    );
}

#[test]
fn concurrent_reporting_dispatches_adopt_one_pending_request() {
    let (_directory, mut service) = service();
    let profile = selected_profile_ready_except_mcp(&service);
    service.begin_mcp_reporting_probe(&profile.id).unwrap();
    let reporting = Arc::new(ReportingCli::new());
    service.cli = reporting.clone();
    let service = Arc::new(service);
    let barrier = Arc::new(Barrier::new(2));
    let joins = (0..2)
        .map(|_| {
            let service = service.clone();
            let profile_id = profile.id.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                service.reconcile_pending_mcp_reporting(&profile_id)
            })
        })
        .collect::<Vec<_>>();
    assert!(joins.into_iter().all(|join| join.join().unwrap().is_ok()));
    assert_eq!(reporting.calls.lock().unwrap().len(), 1);
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .mcp_reporting,
        "ready"
    );
}

#[test]
fn claimed_reporting_dispatch_reopens_without_a_second_exchange() {
    let (directory, service) = service();
    let profile = selected_profile_ready_except_mcp(&service);
    service.begin_mcp_reporting_probe(&profile.id).unwrap();
    let claimed = service
        .claim_pending_mcp_reporting_probe(&profile.id)
        .unwrap()
        .expect("pending request is claimed");
    assert!(!claimed.claim_id.is_empty());
    drop(service);

    let mut reopened = NativeProfileService::open(
        directory.path().join("active.sqlite"),
        directory.path().join("app"),
    )
    .unwrap();
    let reporting = Arc::new(ReportingCli::new());
    reopened.cli = reporting.clone();
    reopened
        .reconcile_pending_mcp_reporting(&profile.id)
        .unwrap();

    assert!(reporting.calls.lock().unwrap().is_empty());
    assert_eq!(
        reopened
            .connection()
            .unwrap()
            .query_row(
                "SELECT state FROM native_codex_profile_mcp_probes WHERE profile_id=?1",
                params![profile.id],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "dispatching"
    );
    assert_eq!(
        reopened
            .profile(&profile.id)
            .unwrap()
            .readiness
            .attentions
            .mcp_reporting
            .as_deref(),
        Some("mcp_reporting_dispatch_claimed_receipt_pending")
    );
}

#[test]
fn two_services_claim_one_reporting_exchange() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let app = directory.path().join("app");
    let mut seed = NativeProfileService::open(database.clone(), app.clone()).unwrap();
    seed.cli = Arc::new(FakeCli::succeeding());
    let profile = selected_profile_ready_except_mcp(&seed);
    seed.begin_mcp_reporting_probe(&profile.id).unwrap();
    drop(seed);

    let mut first = NativeProfileService::open(database.clone(), app.clone()).unwrap();
    let first_reporting = Arc::new(ReportingCli::new());
    first.cli = first_reporting.clone();
    let mut second = NativeProfileService::open(database.clone(), app.clone()).unwrap();
    let second_reporting = Arc::new(ReportingCli::new());
    second.cli = second_reporting.clone();
    let barrier = Arc::new(Barrier::new(2));
    let joins = [first, second]
        .into_iter()
        .map(|service| {
            let barrier = barrier.clone();
            let profile_id = profile.id.clone();
            thread::spawn(move || {
                barrier.wait();
                service.reconcile_pending_mcp_reporting(&profile_id)
            })
        })
        .collect::<Vec<_>>();

    assert!(joins.into_iter().all(|join| join.join().unwrap().is_ok()));
    assert_eq!(
        first_reporting.calls.lock().unwrap().len() + second_reporting.calls.lock().unwrap().len(),
        1
    );
    let reopened = NativeProfileService::open(database, app).unwrap();
    assert_eq!(
        reopened
            .profile(&profile.id)
            .unwrap()
            .readiness
            .mcp_reporting,
        "ready"
    );
}

#[test]
fn mcp_receipts_require_a_current_dispatch_claim_and_matching_authority() {
    let (_directory, service) = service();
    let profile = selected_profile_ready_except_mcp(&service);
    let authority = service.begin_mcp_reporting_probe(&profile.id).unwrap();
    assert!(service
        .record_mcp_reporting_receipt(
            &profile.id,
            &NativeMcpReportingReceipt {
                capability: authority.capability.clone(),
                server: authority.server.clone(),
                tool: authority.tool.clone(),
                correlation_id: String::new(),
                probe_root: authority.probe_root.clone(),
            },
        )
        .is_err());
    assert!(service
        .record_mcp_reporting_receipt(
            &profile.id,
            &NativeMcpReportingReceipt {
                capability: authority.capability.clone(),
                server: authority.server.clone(),
                tool: authority.tool.clone(),
                correlation_id: authority.correlation_id.clone(),
                probe_root: authority.probe_root.clone(),
            },
        )
        .is_err());
    assert_eq!(
        service
            .connection()
            .unwrap()
            .query_row(
                "SELECT state FROM native_codex_profile_mcp_probes WHERE profile_id=?1",
                params![profile.id],
                |row| row.get::<_, String>(0),
            )
            .unwrap(),
        "pending"
    );
    let authority = claim_mcp_reporting_for_current_reconciliation(&service, &profile.id);
    assert!(service
        .record_mcp_reporting_receipt(
            &profile.id,
            &NativeMcpReportingReceipt {
                capability: authority.capability.clone(),
                server: authority.server.clone(),
                tool: authority.tool.clone(),
                correlation_id: String::new(),
                probe_root: authority.probe_root.clone(),
            },
        )
        .is_err());
    let ready = service
        .record_mcp_reporting_receipt(
            &profile.id,
            &NativeMcpReportingReceipt {
                capability: authority.capability.clone(),
                server: authority.server.clone(),
                tool: authority.tool.clone(),
                correlation_id: authority.correlation_id.clone(),
                probe_root: authority.probe_root.clone(),
            },
        )
        .unwrap();
    assert_eq!(ready.readiness.mcp_reporting, "ready");
    assert!(service
        .record_mcp_reporting_receipt(
            &profile.id,
            &NativeMcpReportingReceipt {
                capability: authority.capability,
                server: authority.server,
                tool: authority.tool,
                correlation_id: authority.correlation_id,
                probe_root: authority.probe_root,
            },
        )
        .is_err());
}

#[test]
fn direct_mcp_receipts_cannot_settle_a_pending_probe_across_service_instances() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let app = directory.path().join("app");
    let service = NativeProfileService::open(database.clone(), app.clone()).unwrap();
    let profile = selected_profile_ready_except_mcp(&service);
    let authority = service.begin_mcp_reporting_probe(&profile.id).unwrap();
    drop(service);
    let receipt = NativeMcpReportingReceipt {
        capability: authority.capability,
        server: authority.server,
        tool: authority.tool,
        correlation_id: authority.correlation_id,
        probe_root: authority.probe_root,
    };
    let barrier = Arc::new(Barrier::new(2));
    let mut joins = vec![];
    for _ in 0..2 {
        let database = database.clone();
        let app = app.clone();
        let profile_id = profile.id.clone();
        let receipt = receipt.clone();
        let barrier = barrier.clone();
        joins.push(thread::spawn(move || {
            let service = NativeProfileService::open(database, app).unwrap();
            barrier.wait();
            service.record_mcp_reporting_receipt(&profile_id, &receipt)
        }));
    }
    let outcomes = joins
        .into_iter()
        .map(|join| join.join().unwrap().is_ok())
        .collect::<Vec<_>>();
    assert_eq!(outcomes.into_iter().filter(|success| *success).count(), 0);
    let reopened = NativeProfileService::open(database, app).unwrap();
    assert_eq!(
        reopened
            .profile(&profile.id)
            .unwrap()
            .readiness
            .mcp_reporting,
        "not_assessed"
    );
}

#[test]
fn mcp_receipt_settlement_requires_a_currently_selected_active_profile() {
    let (_directory, service) = service();
    let profile = selected_profile_ready_except_mcp(&service);
    service.begin_mcp_reporting_probe(&profile.id).unwrap();
    let authority = claim_mcp_reporting_for_current_reconciliation(&service, &profile.id);
    let other = service.create_dedicated().unwrap();
    service.select(&other.id).unwrap();

    assert!(service
        .record_mcp_reporting_receipt(
            &profile.id,
            &NativeMcpReportingReceipt {
                capability: authority.capability.clone(),
                server: authority.server.clone(),
                tool: authority.tool.clone(),
                correlation_id: authority.correlation_id.clone(),
                probe_root: authority.probe_root.clone(),
            },
        )
        .is_err());

    service.select(&profile.id).unwrap();
    fs::remove_dir_all(&profile.home_path).unwrap();
    assert!(service
        .record_mcp_reporting_receipt(
            &profile.id,
            &NativeMcpReportingReceipt {
                capability: authority.capability,
                server: authority.server,
                tool: authority.tool,
                correlation_id: authority.correlation_id,
                probe_root: authority.probe_root,
            },
        )
        .is_err());
}

#[test]
fn cancelled_or_expired_probe_cannot_set_mcp_ready() {
    let (_directory, service) = service();
    let profile = selected_profile_ready_except_mcp(&service);
    service.begin_mcp_reporting_probe(&profile.id).unwrap();
    let authority = claim_mcp_reporting_for_current_reconciliation(&service, &profile.id);
    service
            .connection()
            .unwrap()
            .execute(
                "UPDATE native_codex_profile_mcp_probes SET state='cancelled' WHERE profile_id=?1 AND state='dispatching'",
                params![profile.id],
            )
            .unwrap();
    assert!(service
        .record_mcp_reporting_receipt(
            &profile.id,
            &NativeMcpReportingReceipt {
                capability: authority.capability,
                server: authority.server,
                tool: authority.tool,
                correlation_id: authority.correlation_id,
                probe_root: authority.probe_root,
            },
        )
        .is_err());
    assert_ne!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .mcp_reporting,
        "ready"
    );
}

#[test]
fn terminal_mcp_probe_history_allows_one_fresh_pending_request_without_mutation() {
    for (terminal_state, received_at) in [
        ("expired", None),
        ("received", Some("2026-08-07T12:01:00Z")),
        ("cancelled", None),
    ] {
        let (_directory, service) = service();
        let profile = service.create_dedicated().unwrap();
        service.select(&profile.id).unwrap();
        let historical = service.begin_mcp_reporting_probe(&profile.id).unwrap();
        service
                .connection()
                .unwrap()
                .execute(
                    "UPDATE native_codex_profile_mcp_probes SET state=?1,received_at=?2 WHERE profile_id=?3 AND correlation_id=?4",
                    params![terminal_state, received_at, profile.id, historical.correlation_id],
                )
                .unwrap();
        let connection = service.connection().unwrap();
        let before = connection
                .query_row(
                    "SELECT request_id,correlation_id,state,requested_at,deadline_at,received_at FROM native_codex_profile_mcp_probes WHERE profile_id=?1 AND correlation_id=?2",
                    params![profile.id, historical.correlation_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, Option<String>>(5)?)),
                )
                .unwrap();

        let fresh = service.begin_mcp_reporting_probe(&profile.id).unwrap();

        let after = connection
                .query_row(
                    "SELECT request_id,correlation_id,state,requested_at,deadline_at,received_at FROM native_codex_profile_mcp_probes WHERE profile_id=?1 AND correlation_id=?2",
                    params![profile.id, historical.correlation_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, Option<String>>(5)?)),
                )
                .unwrap();
        assert_eq!(after, before, "{terminal_state} history changed");
        assert_ne!(fresh.correlation_id, historical.correlation_id);
        assert_eq!(
                connection
                    .query_row(
                        "SELECT COUNT(*) FROM native_codex_profile_mcp_probes WHERE profile_id=?1 AND state='pending'",
                        params![profile.id],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap(),
                1,
                "{terminal_state} history did not leave exactly one current pending request"
            );
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM native_codex_profile_mcp_probes WHERE profile_id=?1",
                    params![profile.id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            2
        );
    }
}

#[test]
fn mcp_probe_creation_requires_the_current_selected_active_profile() {
    let (_directory, service) = service();
    let first = service.create_dedicated().unwrap();
    let second = service.create_dedicated().unwrap();
    assert_eq!(
        service.profile(&second.id).unwrap().lifecycle,
        Lifecycle::Active
    );
    assert!(!service.profile(&first.id).unwrap().selected);

    assert!(service.begin_mcp_reporting_probe(&first.id).is_err());
    assert_eq!(
        service
            .connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM native_codex_profile_mcp_probes WHERE profile_id=?1",
                params![first.id],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        0
    );
}

#[test]
fn selection_move_prevents_a_competing_unselected_mcp_probe_creation() {
    let (_directory, service) = service();
    let service = Arc::new(service);
    let first = service.create_dedicated().unwrap();
    service.select(&first.id).unwrap();
    let second = service.create_dedicated().unwrap();
    let (creation_ready_sender, creation_ready_receiver) = std::sync::mpsc::channel();
    let (selection_start_sender, selection_start_receiver) = std::sync::mpsc::channel();
    let (selection_committed_sender, selection_committed_receiver) = std::sync::mpsc::channel();
    let (creation_continue_sender, creation_continue_receiver) = std::sync::mpsc::channel();

    let selecting = {
        let service = service.clone();
        let second_id = second.id.clone();
        thread::spawn(move || {
            selection_start_receiver.recv().unwrap();
            let result = service.select(&second_id);
            selection_committed_sender.send(()).unwrap();
            result
        })
    };
    let creating = {
        let service = service.clone();
        let first_id = first.id.clone();
        thread::spawn(move || {
            creation_ready_sender.send(()).unwrap();
            creation_continue_receiver.recv().unwrap();
            service.begin_mcp_reporting_probe(&first_id)
        })
    };

    creation_ready_receiver.recv().unwrap();
    selection_start_sender.send(()).unwrap();
    selection_committed_receiver.recv().unwrap();
    creation_continue_sender.send(()).unwrap();
    selecting.join().unwrap().unwrap();
    assert!(creating.join().unwrap().is_err());
    assert_eq!(
            service
                .connection()
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM native_codex_profile_mcp_probes WHERE profile_id=?1 AND state IN ('pending','dispatching')",
                    params![first.id],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
}

#[test]
fn concurrent_terminal_history_replacement_converges_on_one_current_request() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let app = directory.path().join("app");
    let service = NativeProfileService::open(database.clone(), app.clone()).unwrap();
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    let historical = service.begin_mcp_reporting_probe(&profile.id).unwrap();
    service
            .connection()
            .unwrap()
            .execute(
                "UPDATE native_codex_profile_mcp_probes SET state='expired' WHERE profile_id=?1 AND correlation_id=?2",
                params![profile.id, historical.correlation_id],
            )
            .unwrap();
    drop(service);
    let barrier = Arc::new(Barrier::new(2));
    let mut joins = vec![];
    for _ in 0..2 {
        let database = database.clone();
        let app = app.clone();
        let profile_id = profile.id.clone();
        let barrier = barrier.clone();
        joins.push(thread::spawn(move || {
            let service = NativeProfileService::open(database, app).unwrap();
            barrier.wait();
            service.begin_mcp_reporting_probe(&profile_id)
        }));
    }
    let first = joins.remove(0).join().unwrap().unwrap();
    let second = joins.remove(0).join().unwrap().unwrap();
    assert_eq!(first, second);
    let reopened = NativeProfileService::open(database, app).unwrap();
    let connection = reopened.connection().unwrap();
    assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*),COUNT(CASE WHEN state IN ('pending','dispatching') THEN 1 END),COUNT(CASE WHEN state='pending' THEN 1 END) FROM native_codex_profile_mcp_probes WHERE profile_id=?1",
                    params![profile.id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)),
                )
                .unwrap(),
            (2, 1, 1)
        );
    assert_eq!(
            connection
                .query_row(
                    "SELECT state FROM native_codex_profile_mcp_probes WHERE profile_id=?1 AND correlation_id=?2",
                    params![profile.id, historical.correlation_id],
                    |row| row.get::<_, String>(0),
                )
                .unwrap(),
            "expired"
        );
}

#[test]
fn foreign_and_stale_mcp_probe_receipts_are_rejected_without_readiness_success() {
    let (_directory, service) = service();
    let first = selected_profile_ready_except_mcp(&service);
    let second = service.create_dedicated().unwrap();
    service.begin_mcp_reporting_probe(&first.id).unwrap();
    let authority = claim_mcp_reporting_for_current_reconciliation(&service, &first.id);
    assert!(service
        .record_mcp_reporting_receipt(
            &second.id,
            &NativeMcpReportingReceipt {
                capability: authority.capability.clone(),
                server: authority.server.clone(),
                tool: authority.tool.clone(),
                correlation_id: authority.correlation_id.clone(),
                probe_root: authority.probe_root.clone(),
            },
        )
        .is_err());
    let connection = service.connection().unwrap();
    connection
            .execute(
                "UPDATE native_codex_profile_mcp_probes SET deadline_at='2000-01-01T00:00:00+00:00' WHERE profile_id=?1",
                params![first.id],
            )
            .unwrap();
    service.query().unwrap();
    assert!(service
        .record_mcp_reporting_receipt(
            &first.id,
            &NativeMcpReportingReceipt {
                capability: authority.capability,
                server: authority.server,
                tool: authority.tool,
                correlation_id: authority.correlation_id,
                probe_root: authority.probe_root,
            },
        )
        .is_err());
    assert_eq!(
        service.profile(&first.id).unwrap().readiness.mcp_reporting,
        "not_assessed"
    );
}

#[test]
fn pending_mcp_probe_reopens_with_the_same_private_correlation() {
    let directory = tempfile::tempdir().unwrap();
    let database = directory.path().join("active.sqlite");
    let service =
        NativeProfileService::open(database.clone(), directory.path().join("app")).unwrap();
    let profile = selected_profile_ready_except_mcp(&service);
    let authority = service.begin_mcp_reporting_probe(&profile.id).unwrap();
    drop(service);
    let reopened = NativeProfileService::open(database, directory.path().join("app")).unwrap();
    let retained = reopened.begin_mcp_reporting_probe(&profile.id).unwrap();
    assert_eq!(retained, authority);
    let claimed = claim_mcp_reporting_for_current_reconciliation(&reopened, &profile.id);
    assert_eq!(claimed, retained);
    let ready = reopened
        .record_mcp_reporting_receipt(
            &profile.id,
            &NativeMcpReportingReceipt {
                capability: claimed.capability,
                server: claimed.server,
                tool: claimed.tool,
                correlation_id: claimed.correlation_id,
                probe_root: claimed.probe_root,
            },
        )
        .unwrap();
    assert_eq!(ready.readiness.mcp_reporting, "ready");
}

#[test]
fn unavailable_cli_is_profile_attention_not_composition_failure() {
    let (_directory, mut service) = service();
    struct UnavailableCli;
    impl NativeCliPort for UnavailableCli {
        fn run(&self, _: &NativeCliInvocation) -> Result<NativeCliReceipt, String> {
            Err("missing".into())
        }
        fn start(&self, _: &NativeCliInvocation) -> Result<Box<dyn NativeCliChild>, String> {
            Err("missing".into())
        }
        fn surface(&self) -> Result<NativeCliSurface, String> {
            Err("missing".into())
        }
    }
    service.cli = Arc::new(UnavailableCli);
    let profile = service.create_dedicated().unwrap();
    service.select(&profile.id).unwrap();
    assert!(service.refresh_readiness(&profile.id).is_err());
    assert_eq!(
        service
            .profile(&profile.id)
            .unwrap()
            .readiness
            .attentions
            .cli,
        Some("codex_cli_unavailable".into())
    );
}
