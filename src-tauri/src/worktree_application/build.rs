use super::domain::{
    PhysicalWorktreeBuildRequest, PhysicalWorktreeBuildResult, WorktreeApplicationError,
    WorktreeApplicationErrorKind,
};
use serde_json::json;
use std::{
    env,
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct BuildCommand {
    label: &'static str,
    program: PathBuf,
    arguments: Vec<OsString>,
    working_directory: PathBuf,
    cargo_target: PathBuf,
    log_path: PathBuf,
}

trait BuildCommandRunner {
    fn run(&self, command: &BuildCommand) -> Result<bool, WorktreeApplicationError>;
}

struct SystemBuildCommandRunner;

impl BuildCommandRunner for SystemBuildCommandRunner {
    fn run(&self, command: &BuildCommand) -> Result<bool, WorktreeApplicationError> {
        let mut log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&command.log_path)
            .map_err(|_| output_unavailable())?;
        writeln!(log, "== {} ==", command.label).map_err(|_| output_unavailable())?;
        let error_log = log.try_clone().map_err(|_| output_unavailable())?;
        Command::new(&command.program)
            .args(&command.arguments)
            .current_dir(&command.working_directory)
            .env("CARGO_TARGET_DIR", &command.cargo_target)
            .env_remove("CARGO_BUILD_TARGET")
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(error_log))
            .status()
            .map(|status| status.success())
            .map_err(|_| {
                WorktreeApplicationError::new(
                    WorktreeApplicationErrorKind::ToolchainUnavailable,
                    format!("The {} command could not be started.", command.label),
                )
            })
    }
}

pub(super) fn build(
    request: &PhysicalWorktreeBuildRequest,
) -> Result<PhysicalWorktreeBuildResult, WorktreeApplicationError> {
    let node = resolve_program("node")?;
    let npm = resolve_program("npm")?;
    build_with(request, node, npm, &SystemBuildCommandRunner)
}

fn build_with(
    request: &PhysicalWorktreeBuildRequest,
    node: PathBuf,
    npm: PathBuf,
    runner: &dyn BuildCommandRunner,
) -> Result<PhysicalWorktreeBuildResult, WorktreeApplicationError> {
    let worktree = canonical_directory(&request.worktree_root)?;
    if request.attempt_root.starts_with(&worktree)
        || request.dependency_cache_root.starts_with(&worktree)
    {
        return Err(WorktreeApplicationError::new(
            WorktreeApplicationErrorKind::InvalidRequest,
            "Build storage must be outside the physical worktree.",
        ));
    }
    let attempt_root = prepare_attempt_root(&request.attempt_root)?;
    let dependency_cache = prepare_cache_root(&request.dependency_cache_root)?;
    if attempt_root.starts_with(&worktree) || dependency_cache.starts_with(&worktree) {
        return Err(WorktreeApplicationError::new(
            WorktreeApplicationErrorKind::InvalidRequest,
            "The build attempt root must be outside the physical worktree.",
        ));
    }
    let output_root = attempt_root.join("output");
    if fs::symlink_metadata(&output_root).is_ok() {
        return Err(WorktreeApplicationError::new(
            WorktreeApplicationErrorKind::OutputUnavailable,
            "The immutable build output has already been published.",
        ));
    }
    require_source_file(&worktree.join("package.json"), "package manifest")?;
    require_source_file(&worktree.join("package-lock.json"), "npm lockfile")?;
    require_source_file(&worktree.join("src-tauri/Cargo.toml"), "Cargo manifest")?;
    let scratch_root = attempt_root.join(format!(".build-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&scratch_root).map_err(|_| output_unavailable())?;
    let scratch_root = scratch_root
        .canonicalize()
        .map_err(|_| output_unavailable())?;
    let log_path = attempt_root.join("build.log");
    fs::write(&log_path, []).map_err(|_| output_unavailable())?;
    let result = (|| {
        let dependency =
            dependency_command(&worktree, &scratch_root, &log_path, npm, dependency_cache);
        run_required(runner, &dependency)?;
        let commands = build_commands(&worktree, &scratch_root, &log_path, node)?;
        for command in &commands {
            run_required(runner, command)?;
        }
        let scratch_executable = expected_executable(&scratch_root, &request.cargo_binary_name);
        require_regular(&scratch_executable, "built application executable").map_err(|_| {
            WorktreeApplicationError::new(
                WorktreeApplicationErrorKind::ExecutableUnavailable,
                "The build completed without the expected application executable.",
            )
        })?;
        fs::rename(&scratch_root, &output_root).map_err(|_| output_unavailable())?;
        let output_root = output_root
            .canonicalize()
            .map_err(|_| output_unavailable())?;
        let executable = expected_executable(&output_root, &request.cargo_binary_name);
        Ok(PhysicalWorktreeBuildResult {
            worktree_root: worktree,
            attempt_root,
            output_root,
            log_path,
            executable: fs::canonicalize(executable).map_err(|_| {
                WorktreeApplicationError::new(
                    WorktreeApplicationErrorKind::ExecutableUnavailable,
                    "The published application executable is unavailable.",
                )
            })?,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&scratch_root);
    }
    result
}

fn run_required(
    runner: &dyn BuildCommandRunner,
    command: &BuildCommand,
) -> Result<(), WorktreeApplicationError> {
    if runner.run(command)? {
        Ok(())
    } else {
        Err(WorktreeApplicationError::new(
            WorktreeApplicationErrorKind::BuildFailed,
            format!("The {} command failed.", command.label),
        ))
    }
}

fn dependency_command(
    worktree: &Path,
    scratch_root: &Path,
    log_path: &Path,
    npm: PathBuf,
    cache: PathBuf,
) -> BuildCommand {
    BuildCommand {
        label: "npm dependency preparation",
        program: npm,
        arguments: vec![
            "ci".into(),
            "--prefer-offline".into(),
            "--no-audit".into(),
            "--no-fund".into(),
            "--cache".into(),
            external_path(&cache).into_os_string(),
        ],
        working_directory: external_path(worktree),
        cargo_target: external_path(&scratch_root.join("cargo-target")),
        log_path: log_path.to_path_buf(),
    }
}

fn build_commands(
    worktree: &Path,
    output_root: &Path,
    log_path: &Path,
    node: PathBuf,
) -> Result<Vec<BuildCommand>, WorktreeApplicationError> {
    let type_script = require_regular(
        &worktree.join("node_modules/typescript/bin/tsc"),
        "TypeScript compiler",
    )?;
    let vite = require_regular(
        &worktree.join("node_modules/vite/bin/vite.js"),
        "Vite compiler",
    )?;
    let tauri = require_regular(
        &worktree.join("node_modules/@tauri-apps/cli/tauri.js"),
        "Tauri CLI",
    )?;
    let working_directory = external_path(worktree);
    let tauri_directory = external_path(&worktree.join("src-tauri"));
    let cargo_target = external_path(&output_root.join("cargo-target"));
    let frontend_dist = external_path(&output_root.join("frontend-dist"));
    let frontend_dist_config = pathdiff::diff_paths(&frontend_dist, &tauri_directory)
        .filter(|path| path.is_relative() && !path.as_os_str().is_empty())
        .map(|path| external_path(&path).to_string_lossy().into_owned())
        .ok_or_else(|| {
            WorktreeApplicationError::new(
                WorktreeApplicationErrorKind::InvalidRequest,
                "The output root cannot be addressed from this physical worktree.",
            )
        })?;
    let tauri_config = serde_json::to_string(&json!({
        "build": {
            "beforeBuildCommand": null,
            "frontendDist": frontend_dist_config
        },
        "bundle": { "active": false }
    }))
    .map_err(|_| {
        WorktreeApplicationError::new(
            WorktreeApplicationErrorKind::InvalidRequest,
            "The physical-worktree build configuration is unavailable.",
        )
    })?;
    Ok(vec![
        BuildCommand {
            label: "TypeScript typecheck",
            program: node.clone(),
            arguments: vec![
                external_path(&type_script).into_os_string(),
                "--noEmit".into(),
            ],
            working_directory: working_directory.clone(),
            cargo_target: cargo_target.clone(),
            log_path: log_path.to_path_buf(),
        },
        BuildCommand {
            label: "frontend build",
            program: node.clone(),
            arguments: vec![
                external_path(&vite).into_os_string(),
                "build".into(),
                "--outDir".into(),
                frontend_dist.into_os_string(),
                "--emptyOutDir".into(),
            ],
            working_directory: working_directory.clone(),
            cargo_target: cargo_target.clone(),
            log_path: log_path.to_path_buf(),
        },
        BuildCommand {
            label: "Tauri debug build",
            program: node,
            arguments: vec![
                external_path(&tauri).into_os_string(),
                "build".into(),
                "--debug".into(),
                "--no-bundle".into(),
                "--config".into(),
                tauri_config.into(),
            ],
            working_directory,
            cargo_target,
            log_path: log_path.to_path_buf(),
        },
    ])
}

fn external_path(path: &Path) -> PathBuf {
    let value = path.to_string_lossy();
    let value = value.as_ref();
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{unc}"));
    }
    PathBuf::from(value.strip_prefix(r"\\?\").unwrap_or(value))
}

fn expected_executable(output_root: &Path, binary_name: &str) -> PathBuf {
    #[cfg(windows)]
    let binary_name = format!("{binary_name}.exe");
    output_root.join("cargo-target/debug").join(binary_name)
}

fn canonical_directory(path: &Path) -> Result<PathBuf, WorktreeApplicationError> {
    fs::canonicalize(path)
        .ok()
        .filter(|path| path.is_dir())
        .ok_or_else(|| {
            WorktreeApplicationError::new(
                WorktreeApplicationErrorKind::WorktreeUnavailable,
                "The physical worktree directory is unavailable.",
            )
        })
}

fn prepare_attempt_root(path: &Path) -> Result<PathBuf, WorktreeApplicationError> {
    fs::create_dir_all(path).map_err(|_| output_unavailable())?;
    fs::canonicalize(path)
        .ok()
        .filter(|path| path.is_dir())
        .ok_or_else(output_unavailable)
}

fn prepare_cache_root(path: &Path) -> Result<PathBuf, WorktreeApplicationError> {
    fs::create_dir_all(path).map_err(|_| output_unavailable())?;
    fs::canonicalize(path)
        .ok()
        .filter(|path| path.is_dir())
        .ok_or_else(output_unavailable)
}

fn require_regular(path: &Path, label: &str) -> Result<PathBuf, WorktreeApplicationError> {
    require_file(
        path,
        label,
        WorktreeApplicationErrorKind::ToolchainUnavailable,
    )
}

fn require_source_file(path: &Path, label: &str) -> Result<PathBuf, WorktreeApplicationError> {
    require_file(
        path,
        label,
        WorktreeApplicationErrorKind::WorktreeUnavailable,
    )
}

fn require_file(
    path: &Path,
    label: &str,
    kind: WorktreeApplicationErrorKind,
) -> Result<PathBuf, WorktreeApplicationError> {
    let unavailable =
        || WorktreeApplicationError::new(kind, format!("The {label} is unavailable."));
    let canonical = fs::canonicalize(path).map_err(|_| unavailable())?;
    if !canonical.is_file() {
        return Err(unavailable());
    }
    Ok(canonical)
}

fn resolve_program(name: &str) -> Result<PathBuf, WorktreeApplicationError> {
    let path = env::var_os("PATH").ok_or_else(toolchain_unavailable)?;
    #[cfg(windows)]
    let names = [
        format!("{name}.cmd"),
        format!("{name}.exe"),
        name.to_owned(),
    ];
    #[cfg(not(windows))]
    let names = [name.to_owned()];
    for directory in env::split_paths(&path) {
        for candidate_name in &names {
            let candidate = directory.join(candidate_name);
            if candidate.is_file() {
                return fs::canonicalize(candidate).map_err(|_| toolchain_unavailable());
            }
        }
    }
    Err(toolchain_unavailable())
}

fn toolchain_unavailable() -> WorktreeApplicationError {
    WorktreeApplicationError::new(
        WorktreeApplicationErrorKind::ToolchainUnavailable,
        "Node.js and npm are required to build the worktree application.",
    )
}

fn output_unavailable() -> WorktreeApplicationError {
    WorktreeApplicationError::new(
        WorktreeApplicationErrorKind::OutputUnavailable,
        "The physical-worktree build output root is unavailable.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct RecordingRunner {
        commands: Mutex<Vec<BuildCommand>>,
        fail_at: Option<&'static str>,
    }

    impl BuildCommandRunner for RecordingRunner {
        fn run(&self, command: &BuildCommand) -> Result<bool, WorktreeApplicationError> {
            self.commands.lock().unwrap().push(command.clone());
            if self.fail_at == Some(command.label) {
                return Ok(false);
            }
            if command.label == "Tauri debug build" {
                let executable =
                    expected_executable(command.cargo_target.parent().unwrap(), "sample-app");
                fs::create_dir_all(executable.parent().unwrap()).unwrap();
                fs::write(executable, b"application").unwrap();
            }
            Ok(true)
        }
    }

    #[test]
    fn build_targets_the_selected_physical_worktree() {
        let directory = tempfile::tempdir().unwrap();
        let worktree = fixture(directory.path());
        let attempt_root = directory.path().join("attempt");
        let request = PhysicalWorktreeBuildRequest::new(
            worktree.clone(),
            attempt_root.clone(),
            directory.path().join("npm-cache"),
            "sample-app",
        )
        .unwrap();
        let runner = RecordingRunner {
            commands: Mutex::new(Vec::new()),
            fail_at: None,
        };

        let result = build_with(
            &request,
            worktree.join("node"),
            worktree.join("npm"),
            &runner,
        )
        .unwrap();

        assert_eq!(result.worktree_root, worktree.canonicalize().unwrap());
        assert_eq!(result.attempt_root, attempt_root.canonicalize().unwrap());
        assert_eq!(result.output_root, result.attempt_root.join("output"));
        assert_eq!(
            result.executable,
            expected_executable(&result.output_root, "sample-app")
                .canonicalize()
                .unwrap()
        );
        assert_eq!(result.log_path, result.attempt_root.join("build.log"));
        assert!(result.log_path.is_file());
        let commands = runner.commands.lock().unwrap();
        assert_eq!(
            commands
                .iter()
                .map(|command| command.label)
                .collect::<Vec<_>>(),
            [
                "npm dependency preparation",
                "TypeScript typecheck",
                "frontend build",
                "Tauri debug build"
            ]
        );
        assert!(commands.iter().all(|command| {
            command.cargo_target.ends_with(Path::new("cargo-target"))
                && command.working_directory == external_path(&worktree.canonicalize().unwrap())
                && command.log_path == attempt_root.canonicalize().unwrap().join("build.log")
        }));
        assert!(!result.attempt_root.join(".build").exists());
        assert!(!worktree.join("src-tauri/target").exists());
    }

    #[test]
    fn failed_build_reports_the_exact_step_and_stops() {
        let directory = tempfile::tempdir().unwrap();
        let worktree = fixture(directory.path());
        let attempt_root = directory.path().join("attempt");
        let request = PhysicalWorktreeBuildRequest::new(
            worktree.clone(),
            attempt_root.clone(),
            directory.path().join("npm-cache"),
            "sample-app",
        )
        .unwrap();
        let runner = RecordingRunner {
            commands: Mutex::new(Vec::new()),
            fail_at: Some("frontend build"),
        };

        let error = build_with(
            &request,
            worktree.join("node"),
            worktree.join("npm"),
            &runner,
        )
        .unwrap_err();

        assert_eq!(error.kind, WorktreeApplicationErrorKind::BuildFailed);
        assert!(error.message.contains("frontend build"));
        assert_eq!(
            runner
                .commands
                .lock()
                .unwrap()
                .iter()
                .map(|command| command.label)
                .collect::<Vec<_>>(),
            [
                "npm dependency preparation",
                "TypeScript typecheck",
                "frontend build"
            ]
        );
        assert!(!attempt_root.join("output").exists());
    }

    fn fixture(root: &Path) -> PathBuf {
        let worktree = root.join("worktree");
        for path in [
            "package.json",
            "package-lock.json",
            "src-tauri/Cargo.toml",
            "node_modules/typescript/bin/tsc",
            "node_modules/vite/bin/vite.js",
            "node_modules/@tauri-apps/cli/tauri.js",
            "node",
            "npm",
        ] {
            let path = worktree.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"fixture").unwrap();
        }
        worktree
    }
}
