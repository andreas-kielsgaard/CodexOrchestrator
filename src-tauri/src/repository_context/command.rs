use super::{GitExecutable, RepositoryContextError, RepositoryContextErrorKind};
use std::{
    env,
    ffi::{OsStr, OsString},
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub(super) const SMALL_OUTPUT_LIMIT: usize = 256 * 1024;
pub(super) const LARGE_OUTPUT_LIMIT: usize = 16 * 1024 * 1024;

pub(super) struct GitOutcome {
    pub(super) success: bool,
    pub(super) stdout: Vec<u8>,
}

pub(super) struct HardenedGitRunner {
    executable: GitExecutable,
}

impl HardenedGitRunner {
    pub(super) fn new(executable: GitExecutable) -> Self {
        Self { executable }
    }

    pub(super) fn run<I, S>(
        &self,
        root: &Path,
        arguments: I,
        output_limit: usize,
    ) -> Result<GitOutcome, RepositoryContextError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let executable_directory = self.executable.path().parent().ok_or_else(|| {
            RepositoryContextError::new(
                RepositoryContextErrorKind::MissingGit,
                "Git has an invalid executable location.",
            )
        })?;
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
                "-c",
                "core.fsmonitor=false",
                "-c",
                "credential.interactive=false",
                "-c",
                "diff.external=",
                "-c",
                platform_line_endings(),
            ])
            .arg("-c")
            .arg(format!("core.hooksPath={}", null_device()))
            .args(arguments)
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        add_platform_environment(&mut command);
        let mut child = command.spawn().map_err(|_| {
            RepositoryContextError::new(
                RepositoryContextErrorKind::MissingGit,
                "Git could not be started.",
            )
        })?;
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
fn platform_line_endings() -> &'static str {
    // Keep Windows checkout normalization deterministic without restoring ambient Git config.
    "core.autocrlf=true"
}

#[cfg(not(windows))]
fn platform_line_endings() -> &'static str {
    "core.autocrlf=false"
}

#[cfg(windows)]
fn null_device() -> &'static str {
    "NUL"
}

#[cfg(not(windows))]
fn null_device() -> &'static str {
    "/dev/null"
}

fn repository_unavailable() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::RepositoryUnavailable,
        "Git repository inspection is unavailable.",
    )
}
