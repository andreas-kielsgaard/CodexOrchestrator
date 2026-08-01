use super::{
    domain::{
        BuildId, CacheReuse, InstanceId, InstanceIdentity, InstanceProjection, OwnedProcessLaunch,
        PortProjection, ProcessRole, RuntimeContractError, SessionLink,
    },
    projection::{project_instance, ProjectionRequest},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    env,
    error::Error,
    ffi::OsStr,
    fmt, fs,
    path::{Component, Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeSettings {
    pub(crate) instances_root: PathBuf,
    pub(crate) shared_cache_root: PathBuf,
    pub(crate) port_start: u16,
    pub(crate) port_end: u16,
}

impl RuntimeSettings {
    pub(crate) fn validate(&self) -> Result<(), PlanningError> {
        require_absolute(&self.instances_root, "instances root")?;
        require_absolute(&self.shared_cache_root, "shared cache root")?;
        if self.port_start == 0 || self.port_end.saturating_sub(self.port_start) < 3 {
            return Err(PlanningError::new(
                "runtime port range must contain at least four nonzero ports",
            ));
        }
        Ok(())
    }

    pub(crate) fn candidate_ports(
        &self,
        identity_seed: &str,
    ) -> Result<Vec<PortProjection>, PlanningError> {
        self.validate()?;
        let width = usize::from(self.port_end - self.port_start + 1);
        let pair_count = width / 2;
        let digest = Sha256::digest(identity_seed.as_bytes());
        let offset = usize::from(u16::from_be_bytes([digest[0], digest[1]])) % pair_count;
        Ok((0..pair_count)
            .map(|attempt| {
                let pair = (offset + attempt) % pair_count;
                let vite = self.port_start + u16::try_from(pair * 2).expect("bounded port range");
                PortProjection {
                    vite,
                    status: vite + 1,
                }
            })
            .collect())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ToolchainPrograms {
    pub(crate) git: PathBuf,
    pub(crate) node: PathBuf,
    pub(crate) cargo: PathBuf,
    pub(crate) rustc: PathBuf,
}

impl ToolchainPrograms {
    pub(crate) fn discover() -> Result<Self, PlanningError> {
        Ok(Self {
            git: resolve_program("git")?,
            node: resolve_program("node")?,
            cargo: resolve_program("cargo")?,
            rustc: resolve_program("rustc")?,
        })
    }

    pub(crate) fn validate(&self) -> Result<(), PlanningError> {
        for (path, label) in [
            (&self.git, "Git"),
            (&self.node, "Node"),
            (&self.cargo, "Cargo"),
            (&self.rustc, "Rust compiler"),
        ] {
            require_regular_absolute(path, label)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceSnapshot {
    pub(crate) git_commit: String,
    pub(crate) source_fingerprint: String,
    pub(crate) node_cache_key: String,
    pub(crate) rust_cache_key: String,
}

pub(crate) trait SourceInspector: Send + Sync {
    fn inspect(
        &self,
        worktree: &Path,
        programs: &ToolchainPrograms,
    ) -> Result<SourceSnapshot, PlanningError>;
}

pub(crate) struct SystemSourceInspector;

impl SourceInspector for SystemSourceInspector {
    fn inspect(
        &self,
        worktree: &Path,
        programs: &ToolchainPrograms,
    ) -> Result<SourceSnapshot, PlanningError> {
        programs.validate()?;
        let worktree = worktree
            .canonicalize()
            .map_err(|error| PlanningError::context("resolve worktree", error))?;
        require_absolute(&worktree, "worktree")?;

        let commit = run(&programs.git, ["rev-parse", "HEAD"], &worktree)?;
        let git_commit = String::from_utf8(commit)
            .map_err(|_| PlanningError::new("Git commit output was not UTF-8"))?
            .trim()
            .to_owned();
        if git_commit.len() < 7
            || git_commit.len() > 64
            || !git_commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(PlanningError::new(
                "Git returned an invalid commit identity",
            ));
        }

        let tracked_diff = run(
            &programs.git,
            ["diff", "--binary", "--no-ext-diff", "HEAD", "--", "."],
            &worktree,
        )?;
        let untracked = run(
            &programs.git,
            ["ls-files", "--others", "--exclude-standard", "-z"],
            &worktree,
        )?;

        let package_lock = read_regular(&worktree.join("package-lock.json"), "package lock")?;
        let cargo_lock = read_regular(&worktree.join("src-tauri/Cargo.lock"), "Cargo lock")?;
        let cargo_toml = read_regular(&worktree.join("src-tauri/Cargo.toml"), "Cargo manifest")?;
        let node_version = run(&programs.node, ["--version"], &worktree)?;
        let cargo_version = run(&programs.cargo, ["--version"], &worktree)?;
        let rust_version = run(&programs.rustc, ["-vV"], &worktree)?;

        let node_cache_key = keyed_hash(&[
            b"node-cache-v1",
            &package_lock,
            &node_version,
            env::consts::OS.as_bytes(),
            env::consts::ARCH.as_bytes(),
        ]);
        let rust_flags = env::var("RUSTFLAGS").unwrap_or_default();
        let rust_cache_key = keyed_hash(&[
            b"rust-cache-v1",
            &cargo_lock,
            &cargo_toml,
            &cargo_version,
            &rust_version,
            rust_flags.as_bytes(),
            env::consts::OS.as_bytes(),
            env::consts::ARCH.as_bytes(),
        ]);

        let mut source = Sha256::new();
        hash_part(&mut source, b"source-v2");
        hash_part(&mut source, git_commit.as_bytes());
        hash_part(&mut source, &tracked_diff);
        for raw_path in untracked
            .split(|byte| *byte == 0)
            .filter(|path| !path.is_empty())
        {
            let relative = String::from_utf8(raw_path.to_vec())
                .map_err(|_| PlanningError::new("an untracked Git path was not UTF-8"))?;
            let relative_path = Path::new(&relative);
            if relative_path.is_absolute()
                || relative_path
                    .components()
                    .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
            {
                return Err(PlanningError::new("Git returned an unsafe untracked path"));
            }
            let path = worktree.join(relative_path);
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| PlanningError::context("inspect untracked source", error))?;
            if !metadata.file_type().is_file() {
                return Err(PlanningError::new(format!(
                    "untracked source is not a regular file: {}",
                    path.display()
                )));
            }
            hash_part(&mut source, relative.as_bytes());
            hash_part(
                &mut source,
                &fs::read(&path)
                    .map_err(|error| PlanningError::context("read untracked source", error))?,
            );
        }
        hash_part(&mut source, node_cache_key.as_bytes());
        hash_part(&mut source, rust_cache_key.as_bytes());

        Ok(SourceSnapshot {
            git_commit,
            source_fingerprint: format!("{:x}", source.finalize()),
            node_cache_key,
            rust_cache_key,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct InternalIdentity {
    pub(crate) identity: InstanceIdentity,
    pub(crate) tauri_identifier: String,
}

pub(crate) fn derive_identity(
    worktree: PathBuf,
    source_route: &str,
    purpose: &str,
    source: &SourceSnapshot,
) -> Result<InternalIdentity, PlanningError> {
    let seed = keyed_hash(&[
        b"instance-v1",
        source_route.as_bytes(),
        purpose.as_bytes(),
        source.source_fingerprint.as_bytes(),
    ]);
    let suffix = &seed[..20];
    let instance_id = InstanceId::new(format!("wt-{suffix}")).map_err(contract)?;
    let identity = InstanceIdentity {
        instance_id,
        review_name: purpose.to_owned(),
        worktree_path: worktree,
        git_commit: source.git_commit.clone(),
        source_fingerprint: source.source_fingerprint.clone(),
        build_id: BuildId::new(format!("build-{}", &source.source_fingerprint[..20]))
            .map_err(contract)?,
        session_link: SessionLink::new(format!("test-{suffix}")).map_err(contract)?,
    };
    identity.validate().map_err(contract)?;
    Ok(InternalIdentity {
        tauri_identifier: format!("dev.codex-orchestrator.worktree.{suffix}"),
        identity,
    })
}

pub(crate) fn project_runtime(
    settings: &RuntimeSettings,
    identity: &InstanceIdentity,
    source: &SourceSnapshot,
    ports: PortProjection,
) -> Result<InstanceProjection, PlanningError> {
    let instance_root = settings.instances_root.join(identity.instance_id.as_str());
    let shared_node = settings.shared_cache_root.join("npm");
    let node_shared = fs::create_dir_all(shared_node.join(&source.node_cache_key)).is_ok();
    let (node_cache_root, node_reuse) = if node_shared {
        (shared_node, CacheReuse::SharedKeyed)
    } else {
        let node = instance_root.join("cache/npm");
        fs::create_dir_all(node.join(&source.node_cache_key))
            .map_err(|error| PlanningError::context("create isolated cache fallback", error))?;
        (node, CacheReuse::IsolatedFallback)
    };
    // Shared Rust compilation remains unavailable until a measured compiler cache such as sccache
    // is composed. CARGO_TARGET_DIR and this dependency home therefore remain instance-local.
    let rust_cache_root = instance_root.join("cache/cargo-home");
    fs::create_dir_all(rust_cache_root.join(&source.rust_cache_key))
        .map_err(|error| PlanningError::context("create isolated Rust cache", error))?;
    let projection = project_instance(ProjectionRequest {
        instance_id: identity.instance_id.clone(),
        instances_root: settings.instances_root.clone(),
        node_cache_root,
        rust_cache_root,
        node_cache_key: source.node_cache_key.clone(),
        rust_cache_key: source.rust_cache_key.clone(),
        node_cache_reuse: node_reuse,
        rust_cache_reuse: CacheReuse::IsolatedFallback,
        ports,
    })
    .map_err(contract)?;
    create_mutable_roots(&projection)?;
    Ok(projection)
}

fn create_mutable_roots(projection: &InstanceProjection) -> Result<(), PlanningError> {
    for path in [
        &projection.paths.frontend_dist,
        &projection.paths.cargo_target,
        &projection.paths.app_data,
        &projection.paths.app_data.join("roaming"),
        &projection.paths.app_data.join("local"),
        &projection.paths.credentials_home,
        &projection.paths.temp,
        &projection.paths.logs,
        &projection.paths.screenshots,
        &projection.paths.recordings,
        &projection.paths.evidence,
    ] {
        fs::create_dir_all(path)
            .map_err(|error| PlanningError::context("create isolated runtime path", error))?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ActionKind {
    Build,
    Test,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProcessCommand {
    pub(crate) label: &'static str,
    pub(crate) program: PathBuf,
    pub(crate) arguments: Vec<String>,
    pub(crate) working_directory: PathBuf,
    pub(crate) environment: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ActionPlan {
    pub(crate) kind: ActionKind,
    pub(crate) log_path: PathBuf,
    pub(crate) commands: Vec<ProcessCommand>,
}

pub(crate) fn action_plan(
    kind: ActionKind,
    identity: &InstanceIdentity,
    projection: &InstanceProjection,
    tauri_identifier: &str,
    programs: &ToolchainPrograms,
) -> Result<ActionPlan, PlanningError> {
    programs.validate()?;
    let environment = isolated_environment(identity, projection, tauri_identifier);
    let node_modules = identity.worktree_path.join("node_modules");
    let tsc = node_modules.join("typescript/bin/tsc");
    let vite = node_modules.join("vite/bin/vite.js");
    let tauri = node_modules.join("@tauri-apps/cli/tauri.js");
    let vitest = node_modules.join("vitest/vitest.mjs");
    let commands = match kind {
        ActionKind::Build => {
            for (path, label) in [(&tsc, "TypeScript"), (&vite, "Vite"), (&tauri, "Tauri")] {
                require_regular(path, label)?;
            }
            vec![
                node_command(
                    "typecheck",
                    programs,
                    identity,
                    &environment,
                    &tsc,
                    ["--noEmit"],
                ),
                node_command(
                    "frontend build",
                    programs,
                    identity,
                    &environment,
                    &vite,
                    [
                        "build",
                        "--outDir",
                        &projection.paths.frontend_dist.to_string_lossy(),
                        "--emptyOutDir",
                    ],
                ),
                node_command(
                    "Tauri debug build",
                    programs,
                    identity,
                    &environment,
                    &tauri,
                    [
                        "build",
                        "--debug",
                        "--no-bundle",
                        "--config",
                        &build_config(identity, projection, tauri_identifier)?,
                    ],
                ),
            ]
        }
        ActionKind::Test => {
            require_regular(&vitest, "Vitest")?;
            vec![
                node_command(
                    "frontend tests",
                    programs,
                    identity,
                    &environment,
                    &vitest,
                    ["run"],
                ),
                ProcessCommand {
                    label: "Rust tests",
                    program: programs.cargo.clone(),
                    arguments: vec![
                        "test".into(),
                        "--manifest-path".into(),
                        external_value(&identity.worktree_path.join("src-tauri/Cargo.toml")),
                    ],
                    working_directory: external_path(&identity.worktree_path),
                    environment,
                },
            ]
        }
    };
    let name = match kind {
        ActionKind::Build => "build.log",
        ActionKind::Test => "test.log",
    };
    Ok(ActionPlan {
        kind,
        log_path: projection.paths.logs.join(name),
        commands,
    })
}

pub(crate) fn launch_plan(
    identity: &InstanceIdentity,
    projection: &InstanceProjection,
    tauri_identifier: &str,
    programs: &ToolchainPrograms,
) -> Result<Vec<OwnedProcessLaunch>, PlanningError> {
    programs.validate()?;
    let vite = identity.worktree_path.join("node_modules/vite/bin/vite.js");
    let status = identity
        .worktree_path
        .join("scripts/runtime-status-server.mjs");
    let executable = projection
        .paths
        .cargo_target
        .join("debug/codex-orchestrator.exe");
    for (path, label) in [
        (&vite, "Vite"),
        (&status, "status server"),
        (&executable, "verified worktree build"),
    ] {
        require_regular(path, label)?;
    }
    let environment = isolated_environment(identity, projection, tauri_identifier);
    Ok(vec![
        OwnedProcessLaunch {
            role: ProcessRole::Vite,
            program: programs.node.clone(),
            arguments: vec![
                external_value(&vite),
                "--host".into(),
                "127.0.0.1".into(),
                "--port".into(),
                projection.ports.vite.to_string(),
                "--strictPort".into(),
            ],
            working_directory: external_path(&identity.worktree_path),
            environment: environment.clone(),
            log_path: projection.paths.logs.join("vite.log"),
        },
        OwnedProcessLaunch {
            role: ProcessRole::Status,
            program: programs.node.clone(),
            arguments: vec![external_value(&status)],
            working_directory: external_path(&identity.worktree_path),
            environment: environment.clone(),
            log_path: projection.paths.logs.join("status.log"),
        },
        OwnedProcessLaunch {
            role: ProcessRole::Tauri,
            program: external_path(&executable),
            arguments: Vec::new(),
            working_directory: external_path(&projection.paths.cargo_target.join("debug")),
            environment,
            log_path: projection.paths.logs.join("tauri.log"),
        },
    ])
}

fn node_command<'a, const N: usize>(
    label: &'static str,
    programs: &ToolchainPrograms,
    identity: &InstanceIdentity,
    environment: &BTreeMap<String, String>,
    script: &Path,
    arguments: [&'a str; N],
) -> ProcessCommand {
    ProcessCommand {
        label,
        program: programs.node.clone(),
        arguments: std::iter::once(external_value(script))
            .chain(arguments.into_iter().map(str::to_owned))
            .collect(),
        working_directory: external_path(&identity.worktree_path),
        environment: environment.clone(),
    }
}

fn external_value(path: &Path) -> String {
    let value = path.to_string_lossy();
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{unc}");
    }
    value.strip_prefix(r"\\?\").unwrap_or(&value).to_owned()
}

fn external_path(path: &Path) -> PathBuf {
    PathBuf::from(external_value(path))
}

fn build_config(
    identity: &InstanceIdentity,
    projection: &InstanceProjection,
    tauri_identifier: &str,
) -> Result<String, PlanningError> {
    let frontend_dist = tauri_frontend_dist(identity, projection)?;
    serde_json::to_string(&json!({
        "identifier": tauri_identifier,
        "build": {
            "beforeBuildCommand": null,
            "frontendDist": frontend_dist,
        },
        "app": {
            "windows": [{
                "label": "main",
                "title": format!("Codex Orchestrator [Worktree build: {}]", identity.review_name),
                "width": 1280,
                "height": 820,
                "minWidth": 960,
                "minHeight": 640,
                "visible": false,
                "focus": false,
                "resizable": true
            }],
        },
        "bundle": { "active": false },
    }))
    .map_err(|error| PlanningError::context("serialize Tauri build configuration", error))
}

fn development_config(
    identity: &InstanceIdentity,
    projection: &InstanceProjection,
    tauri_identifier: &str,
) -> Result<String, PlanningError> {
    let frontend_dist = tauri_frontend_dist(identity, projection)?;
    serde_json::to_string(&json!({
        "identifier": tauri_identifier,
        "build": {
            "beforeDevCommand": null,
            "devUrl": format!("http://127.0.0.1:{}", projection.ports.vite),
            "frontendDist": frontend_dist,
        },
        "app": {
            "windows": [{
                "label": "main",
                "title": format!("Codex Orchestrator [Worktree build: {}]", identity.review_name),
                "width": 1280,
                "height": 820,
                "minWidth": 960,
                "minHeight": 640,
            }],
        },
        "bundle": { "active": false },
    }))
    .map_err(|error| PlanningError::context("serialize Tauri development configuration", error))
}

fn tauri_frontend_dist(
    identity: &InstanceIdentity,
    projection: &InstanceProjection,
) -> Result<String, PlanningError> {
    let config_directory = external_path(&identity.worktree_path.join("src-tauri"));
    let frontend_dist = external_path(&projection.paths.frontend_dist);
    pathdiff::diff_paths(&frontend_dist, &config_directory)
        .filter(|path| !path.as_os_str().is_empty() && path.is_relative())
        .map(|path| external_value(&path))
        .ok_or_else(|| {
            PlanningError::new(
                "isolated frontend output must be representable relative to the Tauri config",
            )
        })
}

fn isolated_environment(
    identity: &InstanceIdentity,
    projection: &InstanceProjection,
    tauri_identifier: &str,
) -> BTreeMap<String, String> {
    let mut environment = [
        "SystemRoot",
        "SystemDrive",
        "WINDIR",
        "PATH",
        "ComSpec",
        "PATHEXT",
    ]
    .into_iter()
    .filter_map(|name| env::var(name).ok().map(|value| (name.to_owned(), value)))
    .collect::<BTreeMap<_, _>>();
    let value = |path: &Path| path.to_string_lossy().into_owned();
    let cache_mode = match projection.caches.rust_reuse {
        CacheReuse::SharedKeyed => "shared_keyed",
        CacheReuse::IsolatedFallback => "isolated_fallback",
    };
    let rustup_home = env::var_os("RUSTUP_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join(".rustup")));
    for (name, value) in [
        ("HOME", value(&projection.paths.credentials_home)),
        ("USERPROFILE", value(&projection.paths.credentials_home)),
        ("CODEX_HOME", value(&projection.paths.credentials_home)),
        (
            "CODEX_ORCHESTRATOR_APP_DATA_DIR",
            value(&projection.paths.app_data),
        ),
        (
            "CODEX_ORCHESTRATOR_WORKTREE_BUILD_PATH",
            value(&identity.worktree_path),
        ),
        (
            "CODEX_ORCHESTRATOR_WORKTREE_BUILD_NAME",
            identity.review_name.clone(),
        ),
        (
            "CODEX_ORCHESTRATOR_WORKTREE_READY_PATH",
            value(&projection.paths.app_data.join("review-window-ready")),
        ),
        (
            "CODEX_ORCHESTRATOR_REVIEW_INSTANCE_ROOT",
            value(&projection.paths.instance_root),
        ),
        (
            "CODEX_ORCHESTRATOR_REVIEW_NAVIGATION_PATH",
            value(
                &projection
                    .paths
                    .app_data
                    .join("debug-proof-navigation.json"),
            ),
        ),
        ("APPDATA", value(&projection.paths.app_data.join("roaming"))),
        (
            "LOCALAPPDATA",
            value(&projection.paths.app_data.join("local")),
        ),
        ("TEMP", value(&projection.paths.temp)),
        ("TMP", value(&projection.paths.temp)),
        ("TMPDIR", value(&projection.paths.temp)),
        (
            "CODEX_ORCHESTRATOR_APP_DATA_DIR",
            value(&projection.paths.app_data),
        ),
        ("CARGO_HOME", value(&projection.caches.rust_path)),
        ("CARGO_TARGET_DIR", value(&projection.paths.cargo_target)),
        ("npm_config_cache", value(&projection.caches.node_path)),
        (
            "RUNTIME_STATUS_FILE",
            value(&projection.paths.instance_root.join("runtime-status.json")),
        ),
        ("RUNTIME_STATUS_HOST", "127.0.0.1".into()),
        ("RUNTIME_STATUS_PORT", projection.ports.status.to_string()),
        (
            "RUNTIME_INSTANCE_ID",
            identity.instance_id.as_str().to_owned(),
        ),
        (
            "RUNTIME_SESSION_ID",
            identity.session_link.as_str().to_owned(),
        ),
        ("RUNTIME_WORKTREE_PATH", value(&identity.worktree_path)),
        ("RUNTIME_GIT_COMMIT", identity.git_commit.clone()),
        (
            "VITE_RUNTIME_STATUS_URL",
            format!("http://127.0.0.1:{}/status", projection.ports.status),
        ),
        (
            "VITE_RUNTIME_INSTANCE_ID",
            identity.instance_id.as_str().to_owned(),
        ),
        (
            "VITE_RUNTIME_SESSION_ID",
            identity.session_link.as_str().to_owned(),
        ),
        ("VITE_RUNTIME_WORKTREE_PATH", value(&identity.worktree_path)),
        ("VITE_RUNTIME_GIT_COMMIT", identity.git_commit.clone()),
        (
            "VITE_RUNTIME_SOURCE_FINGERPRINT",
            identity.source_fingerprint.clone(),
        ),
        ("VITE_RUNTIME_TAURI_IDENTIFIER", tauri_identifier.to_owned()),
        ("VITE_RUNTIME_REVIEW_NAME", identity.review_name.clone()),
        ("VITE_HUMAN_REVIEW_INSTANCE", "true".into()),
        ("VITE_RUNTIME_ROOT", value(&projection.paths.instance_root)),
        ("VITE_RUNTIME_DIST", value(&projection.paths.frontend_dist)),
        (
            "VITE_RUNTIME_CARGO_TARGET",
            value(&projection.paths.cargo_target),
        ),
        ("VITE_RUNTIME_APP_DATA", value(&projection.paths.app_data)),
        (
            "VITE_RUNTIME_CREDENTIALS",
            value(&projection.paths.credentials_home),
        ),
        ("VITE_RUNTIME_LOGS", value(&projection.paths.logs)),
        (
            "VITE_RUNTIME_SCREENSHOTS",
            value(&projection.paths.screenshots),
        ),
        (
            "VITE_RUNTIME_RECORDINGS",
            value(&projection.paths.recordings),
        ),
        (
            "VITE_RUNTIME_NODE_CACHE_KEY",
            projection.caches.node_key.clone(),
        ),
        (
            "VITE_RUNTIME_NODE_CACHE_PATH",
            value(&projection.caches.node_path),
        ),
        (
            "VITE_RUNTIME_RUST_CACHE_KEY",
            projection.caches.rust_key.clone(),
        ),
        (
            "VITE_RUNTIME_RUST_CACHE_PATH",
            value(&projection.caches.rust_path),
        ),
        ("VITE_RUNTIME_RUST_CACHE_MODE", cache_mode.into()),
        ("VITE_RUNTIME_VITE_PORT", projection.ports.vite.to_string()),
        (
            "VITE_RUNTIME_STATUS_PORT",
            projection.ports.status.to_string(),
        ),
    ] {
        environment.insert(name.into(), value);
    }
    if let Some(rustup_home) = rustup_home {
        environment.insert("RUSTUP_HOME".into(), value(&rustup_home));
    }
    environment
}

fn run<I, S>(program: &Path, arguments: I, cwd: &Path) -> Result<Vec<u8>, PlanningError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new(program)
        .args(arguments)
        .current_dir(cwd)
        .output()
        .map_err(|error| PlanningError::context("run source inspection command", error))?;
    if !output.status.success() {
        return Err(PlanningError::new(format!(
            "source inspection command failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(output.stdout)
}

fn read_regular(path: &Path, label: &str) -> Result<Vec<u8>, PlanningError> {
    require_regular(path, label)?;
    fs::read(path).map_err(|error| PlanningError::context(format!("read {label}"), error))
}

fn require_regular(path: &Path, label: &str) -> Result<(), PlanningError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| PlanningError::context(format!("inspect {label}"), error))?;
    if !metadata.file_type().is_file() {
        return Err(PlanningError::new(format!(
            "{label} must be a regular file: {}",
            path.display()
        )));
    }
    Ok(())
}

fn require_regular_absolute(path: &Path, label: &str) -> Result<(), PlanningError> {
    require_absolute(path, label)?;
    require_regular(path, label)
}

fn require_absolute(path: &Path, label: &str) -> Result<(), PlanningError> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(PlanningError::new(format!(
            "{label} must be absolute and normalized"
        )));
    }
    Ok(())
}

fn resolve_program(name: &str) -> Result<PathBuf, PlanningError> {
    let path = env::var_os("PATH").ok_or_else(|| PlanningError::new("PATH is unavailable"))?;
    let extensions = if cfg!(windows) {
        vec!["exe", "cmd", "bat"]
    } else {
        vec![""]
    };
    for directory in env::split_paths(&path) {
        for extension in &extensions {
            let candidate = if extension.is_empty() {
                directory.join(name)
            } else {
                directory.join(format!("{name}.{extension}"))
            };
            if candidate.is_file() {
                return candidate
                    .canonicalize()
                    .map_err(|error| PlanningError::context("resolve program", error));
            }
        }
    }
    Err(PlanningError::new(format!("{name} was not found on PATH")))
}

fn keyed_hash(parts: &[&[u8]]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hash_part(&mut hasher, part);
    }
    format!("{:x}", hasher.finalize())
}

fn hash_part(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn contract(error: RuntimeContractError) -> PlanningError {
    PlanningError::new(error.to_string())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlanningError {
    pub(crate) message: String,
}

impl PlanningError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    fn context(operation: impl AsRef<str>, error: impl fmt::Display) -> Self {
        Self::new(format!("{}: {error}", operation.as_ref()))
    }
}

impl fmt::Display for PlanningError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PlanningError {}
