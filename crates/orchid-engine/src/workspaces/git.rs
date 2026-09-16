use super::domain::{CheckoutError, CheckoutErrorKind, GitCommitId};
use crate::git_process::{
    GitCommandEnvironment, GitExecutable, GitProcessError, GitProcessErrorKind, HardenedGitProcess,
};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

const GIT_OUTPUT_LIMIT: usize = 1024 * 1024;

pub(super) struct GitRunner {
    process: HardenedGitProcess,
}

impl GitRunner {
    pub(super) fn new(executable: &GitExecutable) -> Self {
        Self {
            process: HardenedGitProcess::new(executable.clone()),
        }
    }

    pub(super) fn required<I, S>(&self, root: &Path, arguments: I) -> Result<Vec<u8>, CheckoutError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.required_with(root, arguments, GitCommandEnvironment::Clean)
    }

    pub(super) fn optional<I, S>(
        &self,
        root: &Path,
        arguments: I,
    ) -> Result<Option<Vec<u8>>, CheckoutError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let outcome = self
            .process
            .run(
                root,
                arguments,
                GIT_OUTPUT_LIMIT,
                GitCommandEnvironment::Clean,
            )
            .map_err(git_process_error)?;
        match (outcome.success, outcome.exit_code) {
            (true, _) => Ok(Some(outcome.stdout)),
            (false, Some(1)) => Ok(None),
            (false, _) => Err(git_command_failed()),
        }
    }

    fn required_with<I, S>(
        &self,
        root: &Path,
        arguments: I,
        environment: GitCommandEnvironment<'_>,
    ) -> Result<Vec<u8>, CheckoutError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let outcome = self
            .process
            .run(root, arguments, GIT_OUTPUT_LIMIT, environment)
            .map_err(git_process_error)?;
        if outcome.success {
            Ok(outcome.stdout)
        } else {
            Err(git_command_failed())
        }
    }
}

pub(super) fn git_text(bytes: Vec<u8>) -> Result<String, CheckoutError> {
    String::from_utf8(bytes)
        .map(|value| value.trim().to_owned())
        .map_err(|_| git_output_invalid())
}

pub(super) fn git_commit_id(bytes: Vec<u8>) -> Result<GitCommitId, CheckoutError> {
    GitCommitId::new(git_text(bytes)?)
}

pub(super) fn git_path(root: &Path, bytes: Vec<u8>) -> Result<PathBuf, CheckoutError> {
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

fn git_process_error(error: GitProcessError) -> CheckoutError {
    match error.kind {
        GitProcessErrorKind::OutputLimitExceeded => git_output_invalid(),
        GitProcessErrorKind::MissingExecutable | GitProcessErrorKind::StartFailed => {
            CheckoutError::new(
                CheckoutErrorKind::GitUnavailable,
                "Git could not be started for the physical worktree operation.",
            )
        }
    }
}

fn git_command_failed() -> CheckoutError {
    CheckoutError::new(
        CheckoutErrorKind::GitUnavailable,
        "Git could not complete the physical worktree operation.",
    )
}

fn git_output_invalid() -> CheckoutError {
    CheckoutError::new(
        CheckoutErrorKind::GitUnavailable,
        "Git returned invalid physical worktree evidence.",
    )
}
