use super::{RepositoryContextError, RepositoryContextErrorKind};
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(super) const SMALL_OUTPUT_LIMIT: usize = 256 * 1024;
pub(super) const LARGE_OUTPUT_LIMIT: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GitExecutable(PathBuf);

impl GitExecutable {
    pub(crate) fn discover() -> Result<Self, RepositoryContextError> {
        let path = env::var_os("PATH").ok_or_else(missing_git)?;
        #[cfg(windows)]
        let names = ["git.exe", "git"];
        #[cfg(not(windows))]
        let names = ["git"];
        for directory in env::split_paths(&path) {
            for name in names {
                let candidate = directory.join(name);
                if candidate.is_file() {
                    return Self::canonical(candidate);
                }
            }
        }
        Err(missing_git())
    }

    pub(crate) fn resolve(path: &Path) -> Result<Self, RepositoryContextError> {
        if path.is_absolute() && path.is_file() {
            return Self::canonical(path.to_path_buf());
        }
        if matches!(path.to_str(), Some("git" | "git.exe")) {
            return Self::discover();
        }
        Err(missing_git())
    }

    fn canonical(path: PathBuf) -> Result<Self, RepositoryContextError> {
        fs::canonicalize(path).map(Self).map_err(|_| missing_git())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

pub(super) struct GitOutcome {
    pub(super) success: bool,
    pub(super) exit_code: Option<i32>,
    pub(super) stdout: Vec<u8>,
}

pub(super) struct HardenedGitRunner {
    executable: GitExecutable,
}

impl HardenedGitRunner {
    pub(super) fn new(executable: GitExecutable) -> Self {
        Self { executable }
    }

    pub(super) fn required<I, S>(
        &self,
        root: &Path,
        arguments: I,
        output_limit: usize,
    ) -> Result<Vec<u8>, RepositoryContextError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let outcome = self.run(root, arguments, output_limit)?;
        if outcome.success {
            Ok(outcome.stdout)
        } else {
            Err(repository_unavailable())
        }
    }

    pub(super) fn optional<I, S>(
        &self,
        root: &Path,
        arguments: I,
        output_limit: usize,
    ) -> Result<Option<Vec<u8>>, RepositoryContextError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let outcome = self.run(root, arguments, output_limit)?;
        match (outcome.success, outcome.exit_code) {
            (true, _) => Ok(Some(outcome.stdout)),
            (false, Some(1)) => Ok(None),
            (false, _) => Err(repository_unavailable()),
        }
    }

    fn run<I, S>(
        &self,
        root: &Path,
        arguments: I,
        output_limit: usize,
    ) -> Result<GitOutcome, RepositoryContextError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let executable_directory = self.executable.path().parent().ok_or_else(missing_git)?;
        let mut command = Command::new(self.executable.path());
        command
            .env_clear()
            .env("PATH", minimal_child_path(executable_directory)?)
            .env("LC_ALL", "C")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", null_device())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .args([
                "--no-pager",
                "--no-replace-objects",
                "--literal-pathspecs",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "credential.interactive=false",
                "-c",
                "diff.external=",
            ])
            .arg("-c")
            .arg(format!("core.hooksPath={}", null_device()))
            .args(arguments)
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        add_platform_environment(&mut command);
        let mut child = command.spawn().map_err(|_| repository_unavailable())?;
        let mut stdout = Vec::new();
        child
            .stdout
            .take()
            .ok_or_else(repository_unavailable)?
            .take(output_limit as u64 + 1)
            .read_to_end(&mut stdout)
            .map_err(|_| repository_unavailable())?;
        if stdout.len() > output_limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RepositoryContextError::new(
                RepositoryContextErrorKind::OutputLimitExceeded,
                "Git returned more data than this operation permits.",
            ));
        }
        let status = child.wait().map_err(|_| repository_unavailable())?;
        Ok(GitOutcome {
            success: status.success(),
            exit_code: status.code(),
            stdout,
        })
    }
}

fn minimal_child_path(executable_directory: &Path) -> Result<OsString, RepositoryContextError> {
    let mut directories = vec![executable_directory.to_path_buf()];
    #[cfg(windows)]
    if let Some(system_root) = env::var_os("SystemRoot") {
        let system32 = PathBuf::from(system_root).join("System32");
        if system32.is_dir() {
            directories.push(system32);
        }
    }
    env::join_paths(directories).map_err(|_| repository_unavailable())
}

#[cfg(windows)]
fn add_platform_environment(command: &mut Command) {
    if let Some(system_root) = env::var_os("SystemRoot") {
        command
            .env("SystemRoot", &system_root)
            .env("WINDIR", system_root);
    }
}

#[cfg(not(windows))]
fn add_platform_environment(_command: &mut Command) {}

#[cfg(windows)]
fn null_device() -> &'static str {
    "NUL"
}

#[cfg(not(windows))]
fn null_device() -> &'static str {
    "/dev/null"
}

fn missing_git() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::MissingGit,
        "Git is unavailable.",
    )
}

fn repository_unavailable() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::RepositoryUnavailable,
        "Git repository inspection is unavailable.",
    )
}
