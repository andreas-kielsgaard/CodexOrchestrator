use super::domain::{GitCommitId, WorktreeApplicationError, WorktreeApplicationErrorKind};
use crate::repository_context::GitExecutable;
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

const GIT_OUTPUT_LIMIT: usize = 1024 * 1024;

pub(super) struct GitRunner<'a> {
    executable: &'a GitExecutable,
}

impl<'a> GitRunner<'a> {
    pub(super) fn new(executable: &'a GitExecutable) -> Self {
        Self { executable }
    }

    pub(super) fn required<I, S>(
        &self,
        root: &Path,
        arguments: I,
    ) -> Result<Vec<u8>, WorktreeApplicationError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.output(root, arguments, None, &[])?;
        if output.status.success() {
            bounded_stdout(output)
        } else {
            Err(git_command_failed())
        }
    }

    pub(super) fn required_with_index<I, S>(
        &self,
        root: &Path,
        arguments: I,
        index: &Path,
    ) -> Result<Vec<u8>, WorktreeApplicationError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.output(root, arguments, Some(index), &[])?;
        if output.status.success() {
            bounded_stdout(output)
        } else {
            Err(git_command_failed())
        }
    }

    pub(super) fn required_with_environment<I, S>(
        &self,
        root: &Path,
        arguments: I,
        environment: &[(&str, &str)],
    ) -> Result<Vec<u8>, WorktreeApplicationError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.output(root, arguments, None, environment)?;
        if output.status.success() {
            bounded_stdout(output)
        } else {
            Err(git_command_failed())
        }
    }

    pub(super) fn optional<I, S>(
        &self,
        root: &Path,
        arguments: I,
    ) -> Result<Option<Vec<u8>>, WorktreeApplicationError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.output(root, arguments, None, &[])?;
        match output.status.code() {
            Some(0) => bounded_stdout(output).map(Some),
            Some(1) => Ok(None),
            _ => Err(git_command_failed()),
        }
    }

    fn output<I, S>(
        &self,
        root: &Path,
        arguments: I,
        index: Option<&Path>,
        environment: &[(&str, &str)],
    ) -> Result<Output, WorktreeApplicationError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = Command::new(self.executable.path());
        command
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_EDITOR", "true")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", null_device())
            .env("LC_ALL", "C")
            .arg("--no-pager")
            .arg("--no-replace-objects")
            .arg("--literal-pathspecs")
            .arg("-c")
            .arg("core.fsmonitor=false")
            .arg("-c")
            .arg("credential.interactive=false")
            .arg("-c")
            .arg(format!("core.hooksPath={}", null_device()))
            .args(arguments);
        if let Some(index) = index {
            command.env("GIT_INDEX_FILE", external_path(index));
        }
        for (key, value) in environment {
            command.env(key, value);
        }
        command.output().map_err(|_| {
            WorktreeApplicationError::new(
                WorktreeApplicationErrorKind::GitUnavailable,
                "Git could not be started for the physical worktree operation.",
            )
        })
    }
}

pub(super) fn git_text(bytes: Vec<u8>) -> Result<String, WorktreeApplicationError> {
    String::from_utf8(bytes)
        .map(|value| value.trim().to_owned())
        .map_err(|_| git_output_invalid())
}

pub(super) fn git_commit_id(bytes: Vec<u8>) -> Result<GitCommitId, WorktreeApplicationError> {
    GitCommitId::new(git_text(bytes)?)
}

pub(super) fn git_path(root: &Path, bytes: Vec<u8>) -> Result<PathBuf, WorktreeApplicationError> {
    let path = PathBuf::from(git_text(bytes)?);
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    path.canonicalize().map_err(|_| git_output_invalid())
}

pub(super) fn external_path(path: &Path) -> OsString {
    let value = path.to_string_lossy();
    if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        return OsString::from(format!(r"\\{unc}"));
    }
    OsString::from(value.strip_prefix(r"\\?\").unwrap_or(&value))
}

fn bounded_stdout(output: Output) -> Result<Vec<u8>, WorktreeApplicationError> {
    if output.stdout.len() <= GIT_OUTPUT_LIMIT {
        Ok(output.stdout)
    } else {
        Err(git_output_invalid())
    }
}

fn git_command_failed() -> WorktreeApplicationError {
    WorktreeApplicationError::new(
        WorktreeApplicationErrorKind::GitUnavailable,
        "Git could not complete the physical worktree operation.",
    )
}

fn git_output_invalid() -> WorktreeApplicationError {
    WorktreeApplicationError::new(
        WorktreeApplicationErrorKind::GitUnavailable,
        "Git returned invalid physical worktree evidence.",
    )
}

fn null_device() -> &'static str {
    if cfg!(windows) {
        "NUL"
    } else {
        "/dev/null"
    }
}
