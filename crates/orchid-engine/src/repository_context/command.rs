use super::{RepositoryContextError, RepositoryContextErrorKind};
use crate::git_process::{
    GitCommandEnvironment, GitExecutable, GitProcessError, GitProcessErrorKind, HardenedGitProcess,
};
use std::{ffi::OsStr, path::Path};

pub(super) const SMALL_OUTPUT_LIMIT: usize = 256 * 1024;
pub(super) const LARGE_OUTPUT_LIMIT: usize = 16 * 1024 * 1024;

pub(super) struct HardenedGitRunner {
    process: HardenedGitProcess,
}

impl HardenedGitRunner {
    pub(super) fn new(executable: GitExecutable) -> Self {
        Self {
            process: HardenedGitProcess::new(executable),
        }
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
    ) -> Result<crate::git_process::GitProcessOutcome, RepositoryContextError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.process
            .run(root, arguments, output_limit, GitCommandEnvironment::Clean)
            .map_err(repository_error_from_process)
    }
}

pub(super) fn repository_error_from_process(error: GitProcessError) -> RepositoryContextError {
    match error.kind {
        GitProcessErrorKind::MissingExecutable => missing_git(),
        GitProcessErrorKind::OutputLimitExceeded => RepositoryContextError::new(
            RepositoryContextErrorKind::OutputLimitExceeded,
            "Git returned more data than this operation permits.",
        ),
        GitProcessErrorKind::StartFailed => repository_unavailable(),
    }
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
