use super::*;
use crate::agent_sessions::{
    application::import::AgentSessionImportService, imports::*, ports::*,
    workspace::SessionWorkspaces,
};
use crate::execution_configuration::{
    CapabilityProfileService, InMemoryCapabilityProfileRepository,
};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Home(ImportHome);
impl ImportHomeSource for Home {
    fn selected_import_home(&self) -> Result<ImportHome, String> {
        Ok(self.0.clone())
    }
}
struct History {
    source: ImportedThread,
    forks: AtomicUsize,
    fail: bool,
}
impl CodexHistorySource for History {
    fn read(&self, _: &ImportHome, _: &str) -> Result<ImportedThread, String> {
        Ok(self.source.clone())
    }
    fn fork(&self, _: &ImportHome, _: &str, _: &str, cwd: &str) -> Result<ImportedThread, String> {
        self.forks.fetch_add(1, Ordering::SeqCst);
        assert!(std::path::Path::new(cwd).is_dir());
        if self.fail {
            return Err("Fork acknowledgement lost".into());
        }
        let mut fork = self.source.clone();
        fork.id = "import-fork".into();
        Ok(fork)
    }
}
fn service(
    root: &std::path::Path,
    fail: bool,
) -> (AgentSessionImportService, Arc<History>, Arc<FakeRuntime>) {
    let conn = crate::storage::open_active_database(&root.join("db.sqlite")).unwrap();
    conn.execute("INSERT INTO native_codex_profiles VALUES ('profile','home','identity','registered_existing','active','now','now','now')",[]).unwrap();
    let repository = Arc::new(SqliteAgentSessionRepository::new(conn).unwrap());
    let snapshot = test_selected_runtime_profile();
    let source = Arc::new(FixedProviderConfigurationSource(snapshot.clone()));
    let profiles = Arc::new(CapabilityProfileService::new(
        Arc::new(InMemoryCapabilityProfileRepository::default()),
        source.clone(),
    ));
    profiles
        .create("capabilities".into(), "Default".into(), snapshot.exposure)
        .unwrap();
    profiles.set_default_profile("capabilities").unwrap();
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::CompleteWithBinding));
    let providers = Arc::new(crate::agent_sessions::application::SystemAgentSessionProviders);
    let application = Arc::new(
        AgentSessionApplication::new(
            repository.clone(),
            runtime.clone(),
            Arc::new(RecordingNotifier::new(repository.clone())),
            providers.clone(),
            providers,
            None,
        )
        .with_profile_source(source)
        .with_capability_profiles(profiles)
        .with_workspaces(SessionWorkspaces::new(root.join("orchid"), "test".into()).unwrap()),
    );
    let history = Arc::new(History {
        source: ImportedThread {
            id: "01a0a149-469d-74d3-9924-e7415b61a4e9".into(),
            title: "Source".into(),
            cwd: Some(root.join("missing").to_string_lossy().into_owned()),
            version: None,
            turns: vec![ImportedTurn {
                id: "turn".into(),
                status: AgentInvocationStatus::Completed,
                started_at: None,
                completed_at: None,
                items: vec![ImportedItem {
                    kind: "user".into(),
                    text: "Context".into(),
                    raw: serde_json::json!({}),
                    normalized: None,
                }],
            }],
        },
        forks: AtomicUsize::new(0),
        fail,
    });
    (
        AgentSessionImportService {
            application,
            homes: Arc::new(Home(ImportHome {
                profile_id: "profile".into(),
                filesystem_identity: "identity".into(),
                path: "home".into(),
            })),
            history: history.clone(),
            store: repository,
            lane: Mutex::new(()),
        },
        history,
        runtime,
    )
}
fn command() -> ImportCommand {
    ImportCommand {
        request_id: uuid::Uuid::new_v4().to_string(),
        link: "codex://threads/01a0a149-469d-74d3-9924-e7415b61a4e9".into(),
        profile_id: "profile".into(),
        last_turn_id: "turn".into(),
    }
}
#[test]
fn import_prepares_default_profile_empty_workspace_and_reuses_completed_request() {
    let root = tempfile::tempdir().unwrap();
    let (service, history, runtime) = service(root.path(), false);
    let cmd = command();
    let preview = service.preview(&cmd.link).unwrap();
    assert!(preview.allocate_workspace);
    assert_eq!(preview.capability_profile, "Default");
    let id = service.import(cmd.clone()).unwrap();
    assert_eq!(id, service.import(cmd).unwrap());
    assert_eq!(history.forks.load(Ordering::SeqCst), 1);
    assert!(runtime.calls.lock().unwrap().is_empty());
    let saved = service.application.load_session(&id).unwrap();
    assert!(saved.session.session_profile.is_some());
    assert_eq!(saved.session.workspace_origin.as_deref(), Some("allocated"));
    assert_eq!(
        saved
            .session
            .runtime_binding
            .external_context_id
            .as_ref()
            .unwrap()
            .as_str(),
        "import-fork"
    );
    assert!(std::path::Path::new(saved.session.working_directory.as_ref().unwrap()).is_dir());
    service
        .application
        .send_message(message(&id, "Continue"))
        .unwrap();
    assert!(runtime
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|call| matches!(call, RuntimeCall::Resume(_,id) if id=="import-fork")));
}
#[test]
fn uncertain_fork_is_not_repeated_and_changed_home_does_not_fork() {
    let root = tempfile::tempdir().unwrap();
    let (service, history, _) = service(root.path(), true);
    let mut cmd = command();
    cmd.profile_id = "different".into();
    assert!(service.import(cmd.clone()).is_err());
    assert_eq!(history.forks.load(Ordering::SeqCst), 0);
    cmd.profile_id = "profile".into();
    assert!(service.import(cmd.clone()).is_err());
    assert!(service.import(cmd).unwrap_err().contains("uncertain"));
    assert_eq!(history.forks.load(Ordering::SeqCst), 1);
}
#[test]
#[ignore = "Run through the isolated Node app-server contract fixture"]
fn installed_codex_import_reopens_and_continues_through_orchid() {
    use crate::runtime::providers::codex::profiles::NativeProfileService;
    use crate::runtime::providers::codex::app_server::{history::CodexHistoryReader, CodexAppServerRuntime};
    let home = std::env::var("ORCHID_IMPORT_CONTRACT_HOME").unwrap();
    let root = std::path::PathBuf::from(std::env::var("ORCHID_IMPORT_CONTRACT_ROOT").unwrap());
    let program = std::env::var("CODEX_APP_SERVER_CONTRACT_PROGRAM").unwrap();
    let source_id = std::env::var("ORCHID_IMPORT_CONTRACT_THREAD").unwrap();
    let build = || {
        let connection = crate::storage::open_active_database(&root.join("orchid.sqlite")).unwrap();
        let database = Arc::new(
            crate::persistence::ActiveDatabase::from_connection(connection, |_| Ok(())).unwrap(),
        );
        let repository = Arc::new(SqliteAgentSessionRepository::from_database(
            database.clone(),
        ));
        let native = Arc::new(NativeProfileService::new(database, root.join("app-data")));
        if native.selected_import_home().is_err() {
            let profile = native.register_existing(&home).unwrap();
            native
                .select(
                    serde_json::to_value(profile).unwrap()["id"]
                        .as_str()
                        .unwrap(),
                )
                .unwrap();
        }
        let mut snapshot = test_selected_runtime_profile();
        snapshot.exposure.models = ["gpt-5.6-terra".into()].into_iter().collect();
        snapshot.exposure.sandbox_modes = [ExecutionSandboxMode::ReadOnly].into_iter().collect();
        snapshot.locked.sandbox_mode = Some(ExecutionSandboxMode::ReadOnly);
        snapshot.exposure.reasoning_modes = ["low".into()].into_iter().collect();
        let source = Arc::new(FixedProviderConfigurationSource(snapshot.clone()));
        let profiles = Arc::new(CapabilityProfileService::new(
            Arc::new(InMemoryCapabilityProfileRepository::default()),
            source.clone(),
        ));
        profiles
            .create("capabilities".into(), "Default".into(), snapshot.exposure)
            .unwrap();
        profiles.set_default_profile("capabilities").unwrap();
        let providers = Arc::new(crate::agent_sessions::application::SystemAgentSessionProviders);
        let application = Arc::new(
            AgentSessionApplication::new(
                repository.clone(),
                Arc::new(CodexAppServerRuntime::system(&program)),
                Arc::new(RecordingNotifier::new(repository.clone())),
                providers.clone(),
                providers,
                None,
            )
            .with_profile_source(source)
            .with_capability_profiles(profiles)
            .with_workspaces(
                SessionWorkspaces::new(root.join("workspaces"), "contract".into()).unwrap(),
            )
            .with_native_profile_launch_authority(native.clone()),
        );
        AgentSessionImportService {
            application,
            homes: native,
            history: Arc::new(CodexHistoryReader(program.clone())),
            store: repository,
            lane: Mutex::new(()),
        }
    };
    let service = build();
    let link = format!("codex://threads/{source_id}");
    let preview = service.preview(&link).unwrap();
    let cmd = ImportCommand {
        request_id: uuid::Uuid::new_v4().to_string(),
        link,
        profile_id: preview.profile_id,
        last_turn_id: preview.last_turn_id,
    };
    let id = service.import(cmd.clone()).unwrap();
    let before = service.application.load_session(&id).unwrap();
    assert_eq!(before.invocations.len(), 1);
    assert!(before.invocations[0]
        .invocation
        .submitted_text
        .contains("IMPORT_CONTEXT_MARKER"));
    assert!(before.invocations[0].launch_accepted_at.is_none());
    let binding = before
        .session
        .runtime_binding
        .external_context_id
        .clone()
        .unwrap();
    assert_ne!(binding.as_str(), source_id);
    drop(service);
    let service = build();
    assert_eq!(service.import(cmd).unwrap(), id);
    assert_eq!(service.application.load_session(&id).unwrap(), before);
    service
        .application
        .send_direct_user_message(SendDirectUserAgentSessionMessageCommand {
            session_id: id.clone(),
            submitted_text: "Continue with the remembered context in Orchid.".into(),
            model: None,
            reasoning_mode: None,
            sandbox_mode: None,
        })
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let history = service.application.load_session(&id).unwrap();
        if history.invocations.len() == 2 {
            let latest = &history.invocations[1].invocation;
            if latest.status == AgentInvocationStatus::Completed {
                break;
            }
            assert!(
                !matches!(
                    latest.status,
                    AgentInvocationStatus::Failed | AgentInvocationStatus::Interrupted
                ),
                "Continuation failed: {latest:?}"
            );
        }
        assert!(
            std::time::Instant::now() < deadline,
            "Orchid continuation did not complete"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let native_home = service.homes.selected_import_home().unwrap();
    assert_eq!(
        service
            .history
            .read(&native_home, &source_id)
            .unwrap()
            .turns
            .len(),
        1
    );
    assert_eq!(
        service
            .history
            .read(&native_home, binding.as_str())
            .unwrap()
            .turns
            .len(),
        2
    );
    std::fs::write(
        root.join("orchid-import-result.json"),
        serde_json::to_vec(
            &serde_json::json!({"sessionId":id.as_str(), "forkId":binding.as_str()}),
        )
        .unwrap(),
    )
    .unwrap();
}
