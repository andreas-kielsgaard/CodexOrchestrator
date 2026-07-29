#[cfg(windows)]
use super::registry::RegistryErrorKind;
use super::{
    application::{
        ObservationPolicy, PrepareInstanceCommand, ReadInstanceQuery, RecoverInstanceCommand,
        RuntimeApplicationError, RuntimeApplicationErrorKind, RuntimeClock, StartInstanceCommand,
        StopInstanceCommand, WorktreeRuntimeApplication, WorktreeRuntimeControl,
    },
    domain::{
        AuthoritySecret, CacheReuse, EndpointObservation, HealthObservation, InstanceProjection,
        InstanceRecord, InstanceSnapshot, InstanceState, OwnedProcessLaunch, OwnerObservation,
        OwnerRoute, RuntimeObservation,
    },
    execution::{
        ActionExecution, ActionExecutor, ActionProgressObserver, ExecutionError,
        NoopActionProgressObserver, SystemActionExecutor,
    },
    facade::{
        IsolatedTestRequest, TestActionOutcome, TestInstanceError, TestInstanceErrorKind,
        TestInstancePhase, TestSourceRef, TestSourceResolver, WorktreeTestInstanceFacade,
        WorktreeTestInstances,
    },
    health::HealthProbe,
    ownership::{
        OwnershipError, OwnershipErrorKind, ProcessOwner, ReviewWindowExpectation,
        ReviewWindowObservation,
    },
    planning::{
        ActionKind, ActionPlan, PlanningError, ProcessCommand, RuntimeSettings, SourceInspector,
        SourceSnapshot, SystemSourceInspector, ToolchainPrograms,
    },
    registry::SqliteInstanceRegistry,
};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    panic::{catch_unwind, AssertUnwindSafe},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Condvar, Mutex},
    thread,
    time::Duration,
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
        facade.build(&beta.handle).expect("build beta").outcome,
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
    let plans = executor.plans.lock().expect("plans");
    assert_eq!(plans.len(), 3);
    for plan in plans.iter().filter(|plan| plan.kind == ActionKind::Build) {
        let tauri = plan.commands.last().expect("Tauri build command");
        let config_index = tauri
            .arguments
            .iter()
            .position(|argument| argument == "--config")
            .expect("Tauri config argument");
        let config: serde_json::Value =
            serde_json::from_str(&tauri.arguments[config_index + 1]).expect("Tauri config JSON");
        let frontend_dist = config["build"]["frontendDist"]
            .as_str()
            .expect("frontend dist");
        assert!(Path::new(frontend_dist).is_relative());
        assert_ne!(
            Path::new(frontend_dist),
            Path::new(&tauri.environment["VITE_RUNTIME_DIST"])
        );
    }
}

#[test]
fn real_registry_facade_concurrent_stop_reports_in_progress_without_orphan() {
    assert_concurrent_terminal_call(ConcurrentTerminalCall::Stop);
}

#[test]
fn real_registry_facade_concurrent_stop_recover_conflicts_without_orphan() {
    assert_concurrent_terminal_call(ConcurrentTerminalCall::Recover);
}

#[test]
fn real_registry_facade_stop_cannot_supersede_executing_start() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let source_path = fixture_worktree(directory.path(), "start-stop");
    let registry_path = directory.path().join("registry.sqlite");
    let world = Arc::new(BlockingLaunchWorld::default());
    let facade = Arc::new(real_registry_facade(
        directory.path(),
        &registry_path,
        source_path,
        Arc::new(BlockingLaunchProcessOwner(world.clone())),
        Arc::new(BlockingLaunchHealthProbe(world.clone())),
    ));
    let requested = request_lifecycle_instance(&facade, "start stop serialization");

    let start_facade = facade.clone();
    let start_handle = requested.handle.clone();
    let start = thread::spawn(move || start_facade.start(&start_handle));
    world.wait_for_launch();
    let stop_result = facade.stop(&requested.handle);
    world.release_launch();
    let start_result = start.join().expect("start thread");

    assert_eq!(
        stop_result
            .expect_err("stop must not supersede active start")
            .kind,
        TestInstanceErrorKind::OperationInProgress
    );
    assert_eq!(
        start_result.expect("start completes").phase,
        TestInstancePhase::Running
    );
    let raw = Connection::open(&registry_path).expect("inspect real registry");
    assert_eq!(command_count(&raw, "status='pending'"), 0);
    assert_eq!(
        command_count(&raw, "operation='start' AND status='succeeded'"),
        1
    );
    assert_eq!(command_count(&raw, "operation='stop'"), 0);
}

#[test]
fn real_registry_facade_recovery_fails_abandoned_start_before_reserving_recovery() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let source_path = fixture_worktree(directory.path(), "interrupted-start");
    let registry_path = directory.path().join("registry.sqlite");
    let facade = real_registry_facade(
        directory.path(),
        &registry_path,
        source_path.clone(),
        Arc::new(PanickingLaunchProcessOwner),
        Arc::new(ClosedHealthProbe),
    );
    let requested = request_lifecycle_instance(&facade, "interrupted start recovery");
    #[cfg(windows)]
    assert_eq!(
        SqliteInstanceRegistry::open(&registry_path)
            .err()
            .expect("second live registry must fail closed")
            .kind,
        RegistryErrorKind::Conflict
    );
    let interrupted = catch_unwind(AssertUnwindSafe(|| facade.start(&requested.handle)));
    assert!(
        interrupted.is_err(),
        "test launch must simulate interruption"
    );
    drop(facade);

    let recovery_facade = real_registry_facade(
        directory.path(),
        &registry_path,
        source_path,
        Arc::new(AbsentProcessOwner),
        Arc::new(ClosedHealthProbe),
    );
    let resumed = request_lifecycle_instance(&recovery_facade, "interrupted start recovery");
    assert_eq!(resumed.status.phase, TestInstancePhase::Starting);
    assert!(resumed.status.stale);
    assert_eq!(
        recovery_facade
            .recover(&resumed.handle)
            .expect("recover abandoned start")
            .phase,
        TestInstancePhase::Recovered
    );

    let raw = Connection::open(&registry_path).expect("inspect real registry");
    assert_eq!(command_count(&raw, "status='pending'"), 0);
    assert_eq!(
        command_count(&raw, "operation='start' AND status='failed'"),
        1
    );
    assert_eq!(
        command_count(&raw, "operation='recover' AND status='succeeded'"),
        1
    );
    let start_failure: String = raw
        .query_row(
            "SELECT failure_json FROM worktree_runtime_commands WHERE operation='start'",
            [],
            |row| row.get(0),
        )
        .expect("abandoned start failure");
    assert!(start_failure.contains("abandoned"));
    assert!(start_failure.contains("superseded by recovery"));
}

#[test]
fn public_facade_requires_exact_owned_usable_window_and_rendered_marker() {
    let rejected = [
        ReadinessCase::NoWindow,
        ReadinessCase::Titleless,
        ReadinessCase::Tiny,
        ReadinessCase::Hidden,
        ReadinessCase::Minimized,
        ReadinessCase::Cloaked,
        ReadinessCase::WrongTitle,
        ReadinessCase::UnownedSimilar,
        ReadinessCase::MissingMarker,
    ];
    for case in rejected {
        let directory = tempfile::tempdir().expect("temporary directory");
        let registry_path = directory.path().join("registry.sqlite");
        let active = Arc::new(AtomicBool::new(false));
        let facade = readiness_facade(directory.path(), &registry_path, case, active.clone());
        let requested = request_readiness_instance(&facade);
        let error = facade
            .start(&requested.handle)
            .expect_err("false-ready evidence must be rejected");
        assert_eq!(error.kind, TestInstanceErrorKind::Unavailable, "{case:?}");
        assert!(!active.load(Ordering::SeqCst), "{case:?} was not cleaned");
        assert_eq!(
            facade.status(&requested.handle).expect("status").phase,
            TestInstancePhase::Stopped,
            "{case:?}"
        );
        assert_eq!(
            facade
                .recover(&requested.handle)
                .expect("recover failed start")
                .phase,
            TestInstancePhase::Stopped,
            "{case:?}"
        );
        let raw = Connection::open(&registry_path).expect("registry");
        assert_eq!(command_count(&raw, "status='pending'"), 0, "{case:?}");
        assert_eq!(
            command_count(&raw, "operation='start' AND status='failed'"),
            1,
            "{case:?}"
        );
        assert_eq!(
            command_count(&raw, "operation='recover' AND status='succeeded'"),
            1,
            "{case:?}"
        );
    }

    let directory = tempfile::tempdir().expect("temporary directory");
    let registry_path = directory.path().join("registry.sqlite");
    let active = Arc::new(AtomicBool::new(false));
    let facade = readiness_facade(
        directory.path(),
        &registry_path,
        ReadinessCase::Usable,
        active.clone(),
    );
    let requested = request_readiness_instance(&facade);
    assert_eq!(
        facade.start(&requested.handle).expect("usable start").phase,
        TestInstancePhase::Running
    );
    assert!(active.load(Ordering::SeqCst));
    assert_eq!(
        facade.stop(&requested.handle).expect("stop usable").phase,
        TestInstancePhase::Stopped
    );
    assert!(!active.load(Ordering::SeqCst));
}

#[derive(Clone, Copy, Debug)]
enum ReadinessCase {
    NoWindow,
    Titleless,
    Tiny,
    Hidden,
    Minimized,
    Cloaked,
    WrongTitle,
    UnownedSimilar,
    MissingMarker,
    Usable,
}

struct ReadinessOwner {
    active: Arc<AtomicBool>,
    case: ReadinessCase,
}

impl ProcessOwner for ReadinessOwner {
    fn launch(
        &self,
        _route: &OwnerRoute,
        launches: &[OwnedProcessLaunch],
    ) -> Result<OwnerObservation, OwnershipError> {
        self.active.store(true, Ordering::SeqCst);
        if !matches!(self.case, ReadinessCase::MissingMarker) {
            write_ready_marker(launches);
        }
        Ok(OwnerObservation::Owned {
            active_processes: launches.len() as u32,
        })
    }

    fn observe(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Ok(if self.active.load(Ordering::SeqCst) {
            OwnerObservation::Owned {
                active_processes: 3,
            }
        } else {
            OwnerObservation::Absent
        })
    }

    fn observe_review_window(
        &self,
        _route: &OwnerRoute,
        expected: &ReviewWindowExpectation,
    ) -> Result<Option<ReviewWindowObservation>, OwnershipError> {
        if !self.active.load(Ordering::SeqCst)
            || matches!(
                self.case,
                ReadinessCase::NoWindow | ReadinessCase::UnownedSimilar
            )
        {
            return Ok(None);
        }
        let mut window = usable_window(expected);
        match self.case {
            ReadinessCase::Titleless => window.title.clear(),
            ReadinessCase::Tiny => {
                window.width = 18;
                window.height = 18;
                window.client_width = 1;
                window.client_height = 1;
            }
            ReadinessCase::Hidden => window.visible = false,
            ReadinessCase::Minimized => window.minimized = true,
            ReadinessCase::Cloaked => window.cloaked = true,
            ReadinessCase::WrongTitle => window.title.push_str(" copy"),
            _ => {}
        }
        Ok(Some(window))
    }

    fn terminate(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        self.active.store(false, Ordering::SeqCst);
        Ok(OwnerObservation::Absent)
    }
}

struct ReadinessHealth(Arc<AtomicBool>);

impl HealthProbe for ReadinessHealth {
    fn observe(&self, projection: &InstanceProjection) -> HealthObservation {
        health_observation(projection, self.0.load(Ordering::SeqCst))
    }
}

struct ReadinessClock;

impl RuntimeClock for ReadinessClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

fn readiness_facade(
    root: &Path,
    registry_path: &Path,
    case: ReadinessCase,
    active: Arc<AtomicBool>,
) -> WorktreeTestInstanceFacade {
    let source = fixture_worktree(root, "readiness");
    let application = Arc::new(WorktreeRuntimeApplication::new(
        Arc::new(SqliteInstanceRegistry::open(registry_path).expect("registry")),
        Arc::new(ReadinessOwner {
            active: active.clone(),
            case,
        }),
        Arc::new(ReadinessHealth(active)),
        Arc::new(ReadinessClock),
        ObservationPolicy {
            attempts: 1,
            interval: Duration::ZERO,
        },
    ));
    let executable = std::env::current_exe().expect("test executable");
    WorktreeTestInstanceFacade::new(
        application,
        Arc::new(MapSources(HashMap::from([(
            "repository/readiness".into(),
            source,
        )]))),
        Arc::new(FixedInspector),
        Arc::new(RecordingExecutor::default()),
        RuntimeSettings {
            instances_root: root.join("instances"),
            shared_cache_root: root.join("shared-cache"),
            port_start: 33300,
            port_end: 33331,
        },
        ToolchainPrograms {
            git: executable.clone(),
            node: executable.clone(),
            cargo: executable.clone(),
            rustc: executable,
        },
        AuthoritySecret::new("readiness-facade-authority-secret").expect("authority"),
    )
    .expect("facade")
}

fn request_readiness_instance(
    facade: &WorktreeTestInstanceFacade,
) -> super::facade::RequestedTestInstance {
    let requested = facade
        .request(
            IsolatedTestRequest::new(
                TestSourceRef::new("repository/readiness").expect("source"),
                "readiness proof",
            )
            .expect("request"),
        )
        .expect("prepare");
    facade.build(&requested.handle).expect("build");
    requested
}

#[derive(Clone, Copy)]
enum ConcurrentTerminalCall {
    Stop,
    Recover,
}

fn real_registry_facade(
    root: &Path,
    registry_path: &Path,
    source_path: PathBuf,
    owner: Arc<dyn ProcessOwner>,
    health: Arc<dyn HealthProbe>,
) -> WorktreeTestInstanceFacade {
    let executable = std::env::current_exe().expect("test executable");
    WorktreeTestInstanceFacade::new(
        Arc::new(WorktreeRuntimeApplication::system(
            Arc::new(SqliteInstanceRegistry::open(registry_path).expect("real registry")),
            owner,
            health,
        )),
        Arc::new(MapSources(HashMap::from([(
            "repository/lifecycle".into(),
            source_path,
        )]))),
        Arc::new(FixedInspector),
        Arc::new(RecordingExecutor::default()),
        RuntimeSettings {
            instances_root: root.join("instances"),
            shared_cache_root: root.join("shared-cache"),
            port_start: 33200,
            port_end: 33231,
        },
        ToolchainPrograms {
            git: executable.clone(),
            node: executable.clone(),
            cargo: executable.clone(),
            rustc: executable,
        },
        AuthoritySecret::new("lifecycle-facade-authority-secret").expect("authority"),
    )
    .expect("facade")
}

fn request_lifecycle_instance(
    facade: &WorktreeTestInstanceFacade,
    purpose: &str,
) -> super::facade::RequestedTestInstance {
    let requested = facade
        .request(
            IsolatedTestRequest::new(
                TestSourceRef::new("repository/lifecycle").expect("source"),
                purpose,
            )
            .expect("request"),
        )
        .expect("prepare instance");
    if requested.status.phase == TestInstancePhase::Prepared {
        assert_eq!(
            facade
                .build(&requested.handle)
                .expect("build lifecycle instance")
                .outcome,
            TestActionOutcome::Passed
        );
    }
    requested
}

fn command_count(connection: &Connection, predicate: &str) -> i64 {
    connection
        .query_row(
            &format!("SELECT COUNT(*) FROM worktree_runtime_commands WHERE {predicate}"),
            [],
            |row| row.get(0),
        )
        .expect("runtime command count")
}

fn assert_concurrent_terminal_call(second_call: ConcurrentTerminalCall) {
    let directory = tempfile::tempdir().expect("temporary directory");
    let source_path = fixture_worktree(directory.path(), "concurrent");
    let registry_path = directory.path().join("registry.sqlite");
    let world = Arc::new(BlockingRuntimeWorld::default());
    let runtime = Arc::new(WorktreeRuntimeApplication::system(
        Arc::new(SqliteInstanceRegistry::open(&registry_path).expect("real registry")),
        Arc::new(BlockingProcessOwner(world.clone())),
        Arc::new(BlockingHealthProbe(world.clone())),
    ));
    let executable = std::env::current_exe().expect("test executable");
    let facade = Arc::new(
        WorktreeTestInstanceFacade::new(
            runtime,
            Arc::new(MapSources(HashMap::from([(
                "repository/concurrent".into(),
                source_path,
            )]))),
            Arc::new(FixedInspector),
            Arc::new(RecordingExecutor::default()),
            RuntimeSettings {
                instances_root: directory.path().join("instances"),
                shared_cache_root: directory.path().join("shared-cache"),
                port_start: 33100,
                port_end: 33131,
            },
            ToolchainPrograms {
                git: executable.clone(),
                node: executable.clone(),
                cargo: executable.clone(),
                rustc: executable,
            },
            AuthoritySecret::new("concurrent-facade-authority-secret").expect("authority"),
        )
        .expect("facade"),
    );
    let requested = facade
        .request(
            IsolatedTestRequest::new(
                TestSourceRef::new("repository/concurrent").expect("source"),
                "concurrent terminal proof",
            )
            .expect("request"),
        )
        .expect("prepare instance");
    facade
        .build(&requested.handle)
        .expect("build concurrent instance");
    assert_eq!(
        facade.start(&requested.handle).expect("start").phase,
        TestInstancePhase::Running
    );

    let first_facade = facade.clone();
    let first_handle = requested.handle.clone();
    let first_stop = thread::spawn(move || first_facade.stop(&first_handle));
    world.wait_for_termination();

    let second_result = match second_call {
        ConcurrentTerminalCall::Stop => facade.stop(&requested.handle),
        ConcurrentTerminalCall::Recover => facade.recover(&requested.handle),
    };
    world.release_termination();
    let first_result = first_stop.join().expect("first stop thread");

    assert_eq!(
        first_result.expect("first stop completes").phase,
        TestInstancePhase::Stopped
    );
    let second_error = second_result.expect_err("second terminal call must not execute");
    match second_call {
        ConcurrentTerminalCall::Stop => {
            assert_eq!(
                second_error.kind,
                TestInstanceErrorKind::OperationInProgress
            );
            assert!(second_error
                .message
                .contains("stop transition is already in progress"));
        }
        ConcurrentTerminalCall::Recover => {
            assert_eq!(second_error.kind, TestInstanceErrorKind::Conflict);
            assert!(second_error
                .message
                .contains("cannot begin recover while the stop transition is in progress"));
        }
    }

    let raw = Connection::open(&registry_path).expect("inspect real registry");
    let pending_commands: i64 = raw
        .query_row(
            "SELECT COUNT(*) FROM worktree_runtime_commands WHERE status='pending'",
            [],
            |row| row.get(0),
        )
        .expect("pending command count");
    let stop_commands: i64 = raw
        .query_row(
            "SELECT COUNT(*) FROM worktree_runtime_commands WHERE operation='stop'",
            [],
            |row| row.get(0),
        )
        .expect("stop command count");
    let recover_commands: i64 = raw
        .query_row(
            "SELECT COUNT(*) FROM worktree_runtime_commands WHERE operation='recover'",
            [],
            |row| row.get(0),
        )
        .expect("recover command count");
    assert_eq!(pending_commands, 0);
    assert_eq!(stop_commands, 1);
    assert_eq!(recover_commands, 0);
}

#[derive(Default)]
struct BlockingRuntimeWorld {
    state: Mutex<BlockingRuntimeState>,
    changed: Condvar,
}

#[derive(Default)]
struct BlockingLaunchWorld {
    state: Mutex<BlockingLaunchState>,
    changed: Condvar,
}

#[derive(Default)]
struct BlockingLaunchState {
    active: bool,
    launch_entered: bool,
    release_launch: bool,
}

impl BlockingLaunchWorld {
    fn wait_for_launch(&self) {
        let state = self.state.lock().expect("launch state");
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, Duration::from_secs(5), |state| !state.launch_entered)
            .expect("wait for launch");
        assert!(!timeout.timed_out(), "start did not enter process launch");
        assert!(state.launch_entered);
    }

    fn release_launch(&self) {
        let mut state = self.state.lock().expect("launch state");
        state.release_launch = true;
        self.changed.notify_all();
    }

    fn active(&self) -> bool {
        self.state.lock().expect("launch state").active
    }
}

struct BlockingLaunchProcessOwner(Arc<BlockingLaunchWorld>);

impl ProcessOwner for BlockingLaunchProcessOwner {
    fn launch(
        &self,
        _route: &OwnerRoute,
        launches: &[OwnedProcessLaunch],
    ) -> Result<OwnerObservation, OwnershipError> {
        let mut state = self.0.state.lock().expect("launch state");
        state.launch_entered = true;
        self.0.changed.notify_all();
        while !state.release_launch {
            state = self.0.changed.wait(state).expect("release launch");
        }
        write_ready_marker(launches);
        state.active = true;
        Ok(OwnerObservation::Owned {
            active_processes: launches.len() as u32,
        })
    }

    fn observe(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Ok(if self.0.active() {
            OwnerObservation::Owned {
                active_processes: 3,
            }
        } else {
            OwnerObservation::Absent
        })
    }

    fn observe_review_window(
        &self,
        route: &OwnerRoute,
        expected: &ReviewWindowExpectation,
    ) -> Result<Option<ReviewWindowObservation>, OwnershipError> {
        Ok(self
            .observe(route)?
            .is_active()
            .then(|| usable_window(expected)))
    }

    fn terminate(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        self.0.state.lock().expect("launch state").active = false;
        Ok(OwnerObservation::Absent)
    }
}

struct BlockingLaunchHealthProbe(Arc<BlockingLaunchWorld>);

impl HealthProbe for BlockingLaunchHealthProbe {
    fn observe(&self, projection: &InstanceProjection) -> HealthObservation {
        health_observation(projection, self.0.active())
    }
}

struct PanickingLaunchProcessOwner;

impl ProcessOwner for PanickingLaunchProcessOwner {
    fn launch(
        &self,
        _route: &OwnerRoute,
        _launches: &[OwnedProcessLaunch],
    ) -> Result<OwnerObservation, OwnershipError> {
        panic!("simulate process interruption after durable start reservation")
    }

    fn observe(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Ok(OwnerObservation::Absent)
    }

    fn terminate(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Ok(OwnerObservation::Absent)
    }
}

struct AbsentProcessOwner;

impl ProcessOwner for AbsentProcessOwner {
    fn launch(
        &self,
        _route: &OwnerRoute,
        _launches: &[OwnedProcessLaunch],
    ) -> Result<OwnerObservation, OwnershipError> {
        Err(OwnershipError {
            kind: OwnershipErrorKind::LaunchFailed,
            message: "absent test owner cannot launch".into(),
        })
    }

    fn observe(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Ok(OwnerObservation::Absent)
    }

    fn terminate(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Ok(OwnerObservation::Absent)
    }
}

struct ClosedHealthProbe;

impl HealthProbe for ClosedHealthProbe {
    fn observe(&self, projection: &InstanceProjection) -> HealthObservation {
        health_observation(projection, false)
    }
}

fn health_observation(projection: &InstanceProjection, reachable: bool) -> HealthObservation {
    HealthObservation {
        vite: EndpointObservation {
            port: projection.ports.vite,
            reachable,
        },
        status: EndpointObservation {
            port: projection.ports.status,
            reachable,
        },
    }
}

fn write_ready_marker(launches: &[OwnedProcessLaunch]) {
    if let Some(path) = launches.iter().find_map(|launch| {
        launch
            .environment
            .get("CODEX_ORCHESTRATOR_WORKTREE_READY_PATH")
    }) {
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("ready marker parent");
        }
        fs::write(path, "application-surface-rendered").expect("ready marker");
    }
}

fn usable_window(expected: &ReviewWindowExpectation) -> ReviewWindowObservation {
    ReviewWindowObservation {
        title: expected.title.clone(),
        visible: true,
        minimized: false,
        cloaked: false,
        width: expected.minimum_width + 200,
        height: expected.minimum_height + 120,
        client_width: expected.minimum_width + 160,
        client_height: expected.minimum_height + 40,
    }
}

#[derive(Default)]
struct BlockingRuntimeState {
    active: bool,
    termination_entered: bool,
    release_termination: bool,
}

impl BlockingRuntimeWorld {
    fn wait_for_termination(&self) {
        let state = self.state.lock().expect("runtime state");
        let (state, timeout) = self
            .changed
            .wait_timeout_while(state, Duration::from_secs(5), |state| {
                !state.termination_entered
            })
            .expect("wait for termination");
        assert!(!timeout.timed_out(), "first stop did not begin termination");
        assert!(state.termination_entered);
    }

    fn release_termination(&self) {
        let mut state = self.state.lock().expect("runtime state");
        state.release_termination = true;
        self.changed.notify_all();
    }

    fn active(&self) -> bool {
        self.state.lock().expect("runtime state").active
    }
}

struct BlockingProcessOwner(Arc<BlockingRuntimeWorld>);

impl ProcessOwner for BlockingProcessOwner {
    fn launch(
        &self,
        _route: &OwnerRoute,
        launches: &[OwnedProcessLaunch],
    ) -> Result<OwnerObservation, OwnershipError> {
        let mut state = self.0.state.lock().expect("runtime state");
        if state.active {
            return Err(OwnershipError {
                kind: OwnershipErrorKind::AlreadyExists,
                message: "test owner is already active".into(),
            });
        }
        write_ready_marker(launches);
        state.active = true;
        Ok(OwnerObservation::Owned {
            active_processes: launches.len() as u32,
        })
    }

    fn observe(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Ok(if self.0.active() {
            OwnerObservation::Owned {
                active_processes: 3,
            }
        } else {
            OwnerObservation::Absent
        })
    }

    fn observe_review_window(
        &self,
        route: &OwnerRoute,
        expected: &ReviewWindowExpectation,
    ) -> Result<Option<ReviewWindowObservation>, OwnershipError> {
        Ok(self
            .observe(route)?
            .is_active()
            .then(|| usable_window(expected)))
    }

    fn terminate(&self, _route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        let mut state = self.0.state.lock().expect("runtime state");
        state.active = false;
        if state.termination_entered {
            return Ok(OwnerObservation::Absent);
        }
        state.termination_entered = true;
        self.0.changed.notify_all();
        while !state.release_termination {
            state = self.0.changed.wait(state).expect("release termination");
        }
        Ok(OwnerObservation::Absent)
    }
}

struct BlockingHealthProbe(Arc<BlockingRuntimeWorld>);

impl HealthProbe for BlockingHealthProbe {
    fn observe(&self, projection: &InstanceProjection) -> HealthObservation {
        let reachable = self.0.active();
        HealthObservation {
            vite: EndpointObservation {
                port: projection.ports.vite,
                reachable,
            },
            status: EndpointObservation {
                port: projection.ports.status,
                reachable,
            },
        }
    }
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
        .execute(
            &ActionPlan {
                kind: ActionKind::Test,
                log_path: passing_log.clone(),
                commands: vec![command(false)],
            },
            &NoopActionProgressObserver,
        )
        .expect("passing execution");
    assert!(passing.succeeded);
    assert!(fs::read_to_string(passing_log)
        .expect("passing log")
        .contains("action-executor-proof"));

    let failing = executor
        .execute(
            &ActionPlan {
                kind: ActionKind::Test,
                log_path: directory.path().join("failing.log"),
                commands: vec![command(true)],
            },
            &NoopActionProgressObserver,
        )
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
    fn execute(
        &self,
        plan: &ActionPlan,
        _progress: &dyn ActionProgressObserver,
    ) -> Result<ActionExecution, ExecutionError> {
        self.plans.lock().expect("plans").push(plan.clone());
        if plan.kind == ActionKind::Build {
            let target = plan
                .commands
                .last()
                .and_then(|command| command.environment.get("CARGO_TARGET_DIR"))
                .ok_or_else(|| ExecutionError {
                    message: "missing isolated Cargo target in build plan".into(),
                })?;
            let executable = PathBuf::from(target).join("debug/codex-orchestrator.exe");
            fs::create_dir_all(executable.parent().expect("target parent")).map_err(|error| {
                ExecutionError {
                    message: error.to_string(),
                }
            })?;
            fs::write(executable, "verified fixture build").map_err(|error| ExecutionError {
                message: error.to_string(),
            })?;
        }
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

    fn focus(
        &self,
        command: super::application::FocusInstanceCommand,
    ) -> Result<InstanceSnapshot, RuntimeApplicationError> {
        self.transition(&command.instance_id, InstanceState::Running, true)
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
