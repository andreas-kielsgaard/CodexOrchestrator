use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GitExecutable(PathBuf);

impl GitExecutable {
    pub(crate) fn discover() -> Result<Self, GitProcessError> {
        let path = env::var_os("PATH").ok_or_else(missing_executable)?;
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
        Err(missing_executable())
    }

    pub(crate) fn resolve(path: &Path) -> Result<Self, GitProcessError> {
        if path.is_absolute() && path.is_file() {
            return Self::canonical(path.to_path_buf());
        }
        if matches!(path.to_str(), Some("git" | "git.exe")) {
            return Self::discover();
        }
        Err(missing_executable())
    }

    fn canonical(path: PathBuf) -> Result<Self, GitProcessError> {
        fs::canonicalize(path)
            .map(Self)
            .map_err(|_| missing_executable())
    }

    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum GitCommandEnvironment<'a> {
    Clean,
    IsolatedIndex(&'a Path),
    DeterministicCommit {
        identity: &'a str,
        email: &'a str,
        date: &'a str,
    },
}

pub(crate) struct GitProcessOutcome {
    pub(crate) success: bool,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: Vec<u8>,
}

pub(crate) struct HardenedGitProcess {
    executable: GitExecutable,
}

impl HardenedGitProcess {
    pub(crate) fn new(executable: GitExecutable) -> Self {
        Self { executable }
    }

    pub(crate) fn run<I, S>(
        &self,
        root: &Path,
        arguments: I,
        output_limit: usize,
        environment: GitCommandEnvironment<'_>,
    ) -> Result<GitProcessOutcome, GitProcessError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let executable_directory = self
            .executable
            .path()
            .parent()
            .ok_or_else(missing_executable)?;
        let mut command = Command::new(self.executable.path());
        command
            .env_clear()
            .env("PATH", minimal_child_path(executable_directory)?)
            .env("LC_ALL", "C")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", null_device())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_EDITOR", "true")
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
        apply_platform_environment(&mut command);
        apply_explicit_environment(&mut command, environment);

        let mut child = command.spawn().map_err(|_| start_failed())?;
        let mut stdout = Vec::new();
        if child
            .stdout
            .take()
            .ok_or_else(start_failed)?
            .take(output_limit as u64 + 1)
            .read_to_end(&mut stdout)
            .is_err()
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(start_failed());
        }
        if stdout.len() > output_limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(GitProcessError::new(
                GitProcessErrorKind::OutputLimitExceeded,
                "Git returned more data than this operation permits.",
            ));
        }
        let status = child.wait().map_err(|_| start_failed())?;
        Ok(GitProcessOutcome {
            success: status.success(),
            exit_code: status.code(),
            stdout,
        })
    }
}

fn apply_explicit_environment(command: &mut Command, environment: GitCommandEnvironment<'_>) {
    match environment {
        GitCommandEnvironment::Clean => {}
        GitCommandEnvironment::IsolatedIndex(path) => {
            command.env("GIT_INDEX_FILE", external_path(path));
        }
        GitCommandEnvironment::DeterministicCommit {
            identity,
            email,
            date,
        } => {
            command
                .env("GIT_AUTHOR_NAME", identity)
                .env("GIT_AUTHOR_EMAIL", email)
                .env("GIT_AUTHOR_DATE", date)
                .env("GIT_COMMITTER_NAME", identity)
                .env("GIT_COMMITTER_EMAIL", email)
                .env("GIT_COMMITTER_DATE", date);
        }
    }
}

fn minimal_child_path(executable_directory: &Path) -> Result<OsString, GitProcessError> {
    let mut directories = vec![executable_directory.to_path_buf()];
    #[cfg(windows)]
    if let Some(system_root) = env::var_os("SystemRoot") {
        let system32 = PathBuf::from(system_root).join("System32");
        if system32.is_dir() {
            directories.push(system32);
        }
    }
    env::join_paths(directories).map_err(|_| start_failed())
}

#[cfg(windows)]
fn apply_platform_environment(command: &mut Command) {
    if let Some(system_root) = env::var_os("SystemRoot") {
        command
            .env("SystemRoot", &system_root)
            .env("WINDIR", system_root);
    }
}

#[cfg(not(windows))]
fn apply_platform_environment(_command: &mut Command) {}

fn external_path(path: &Path) -> OsString {
    let value = path.to_string_lossy();
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        return OsString::from(format!(r"\\{unc}"));
    }
    OsString::from(value.strip_prefix(r"\\?\").unwrap_or(&value))
}

fn null_device() -> &'static str {
    if cfg!(windows) {
        "NUL"
    } else {
        "/dev/null"
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GitProcessErrorKind {
    MissingExecutable,
    StartFailed,
    OutputLimitExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GitProcessError {
    pub(crate) kind: GitProcessErrorKind,
    message: &'static str,
}

impl GitProcessError {
    fn new(kind: GitProcessErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }
}

impl std::fmt::Display for GitProcessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

fn missing_executable() -> GitProcessError {
    GitProcessError::new(
        GitProcessErrorKind::MissingExecutable,
        "Git is unavailable.",
    )
}

fn start_failed() -> GitProcessError {
    GitProcessError::new(
        GitProcessErrorKind::StartFailed,
        "Git could not be started or observed.",
    )
}
