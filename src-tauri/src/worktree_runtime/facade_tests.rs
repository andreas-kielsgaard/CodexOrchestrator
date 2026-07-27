use super::{
    application::{
        PrepareInstanceCommand, ReadInstanceQuery, RecoverInstanceCommand, RuntimeApplicationError,
        RuntimeApplicationErrorKind, StartInstanceCommand, StopInstanceCommand,
        WorktreeRuntimeControl,
    },
    domain::{
        CacheReuse, EndpointObservation, HealthObservation, InstanceRecord, InstanceSnapshot,
        InstanceState, OwnerObservation, RuntimeObservation,
    },
    execution::{ActionExecution, ActionExecutor, ExecutionError, SystemActionExecutor},
    facade::{
        IsolatedTestRequest, TestActionOutcome, TestInstanceError, TestInstanceErrorKind,
        TestInstancePhase, TestSourceRef, TestSourceResolver, WorktreeTestInstanceFacade,
        WorktreeTestInstances,
    },
    planning::{
        ActionKind, ActionPlan, PlanningError, ProcessCommand, RuntimeSettings, SourceInspector,
        SourceSnapshot, SystemSourceInspector, ToolchainPrograms,
    },
};
use chrono::Utc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

#[test]
fn semantic_facade_projects_and_controls_two_isolated_instances() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let alpha_path = fixture_worktree(directory.path(), "alpha");
    let beta_path = fixture_worktree(directory.path(), "beta");
    let runtime = Arc::new(FakeRuntime::default());
    let executor = Arc::new(RecordingExecutor::default());
    let facade = facade(
        directory.path(),
        runtime.clone(),
        Arc::new(MapSources(HashMap::from([
            ("repository/alpha".into(), alpha_path),
            ("repository/beta".into(), beta_path),
        ]))),
        Arc::new(FixedInspector),
        executor.clone(),
        directory.path().join("shared-cache"),
    );

    let alpha = facade
        .request(
            IsolatedTestRequest::new(
                TestSourceRef::new("repository/alpha").expect("source"),
                "feature verification",
            )
            .expect("request"),
        )
        .expect("request alpha");
    let beta = facade
        .request(
            IsolatedTestRequest::new(
                TestSourceRef::new("repository/beta").expect("source"),
                "feature verification",
            )
            .expect("request"),
        )
        .expect("request beta");
    assert_ne!(alpha.handle, beta.handle);
    assert_eq!(alpha.status.phase, TestInstancePhase::Prepared);
    assert_eq!(
        facade.build(&alpha.handle).expect("build").outcome,
        TestActionOutcome::Passed
    );
    assert_eq!(
        facade.test(&beta.handle).expect("test").outcome,
        TestActionOutcome::Passed
    );
    assert_eq!(
        facade.start(&alpha.handle).expect("start alpha").phase,
        TestInstancePhase::Running
    );
    assert_eq!(
        facade.start(&beta.handle).expect("start beta").phase,
        TestInstancePhase::Running
    );
    assert_eq!(
        facade.stop(&alpha.handle).expect("stop alpha").phase,
        TestInstancePhase::Stopped
    );
    assert_eq!(
        facade.status(&beta.handle).expect("beta status").phase,
        TestInstancePhase::Running
    );

    let prepared = runtime.prepared.lock().expect("prepared");
    assert_eq!(prepared.len(), 2);
    let first = &prepared[0];
    let second = &prepared[1];
    assert_ne!(
        first.projection.paths.instance_root,
        second.projection.paths.instance_root
    );
    assert_ne!(
        first.projection.paths.frontend_dist,
        second.projection.paths.frontend_dist
    );
    assert_ne!(
        first.projection.paths.cargo_target,
        second.projection.paths.cargo_target
    );
    assert_ne!(first.projection.ports, second.projection.ports);
    assert_eq!(
        first.projection.caches.node_path,
        second.projection.caches.node_path
    );
    assert_eq!(first.projection.caches.node_reuse, CacheReuse::SharedKeyed);
    drop(prepared);

    let launches = runtime.launches.lock().expect("launches");
    assert_eq!(launches.len(), 2);
    assert!(launches.iter().all(|launches| launches.len() == 3));
    for group in launches.iter() {
        let environment = &group[0].environment;
        assert!(!environment.keys().any(|key| {
            key.contains("OPENAI") || key.contains("TOKEN") || key.contains("API_KEY")
        }));
        assert!(Path::new(&environment["CODEX_HOME"]).starts_with(directory.path()));
    }
    assert_eq!(executor.plans.lock().expect("plans").len(), 2);
}

#[test]
fn facade_falls_back_to_isolated_caches_when_shared_root_is_unusable() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let worktree = fixture_worktree(directory.path(), "fallback");
    let blocked_cache = directory.path().join("blocked-cache");
    fs::write(&blocked_cache, "not a directory").expect("blocked cache");
    let runtime = Arc::new(FakeRuntime::default());
    let facade = facade(
        directory.path(),
        runtime.clone(),
        Arc::new(MapSources(HashMap::from([(
            "repository/fallback".into(),
            worktree,
        )]))),
        Arc::new(FixedInspector),
        Arc::new(RecordingExecutor::default()),
        blocked_cache,
    );

    facade
        .request(
            IsolatedTestRequest::new(
                TestSourceRef::new("repository/fallback").expect("source"),
                "cache fallback",
            )
            .expect("request"),
        )
        .expect("request instance");

    let prepared = runtime.prepared.lock().expect("prepared");
    let projection = &prepared[0].projection;
    assert_eq!(projection.caches.node_reuse, CacheReuse::IsolatedFallback);
    assert!(projection
        .caches
        .node_path
        .starts_with(&projection.paths.instance_root));
    assert!(projection
        .caches
        .rust_path
        .starts_with(&projection.paths.instance_root));
}

#[test]
fn facade_refuses_build_test_and_start_after_source_identity_changes() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let worktree = fixture_worktree(directory.path(), "changing");
    let runtime = Arc::new(FakeRuntime::default());
    let executor = Arc::new(RecordingExecutor::default());
    let inspector = Arc::new(ChangingInspector(AtomicBool::new(false)));
    let facade = facade(
        directory.path(),
        runtime.clone(),
        Arc::new(MapSources(HashMap::from([(
            "repository/changing".into(),
            worktree,
        )]))),
        inspector.clone(),
        executor.clone(),
        directory.path().join("shared-cache"),
    );
    let instance = facade
        .request(
            IsolatedTestRequest::new(
                TestSourceRef::new("repository/changing").expect("source"),
                "source invalidation",
            )
            .expect("request"),
        )
        .expect("request instance");

    inspector.0.store(true, Ordering::SeqCst);
    for error in [
        facade
            .build(&instance.handle)
            .expect_err("changed source must not build"),
        facade
            .test(&instance.handle)
            .expect_err("changed source must not test"),
        facade
            .start(&instance.handle)
            .expect_err("changed source must not start"),
    ] {
        assert_eq!(error.kind, TestInstanceErrorKind::Conflict);
        assert!(error
            .message
            .contains("request a new isolated test instance"));
    }
    assert!(executor.plans.lock().expect("plans").is_empty());
    assert!(runtime.launches.lock().expect("launches").is_empty());
}

#[test]
fn system_source_inspector_invalidates_nested_untracked_content() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let root = directory.path();
    fs::create_dir_all(root.join("src-tauri")).expect("Rust directory");
    fs::write(root.join("package-lock.json"), "{\"lockfileVersion\":3}\n").expect("package lock");
    fs::write(root.join("src-tauri/Cargo.lock"), "# lock\n").expect("Cargo lock");
    fs::write(
        root.join("src-tauri/Cargo.toml"),
        "[package]\nname=\"proof\"\nversion=\"0.1.0\"\n",
    )
    .expect("Cargo manifest");
    run_git(root, ["init"]);
    run_git(root, ["add", "."]);
    run_git(
        root,
        [
            "-c",
            "user.name=Codex Proof",
            "-c",
            "user.email=proof@example.invalid",
            "commit",
            "-m",
            "fixture",
        ],
    );
    let nested = root.join("untracked/nested/source.txt");
    fs::create_dir_all(nested.parent().expect("parent")).expect("nested directory");
    fs::write(&nested, "alpha").expect("untracked source");

    let programs = ToolchainPrograms::discover().expect("toolchain programs");
    let inspector = SystemSourceInspector;
    let before = inspector
        .inspect(root, &programs)
        .expect("first fingerprint");
    fs::write(&nested, "beta").expect("change untracked source");
    let after = inspector
        .inspect(root, &programs)
        .expect("second fingerprint");

    assert_ne!(before.source_fingerprint, after.source_fingerprint);
    assert_eq!(before.node_cache_key, after.node_cache_key);
    assert_eq!(before.rust_cache_key, after.rust_cache_key);
}

#[test]
fn ordinary_development_and_packaging_defaults_remain_the_product_defaults() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repository root");
    let package: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("package.json")).expect("package"))
            .expect("package JSON");
    let tauri: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("src-tauri/tauri.conf.json")).expect("Tauri config"),
    )
    .expect("Tauri JSON");

    assert_eq!(package["scripts"]["dev"], "vite");
    assert_eq!(package["scripts"]["build:tauri"], "tauri build");
    assert_eq!(tauri["identifier"], "dev.codex-orchestrator.app");
    assert_eq!(tauri["build"]["devUrl"], "http://localhost:1420");
    assert_eq!(tauri["build"]["frontendDist"], "../dist");
    assert_eq!(tauri["bundle"]["active"], true);
}

#[test]
fn system_action_executor_records_success_and_failure_semantically() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let executable = std::env::current_exe().expect("test executable");
    let command = |failure: bool| ProcessCommand {
        label: if failure {
            "failing proof"
        } else {
            "passing proof"
        },
        program: executable.clone(),
        arguments: vec![
            "--ignored".into(),
            "--exact".into(),
            "worktree_runtime::facade_tests::action_executor_child".into(),
            "--nocapture".into(),
        ],
        working_directory: directory.path().to_path_buf(),
        environment: child_environment(failure),
    };
    let executor = SystemActionExecutor;
    let passing_log = directory.path().join("passing.log");
    let passing = executor
        .execute(&ActionPlan {
            kind: ActionKind::Test,
            log_path: passing_log.clone(),
            commands: vec![command(false)],
        })
        .expect("passing execution");
    assert!(passing.succeeded);
    assert!(fs::read_to_string(passing_log)
        .expect("passing log")
        .contains("action-executor-proof"));

    let failing = executor
        .execute(&ActionPlan {
            kind: ActionKind::Test,
            log_path: directory.path().join("failing.log"),
            commands: vec![command(true)],
        })
        .expect("failing execution is a semantic result");
    assert!(!failing.succeeded);
    assert_eq!(failing.failed_step.as_deref(), Some("failing proof"));
}

#[test]
#[ignore = "helper process launched only by the action-executor integration proof"]
fn action_executor_child() {
    println!("action-executor-proof");
    assert_ne!(
        std::env::var("ACTION_EXECUTOR_FAIL").as_deref(),
        Ok("true"),
        "requested proof failure"
    );
}

fn child_environment(failure: bool) -> BTreeMap<String, String> {
    let mut environment = ["SystemRoot", "SystemDrive", "WINDIR", "TEMP", "TMP"]
        .into_iter()
        .filter_map(|name| std::env::var(name).ok().map(|value| (name.into(), value)))
        .collect::<BTreeMap<_, _>>();
    if failure {
        environment.insert("ACTION_EXECUTOR_FAIL".into(), "true".into());
    }
    environment
}

fn fixture_worktree(root: &Path, name: &str) -> PathBuf {
    let worktree = root.join(name);
    for relative in [
        "node_modules/typescript/bin/tsc",
        "node_modules/vite/bin/vite.js",
        "node_modules/@tauri-apps/cli/tauri.js",
        "node_modules/vitest/vitest.mjs",
        "scripts/runtime-status-server.mjs",
        "src-tauri/Cargo.toml",
    ] {
        let path = worktree.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("fixture directory");
        fs::write(path, "fixture").expect("fixture file");
    }
    worktree.canonicalize().expect("fixture worktree")
}

fn facade(
    root: &Path,
    runtime: Arc<FakeRuntime>,
    sources: Arc<dyn TestSourceResolver>,
    inspector: Arc<dyn SourceInspector>,
    executor: Arc<dyn ActionExecutor>,
    shared_cache_root: PathBuf,
) -> WorktreeTestInstanceFacade {
    let executable = std::env::current_exe().expect("test executable");
    WorktreeTestInstanceFacade::new(
        runtime,
        sources,
        inspector,
        executor,
        RuntimeSettings {
            instances_root: root.join("instances"),
            shared_cache_root,
            port_start: 33000,
            port_end: 33031,
        },
        ToolchainPrograms {
            git: executable.clone(),
            node: executable.clone(),
            cargo: executable.clone(),
            rustc: executable,
        },
        super::domain::AuthoritySecret::new("facade-test-authority-secret").expect("authority"),
    )
    .expect("facade")
}

struct MapSources(HashMap<String, PathBuf>);

impl TestSourceResolver for MapSources {
    fn resolve(&self, source: &TestSourceRef) -> Result<PathBuf, TestInstanceError> {
        self.0.get(source.as_str()).cloned().ok_or_else(|| {
            TestInstanceError::new(TestInstanceErrorKind::NotFound, "source not found")
        })
    }
}

struct FixedInspector;

impl SourceInspector for FixedInspector {
    fn inspect(
        &self,
        worktree: &Path,
        _programs: &ToolchainPrograms,
    ) -> Result<SourceSnapshot, PlanningError> {
        let marker = if worktree.ends_with("alpha") {
            'a'
        } else if worktree.ends_with("beta") {
            'b'
        } else {
            'c'
        };
        Ok(SourceSnapshot {
            git_commit: "0123456789abcdef0123456789abcdef01234567".into(),
            source_fingerprint: marker.to_string().repeat(64),
            node_cache_key: "shared-node-key".into(),
            rust_cache_key: "shared-rust-key".into(),
        })
    }
}

struct ChangingInspector(AtomicBool);

impl SourceInspector for ChangingInspector {
    fn inspect(
        &self,
        _worktree: &Path,
        _programs: &ToolchainPrograms,
    ) -> Result<SourceSnapshot, PlanningError> {
        let marker = if self.0.load(Ordering::SeqCst) {
            'd'
        } else {
            'c'
        };
        Ok(SourceSnapshot {
            git_commit: "0123456789abcdef0123456789abcdef01234567".into(),
            source_fingerprint: marker.to_string().repeat(64),
            node_cache_key: "shared-node-key".into(),
            rust_cache_key: "shared-rust-key".into(),
        })
    }
}

#[derive(Default)]
struct RecordingExecutor {
    plans: Mutex<Vec<ActionPlan>>,
}

impl ActionExecutor for RecordingExecutor {
    fn execute(&self, plan: &ActionPlan) -> Result<ActionExecution, ExecutionError> {
        self.plans.lock().expect("plans").push(plan.clone());
        Ok(ActionExecution {
            succeeded: true,
            failed_step: None,
        })
    }
}

#[derive(Default)]
struct FakeRuntime {
    snapshots: Mutex<HashMap<super::domain::InstanceId, InstanceSnapshot>>,
    ports: Mutex<HashSet<u16>>,
    prepared: Mutex<Vec<InstanceRecord>>,
    launches: Mutex<Vec<Vec<super::domain::OwnedProcessLaunch>>>,
}

impl WorktreeRuntimeControl for FakeRuntime {
    fn prepare(
        &self,
        command: PrepareInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        let mut ports = self.ports.lock().expect("ports");
        if [
            command.projection.ports.vite,
            command.projection.ports.status,
        ]
        .iter()
        .any(|port| ports.contains(port))
        {
            return Err(runtime_failure(
                RuntimeApplicationErrorKind::PortLeaseConflict,
                "port already leased",
            ));
        }
        ports.extend([
            command.projection.ports.vite,
            command.projection.ports.status,
        ]);
        let record = InstanceRecord {
            identity: command.identity,
            projection: command.projection,
            state: InstanceState::Prepared,
            owner_route: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let snapshot = InstanceSnapshot::from_record(record.clone());
        self.prepared.lock().expect("prepared").push(record);
        self.snapshots.lock().expect("snapshots").insert(
            snapshot.projected.identity.instance_id.clone(),
            snapshot.clone(),
        );
        Ok(snapshot)
    }

    fn start(
        &self,
        command: StartInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        self.launches
            .lock()
            .expect("launches")
            .push(command.launches);
        self.transition(&command.instance_id, InstanceState::Running, true)
    }

    fn read(&self, query: ReadInstanceQuery) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        self.snapshots
            .lock()
            .expect("snapshots")
            .get(&query.instance_id)
            .cloned()
            .ok_or_else(|| runtime_failure(RuntimeApplicationErrorKind::NotFound, "not found"))
    }

    fn stop(
        &self,
        command: StopInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        self.transition(&command.instance_id, InstanceState::Stopped, false)
    }

    fn recover(
        &self,
        command: RecoverInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        self.transition(&command.instance_id, InstanceState::Recovered, false)
    }
}

impl FakeRuntime {
    fn transition(
        &self,
        instance_id: &super::domain::InstanceId,
        state: InstanceState,
        reachable: bool,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        let mut snapshots = self.snapshots.lock().expect("snapshots");
        let current = snapshots
            .get(instance_id)
            .cloned()
            .ok_or_else(|| runtime_failure(RuntimeApplicationErrorKind::NotFound, "not found"))?;
        let mut record = current.projected;
        record.state = state;
        record.updated_at = Utc::now();
        let snapshot = InstanceSnapshot::with_observation(
            record.clone(),
            RuntimeObservation {
                owner: if reachable {
                    OwnerObservation::Owned {
                        active_processes: 3,
                    }
                } else {
                    OwnerObservation::Absent
                },
                health: HealthObservation {
                    vite: EndpointObservation {
                        port: record.projection.ports.vite,
                        reachable,
                    },
                    status: EndpointObservation {
                        port: record.projection.ports.status,
                        reachable,
                    },
                },
                observed_at: Utc::now(),
            },
        );
        snapshots.insert(instance_id.clone(), snapshot.clone());
        Ok(snapshot)
    }
}

fn runtime_failure(kind: RuntimeApplicationErrorKind, message: &str) -> RuntimeApplicationError {
    RuntimeApplicationError {
        kind,
        message: message.into(),
    }
}

fn run_git<const N: usize>(cwd: &Path, arguments: [&str; N]) {
    let status = Command::new("git")
        .args(arguments)
        .current_dir(cwd)
        .status()
        .expect("run Git");
    assert!(status.success(), "Git fixture command failed");
}
