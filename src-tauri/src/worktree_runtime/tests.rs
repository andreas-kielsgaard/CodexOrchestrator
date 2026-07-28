use super::{
    application::{
        ObservationPolicy, PrepareInstanceCommand, ReadInstanceQuery, RecoverInstanceCommand,
        RuntimeApplicationErrorKind, RuntimeClock, StartInstanceCommand, StopInstanceCommand,
        WorktreeRuntimeApplication, WorktreeRuntimeControl,
    },
    domain::{
        AuthoritySecret, BuildId, CacheReuse, EndpointObservation, HealthObservation, InstanceId,
        InstanceIdentity, InstanceProjection, InstanceState, OwnedProcessLaunch, OwnerObservation,
        OwnerRoute, PortProjection, ProcessRole, RequestId, SessionLink,
    },
    health::HealthProbe,
    ownership::{OwnershipError, OwnershipErrorKind, ProcessOwner},
    planning::{launch_plan, project_runtime, RuntimeSettings, SourceSnapshot, ToolchainPrograms},
    projection::{project_instance, ProjectionRequest},
    registry::SqliteInstanceRegistry,
};
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::Connection;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
    sync::{
        atomic::{AtomicI64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

const AUTHORITY: &str = "test-authority-secret-0001";
const CHILD_PORT_ENV: &str = "CODEX_WORKTREE_RUNTIME_CHILD_PORT";

#[test]
fn registry_and_application_keep_two_instances_isolated_and_recover_stale_state() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let registry_path = directory.path().join("registry.sqlite");
    let world = Arc::new(FakeWorld::default());
    let application = test_application(
        &registry_path,
        Arc::new(FakeOwner(world.clone())),
        Arc::new(FakeHealth(world.clone())),
    );
    let alpha = fixture(directory.path(), "alpha", 32101, 32102);
    let beta = fixture(directory.path(), "beta", 32103, 32104);

    let alpha_prepared = prepare(&application, &alpha, "prepare-alpha");
    let replay = prepare(&application, &alpha, "prepare-alpha");
    assert_eq!(alpha_prepared.projected.state, InstanceState::Prepared);
    assert!(replay.idempotent_replay);

    prepare(&application, &beta, "prepare-beta");
    let alpha_running = start(&application, &alpha, "start-alpha");
    let beta_running = start(&application, &beta, "start-beta");
    assert_eq!(alpha_running.projected.state, InstanceState::Running);
    assert_eq!(beta_running.projected.state, InstanceState::Running);
    assert!(!alpha_running.stale);
    assert!(!beta_running.stale);
    assert_ne!(
        alpha_running.projected.owner_route,
        beta_running.projected.owner_route
    );
    let start_replay = start(&application, &alpha, "start-alpha");
    assert!(start_replay.idempotent_replay);

    let stopped = application
        .stop(StopInstanceCommand {
            request_id: request("stop-alpha"),
            authority: authority(),
            instance_id: alpha.identity.instance_id.clone(),
        })
        .expect("stop alpha");
    assert_eq!(stopped.projected.state, InstanceState::Stopped);
    assert!(stopped
        .observed
        .as_ref()
        .expect("observation")
        .health
        .all_closed());
    let stop_replay = application
        .stop(StopInstanceCommand {
            request_id: request("stop-alpha"),
            authority: authority(),
            instance_id: alpha.identity.instance_id.clone(),
        })
        .expect("replay stop alpha");
    assert!(stop_replay.idempotent_replay);

    let beta_observed = application
        .read(ReadInstanceQuery {
            authority: authority(),
            instance_id: beta.identity.instance_id.clone(),
        })
        .expect("read beta after alpha stop");
    assert_eq!(beta_observed.projected.state, InstanceState::Running);
    assert!(beta_observed
        .observed
        .as_ref()
        .expect("beta observation")
        .owner
        .is_active());
    assert!(!beta_observed.stale);

    let wrong_authority = application.stop(StopInstanceCommand {
        request_id: request("unauthorized-stop"),
        authority: AuthoritySecret::new("different-authority-secret").expect("authority"),
        instance_id: beta.identity.instance_id.clone(),
    });
    assert_eq!(
        wrong_authority.expect_err("must reject authority").kind,
        RuntimeApplicationErrorKind::Unauthorized
    );

    let beta_route = beta_running
        .projected
        .owner_route
        .as_ref()
        .expect("beta route")
        .clone();
    world.crash(&beta_route.job_name);
    drop(application);

    let restarted = test_application(
        &registry_path,
        Arc::new(FakeOwner(world.clone())),
        Arc::new(FakeHealth(world)),
    );
    let recovered = restarted
        .recover(RecoverInstanceCommand {
            request_id: request("recover-beta"),
            authority: authority(),
            instance_id: beta.identity.instance_id.clone(),
        })
        .expect("recover stale beta after restart");
    assert_eq!(recovered.projected.state, InstanceState::Recovered);
    assert_eq!(
        recovered.observed.as_ref().expect("observation").owner,
        OwnerObservation::Absent
    );
    assert!(recovered
        .observed
        .as_ref()
        .expect("observation")
        .health
        .all_closed());
    let recovery_replay = restarted
        .recover(RecoverInstanceCommand {
            request_id: request("recover-beta"),
            authority: authority(),
            instance_id: beta.identity.instance_id.clone(),
        })
        .expect("replay recovery");
    assert!(recovery_replay.idempotent_replay);

    let raw = Connection::open(&registry_path).expect("raw registry");
    let authority_hash: String = raw
        .query_row(
            "SELECT authority_hash FROM worktree_runtime_instances WHERE instance_id='alpha'",
            [],
            |row| row.get(0),
        )
        .expect("authority hash");
    assert_ne!(authority_hash, AUTHORITY);
    assert!(!authority_hash.contains(AUTHORITY));
    let lease_count: i64 = raw
        .query_row(
            "SELECT COUNT(*) FROM worktree_runtime_port_leases",
            [],
            |row| row.get(0),
        )
        .expect("lease count");
    assert_eq!(lease_count, 4);
}

#[test]
fn durable_port_leases_and_idempotency_conflicts_fail_closed() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let registry_path = directory.path().join("registry.sqlite");
    let world = Arc::new(FakeWorld::default());
    let application = test_application(
        &registry_path,
        Arc::new(FakeOwner(world.clone())),
        Arc::new(FakeHealth(world.clone())),
    );
    let alpha = fixture(directory.path(), "alpha", 32201, 32202);
    let beta = fixture(directory.path(), "beta", 32201, 32203);
    prepare(&application, &alpha, "prepare-alpha");

    let lease_conflict = application.prepare(PrepareInstanceCommand {
        request_id: request("prepare-beta"),
        authority: authority(),
        identity: beta.identity,
        projection: beta.projection,
    });
    assert_eq!(
        lease_conflict.expect_err("port must remain exclusive").kind,
        RuntimeApplicationErrorKind::PortLeaseConflict
    );

    let semantic_conflict = application.prepare(PrepareInstanceCommand {
        request_id: request("prepare-alpha"),
        authority: authority(),
        identity: fixture(directory.path(), "gamma", 32205, 32206).identity,
        projection: fixture(directory.path(), "gamma", 32205, 32206).projection,
    });
    assert_eq!(
        semantic_conflict
            .expect_err("request semantics must remain stable")
            .kind,
        RuntimeApplicationErrorKind::IdempotencyConflict
    );

    let occupied = fixture(directory.path(), "occupied", 32207, 32208);
    prepare(&application, &occupied, "prepare-occupied");
    world.occupy(occupied.projection.ports.vite);
    let start_conflict = application.start(StartInstanceCommand {
        request_id: request("start-occupied"),
        authority: authority(),
        instance_id: occupied.identity.instance_id.clone(),
        launches: owned_launches(&occupied),
    });
    assert_eq!(
        start_conflict
            .expect_err("pre-existing endpoint must fail closed")
            .kind,
        RuntimeApplicationErrorKind::OwnershipAmbiguous
    );
    let still_prepared = application
        .read(ReadInstanceQuery {
            authority: authority(),
            instance_id: occupied.identity.instance_id,
        })
        .expect("read occupied instance");
    assert_eq!(still_prepared.projected.state, InstanceState::Prepared);
    assert!(still_prepared.stale);
}

#[test]
fn projection_keeps_shared_keyed_caches_separate_from_instance_paths() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let projection = project_instance(ProjectionRequest {
        instance_id: instance("projection"),
        instances_root: directory.path().join("instances"),
        node_cache_root: directory.path().join("cache/npm"),
        rust_cache_root: directory.path().join("instances/projection/cache/rust"),
        node_cache_key: "node-key".into(),
        rust_cache_key: "rust-key".into(),
        node_cache_reuse: CacheReuse::SharedKeyed,
        rust_cache_reuse: CacheReuse::IsolatedFallback,
        ports: PortProjection {
            vite: 32301,
            status: 32302,
        },
    })
    .expect("projection");
    assert!(projection
        .paths
        .frontend_dist
        .starts_with(&projection.paths.instance_root));
    assert!(projection
        .paths
        .temp
        .starts_with(&projection.paths.instance_root));
    assert!(projection
        .paths
        .screenshots
        .starts_with(&projection.paths.instance_root));
    assert!(projection
        .paths
        .recordings
        .starts_with(&projection.paths.instance_root));
    assert!(!projection
        .caches
        .node_path
        .starts_with(&projection.paths.instance_root));
    assert_eq!(
        projection.caches.node_path,
        directory.path().join("cache/npm/node-key")
    );
    let mut false_isolated = projection.clone();
    false_isolated.caches.rust_path = directory.path().join("cache/rust/rust-key");
    assert!(false_isolated.validate().is_err());
    let mut false_shared = projection;
    false_shared.caches.node_path = false_shared.paths.instance_root.join("cache/npm/node-key");
    assert!(false_shared.validate().is_err());
}

#[test]
fn planning_projects_keyed_node_reuse_but_keeps_rust_compilation_instance_local() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fixture = fixture(directory.path(), "planned", 32311, 32312);
    let settings = RuntimeSettings {
        instances_root: directory.path().join("planned-instances"),
        shared_cache_root: directory.path().join("shared-cache"),
        port_start: 32310,
        port_end: 32319,
    };
    let candidates = settings
        .candidate_ports("stable-identity")
        .expect("port candidates");
    assert_eq!(candidates.len(), 5);
    assert_eq!(
        candidates
            .iter()
            .map(|ports| (ports.vite, ports.status))
            .collect::<HashSet<_>>()
            .len(),
        candidates.len()
    );
    let source = SourceSnapshot {
        git_commit: fixture.identity.git_commit.clone(),
        source_fingerprint: fixture.identity.source_fingerprint.clone(),
        node_cache_key: "planned-node-key".into(),
        rust_cache_key: "planned-rust-key".into(),
    };
    let projection = project_runtime(
        &settings,
        &fixture.identity,
        &source,
        PortProjection {
            vite: 32311,
            status: 32312,
        },
    )
    .expect("planned projection");
    assert_eq!(projection.caches.node_reuse, CacheReuse::SharedKeyed);
    assert_eq!(projection.caches.rust_reuse, CacheReuse::IsolatedFallback);
    assert!(projection
        .caches
        .rust_path
        .starts_with(&projection.paths.instance_root));
    assert!(projection.paths.credentials_home.is_dir());
}

#[test]
fn launch_planning_owns_vite_status_and_tauri_with_a_scrubbed_environment() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let fixture = fixture(directory.path(), "launch-plan", 32321, 32322);
    for relative in [
        "node_modules/vite/bin/vite.js",
        "node_modules/@tauri-apps/cli/tauri.js",
        "scripts/runtime-status-server.mjs",
    ] {
        let path = fixture.identity.worktree_path.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("tool parent");
        std::fs::write(path, "fixture").expect("tool file");
    }
    let program = directory.path().join("tool.exe");
    std::fs::write(&program, "fixture").expect("program");
    let launches = launch_plan(
        &fixture.identity,
        &fixture.projection,
        "dev.codex-orchestrator.worktree.launch-plan",
        &ToolchainPrograms {
            git: program.clone(),
            node: program.clone(),
            cargo: program.clone(),
            rustc: program,
        },
    )
    .expect("launch plan");
    assert_eq!(
        launches
            .iter()
            .map(|launch| launch.role)
            .collect::<Vec<_>>(),
        vec![ProcessRole::Vite, ProcessRole::Status, ProcessRole::Tauri]
    );
    for launch in launches {
        assert!(!launch.environment.contains_key("OPENAI_API_KEY"));
        assert_eq!(
            launch.environment["CODEX_HOME"],
            fixture.projection.paths.credentials_home.to_string_lossy()
        );
        assert_eq!(
            launch.environment["CODEX_ORCHESTRATOR_APP_DATA_DIR"],
            fixture.projection.paths.app_data.to_string_lossy()
        );
        assert_eq!(
            launch.environment["VITE_RUNTIME_ROOT"],
            fixture.projection.paths.instance_root.to_string_lossy()
        );
        assert_eq!(
            launch.environment["TEMP"],
            fixture.projection.paths.temp.to_string_lossy()
        );
        assert_eq!(
            launch.environment["TMP"],
            fixture.projection.paths.temp.to_string_lossy()
        );
        assert_eq!(
            launch.environment["RUNTIME_INSTANCE_ID"],
            fixture.identity.instance_id.as_str()
        );
    }
}

#[cfg(windows)]
#[test]
fn named_jobs_prove_dual_instance_stop_isolation_and_restart_recovery() {
    use super::{health::TcpHealthProbe, ownership::WindowsJobProcessOwner};

    let directory = tempfile::tempdir().expect("temporary directory");
    let registry_path = directory.path().join("registry.sqlite");
    let ports = free_ports(4);
    let alpha = fixture(directory.path(), "job-alpha", ports[0], ports[1]);
    let beta = fixture(directory.path(), "job-beta", ports[2], ports[3]);
    let owner = Arc::new(WindowsJobProcessOwner::default());
    let application = WorktreeRuntimeApplication::system(
        Arc::new(SqliteInstanceRegistry::open(&registry_path).expect("registry")),
        owner.clone(),
        Arc::new(TcpHealthProbe::new(Duration::from_millis(30))),
    );
    prepare(&application, &alpha, "job-prepare-alpha");
    prepare(&application, &beta, "job-prepare-beta");
    let alpha_running = start(&application, &alpha, "job-start-alpha");
    let beta_running = start(&application, &beta, "job-start-beta");
    assert!(matches!(
        alpha_running
            .observed
            .as_ref()
            .expect("alpha observation")
            .owner,
        OwnerObservation::Owned { active_processes } if active_processes >= 3
    ));
    assert!(matches!(
        beta_running
            .observed
            .as_ref()
            .expect("beta observation")
            .owner,
        OwnerObservation::Owned { active_processes } if active_processes >= 3
    ));

    application
        .stop(StopInstanceCommand {
            request_id: request("job-stop-alpha"),
            authority: authority(),
            instance_id: alpha.identity.instance_id.clone(),
        })
        .expect("stop alpha job");
    let beta_after_stop = application
        .read(ReadInstanceQuery {
            authority: authority(),
            instance_id: beta.identity.instance_id.clone(),
        })
        .expect("beta remains live");
    assert_eq!(beta_after_stop.projected.state, InstanceState::Running);
    assert!(beta_after_stop
        .observed
        .as_ref()
        .expect("beta observation")
        .owner
        .is_active());
    assert!(beta_after_stop
        .observed
        .as_ref()
        .expect("beta observation")
        .health
        .healthy());

    drop(application);
    drop(owner);
    wait_ports_closed(&[ports[2], ports[3]]);

    let restarted = WorktreeRuntimeApplication::system(
        Arc::new(SqliteInstanceRegistry::open(&registry_path).expect("reopened registry")),
        Arc::new(WindowsJobProcessOwner::default()),
        Arc::new(TcpHealthProbe::new(Duration::from_millis(30))),
    );
    let recovered = restarted
        .recover(RecoverInstanceCommand {
            request_id: request("job-recover-beta"),
            authority: authority(),
            instance_id: beta.identity.instance_id,
        })
        .expect("recover beta stale record");
    assert_eq!(recovered.projected.state, InstanceState::Recovered);
    assert_eq!(
        recovered.observed.as_ref().expect("recovery").owner,
        OwnerObservation::Absent
    );
}

#[cfg(windows)]
#[test]
#[ignore = "helper process launched only by the Windows Job Object integration proof"]
fn owned_child_tcp_server() {
    use std::net::TcpListener;

    let Ok(port) = std::env::var(CHILD_PORT_ENV) else {
        loop {
            std::thread::park();
        }
    };
    let port = port.parse::<u16>().expect("valid child port");
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind child endpoint");
    for connection in listener.incoming() {
        connection.expect("health connection");
    }
}

struct Fixture {
    identity: InstanceIdentity,
    projection: InstanceProjection,
}

fn fixture(root: &Path, name: &str, vite: u16, status: u16) -> Fixture {
    std::fs::create_dir_all(root.join("worktrees").join(name)).expect("fixture worktree");
    Fixture {
        identity: InstanceIdentity {
            instance_id: instance(name),
            review_name: format!("Review {name}"),
            worktree_path: root.join("worktrees").join(name),
            git_commit: "0123456789abcdef0123456789abcdef01234567".into(),
            source_fingerprint: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789"
                .into(),
            build_id: BuildId::new(format!("build-{name}")).expect("build ID"),
            session_link: SessionLink::new(format!("session-{name}")).expect("session link"),
        },
        projection: project_instance(ProjectionRequest {
            instance_id: instance(name),
            instances_root: root.join("instances"),
            node_cache_root: root.join("cache/npm"),
            rust_cache_root: root.join("instances").join(name).join("cache/rust"),
            node_cache_key: "shared-node-key".into(),
            rust_cache_key: "shared-rust-key".into(),
            node_cache_reuse: CacheReuse::SharedKeyed,
            rust_cache_reuse: CacheReuse::IsolatedFallback,
            ports: PortProjection { vite, status },
        })
        .expect("projection"),
    }
}

fn test_application(
    registry_path: &Path,
    owner: Arc<dyn ProcessOwner>,
    health: Arc<dyn HealthProbe>,
) -> WorktreeRuntimeApplication {
    WorktreeRuntimeApplication::new(
        Arc::new(SqliteInstanceRegistry::open(registry_path).expect("registry")),
        owner,
        health,
        Arc::new(TestClock(AtomicI64::new(0))),
        ObservationPolicy {
            attempts: 5,
            interval: Duration::ZERO,
        },
    )
}

fn prepare(
    application: &WorktreeRuntimeApplication,
    fixture: &Fixture,
    request_id: &str,
) -> super::domain::InstanceSnapshot {
    application
        .prepare(PrepareInstanceCommand {
            request_id: request(request_id),
            authority: authority(),
            identity: fixture.identity.clone(),
            projection: fixture.projection.clone(),
        })
        .expect("prepare instance")
}

fn start(
    application: &WorktreeRuntimeApplication,
    fixture: &Fixture,
    request_id: &str,
) -> super::domain::InstanceSnapshot {
    let launches = owned_launches(fixture);
    application
        .start(StartInstanceCommand {
            request_id: request(request_id),
            authority: authority(),
            instance_id: fixture.identity.instance_id.clone(),
            launches,
        })
        .expect("start instance")
}

fn owned_launches(fixture: &Fixture) -> Vec<OwnedProcessLaunch> {
    let executable = std::env::current_exe().expect("test executable");
    [
        (ProcessRole::Vite, Some(fixture.projection.ports.vite)),
        (ProcessRole::Status, Some(fixture.projection.ports.status)),
        (ProcessRole::Tauri, None),
    ]
    .into_iter()
    .map(|(role, port)| OwnedProcessLaunch {
        role,
        program: executable.clone(),
        arguments: if cfg!(windows) {
            vec![
                "--ignored".into(),
                "--exact".into(),
                "worktree_runtime::tests::owned_child_tcp_server".into(),
                "--nocapture".into(),
            ]
        } else {
            Vec::new()
        },
        working_directory: fixture.identity.worktree_path.clone(),
        environment: child_environment(port),
        log_path: fixture.projection.paths.logs.join(format!("{role:?}.log")),
    })
    .collect()
}

fn child_environment(port: Option<u16>) -> BTreeMap<String, String> {
    let mut environment = ["SystemRoot", "SystemDrive", "WINDIR", "TEMP", "TMP"]
        .into_iter()
        .filter_map(|name| std::env::var(name).ok().map(|value| (name.into(), value)))
        .collect::<BTreeMap<_, _>>();
    if let Some(port) = port {
        environment.insert(CHILD_PORT_ENV.into(), port.to_string());
    }
    environment
}

fn instance(value: &str) -> InstanceId {
    InstanceId::new(value).expect("instance ID")
}

fn request(value: &str) -> RequestId {
    RequestId::new(value).expect("request ID")
}

fn authority() -> AuthoritySecret {
    AuthoritySecret::new(AUTHORITY).expect("authority")
}

struct TestClock(AtomicI64);

impl RuntimeClock for TestClock {
    fn now(&self) -> DateTime<Utc> {
        let seconds = self.0.fetch_add(1, Ordering::SeqCst);
        Utc.timestamp_opt(1_800_000_000 + seconds, 0)
            .single()
            .expect("test time")
    }
}

#[derive(Default)]
struct FakeWorld {
    jobs: Mutex<HashMap<String, Vec<u16>>>,
    active_ports: Mutex<HashSet<u16>>,
}

impl FakeWorld {
    fn crash(&self, job_name: &str) {
        if let Some(ports) = self.jobs.lock().expect("jobs").remove(job_name) {
            let mut active = self.active_ports.lock().expect("ports");
            for port in ports {
                active.remove(&port);
            }
        }
    }

    fn occupy(&self, port: u16) {
        self.active_ports.lock().expect("ports").insert(port);
    }
}

struct FakeOwner(Arc<FakeWorld>);

impl ProcessOwner for FakeOwner {
    fn launch(
        &self,
        route: &OwnerRoute,
        launches: &[OwnedProcessLaunch],
    ) -> Result<OwnerObservation, OwnershipError> {
        let ports = launches
            .iter()
            .filter_map(|launch| {
                launch
                    .environment
                    .get(CHILD_PORT_ENV)
                    .map(|port| port.parse::<u16>().expect("port"))
            })
            .collect::<Vec<_>>();
        let mut jobs = self.0.jobs.lock().expect("jobs");
        if jobs.insert(route.job_name.clone(), ports.clone()).is_some() {
            return Err(OwnershipError {
                kind: OwnershipErrorKind::AlreadyExists,
                message: "duplicate fake job".into(),
            });
        }
        self.0.active_ports.lock().expect("ports").extend(ports);
        Ok(OwnerObservation::Owned {
            active_processes: launches.len() as u32,
        })
    }

    fn observe(&self, route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        Ok(self
            .0
            .jobs
            .lock()
            .expect("jobs")
            .get(&route.job_name)
            .map(|ports| OwnerObservation::Owned {
                active_processes: ports.len() as u32,
            })
            .unwrap_or(OwnerObservation::Absent))
    }

    fn terminate(&self, route: &OwnerRoute) -> Result<OwnerObservation, OwnershipError> {
        self.0.crash(&route.job_name);
        Ok(OwnerObservation::Absent)
    }
}

struct FakeHealth(Arc<FakeWorld>);

impl HealthProbe for FakeHealth {
    fn observe(&self, projection: &InstanceProjection) -> HealthObservation {
        let active = self.0.active_ports.lock().expect("ports");
        HealthObservation {
            vite: EndpointObservation {
                port: projection.ports.vite,
                reachable: active.contains(&projection.ports.vite),
            },
            status: EndpointObservation {
                port: projection.ports.status,
                reachable: active.contains(&projection.ports.status),
            },
        }
    }
}

#[cfg(windows)]
fn free_ports(count: usize) -> Vec<u16> {
    use std::net::TcpListener;

    let listeners = (0..count)
        .map(|_| TcpListener::bind(("127.0.0.1", 0)).expect("reserve port"))
        .collect::<Vec<_>>();
    listeners
        .iter()
        .map(|listener| listener.local_addr().expect("address").port())
        .collect()
}

#[cfg(windows)]
fn wait_ports_closed(ports: &[u16]) {
    use std::{
        net::{Ipv4Addr, SocketAddrV4, TcpStream},
        thread,
        time::Instant,
    };

    let deadline = Instant::now() + Duration::from_secs(5);
    while ports.iter().any(|port| {
        TcpStream::connect_timeout(
            &SocketAddrV4::new(Ipv4Addr::LOCALHOST, *port).into(),
            Duration::from_millis(30),
        )
        .is_ok()
    }) {
        assert!(Instant::now() < deadline, "owned endpoints did not close");
        thread::sleep(Duration::from_millis(20));
    }
}
