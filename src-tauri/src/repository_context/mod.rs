mod command;
mod identity;
mod refs;
mod remotes;
mod status;
mod worktrees;

use command::HardenedGitRunner;
use std::{path::Path, sync::Arc};

pub(crate) use crate::git_process::GitExecutable;
pub(crate) use identity::{
    CanonicalDirectory, PathIdentity, RepositoryId, RepositoryIdentity, RepositoryIdentityReader,
};
pub(crate) use refs::{
    BranchRef, BranchSummary, CommitFacts, CommitReader, FullRefName, ObjectId, ReferenceReader,
};
pub(crate) use remotes::{RemoteObservation, RepositoryRemoteReader};
pub(crate) use status::RepositoryStatusReader;
pub(crate) use worktrees::{
    WorktreeInventoryReader, WorktreeLocation, WorktreeObservation, WorktreeObservationId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RepositoryContextErrorKind {
    MissingGit,
    PathUnavailable,
    RepositoryUnavailable,
    InvalidGitOutput,
    OutputLimitExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryContextError {
    pub(crate) kind: RepositoryContextErrorKind,
    message: &'static str,
}

impl RepositoryContextError {
    fn new(kind: RepositoryContextErrorKind, message: &'static str) -> Self {
        Self { kind, message }
    }
}

impl std::fmt::Display for RepositoryContextError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message)
    }
}

/// Read-only Git capabilities shared by product features. Mutation deliberately has no home here.
#[derive(Clone)]
pub(crate) struct RepositoryContext {
    executable: GitExecutable,
    runner: Arc<HardenedGitRunner>,
}

impl RepositoryContext {
    pub(crate) fn discover() -> Result<Self, RepositoryContextError> {
        Self::from_executable(
            GitExecutable::discover().map_err(command::repository_error_from_process)?,
        )
    }

    pub(crate) fn with_git(path: &Path) -> Result<Self, RepositoryContextError> {
        Self::from_executable(
            GitExecutable::resolve(path).map_err(command::repository_error_from_process)?,
        )
    }

    fn from_executable(executable: GitExecutable) -> Result<Self, RepositoryContextError> {
        Ok(Self {
            runner: Arc::new(HardenedGitRunner::new(executable.clone())),
            executable,
        })
    }

    /// The verified executable for feature-owned Git mutations.
    pub(crate) fn git_executable(&self) -> &GitExecutable {
        &self.executable
    }

    pub(crate) fn identities(&self) -> RepositoryIdentityReader {
        RepositoryIdentityReader::new(self.runner.clone())
    }

    pub(crate) fn references(&self) -> ReferenceReader {
        ReferenceReader::new(self.runner.clone())
    }

    pub(crate) fn commits(&self) -> CommitReader {
        CommitReader::new(self.runner.clone())
    }

    pub(crate) fn remotes(&self) -> RepositoryRemoteReader {
        RepositoryRemoteReader::new(self.runner.clone())
    }

    pub(crate) fn status(&self) -> RepositoryStatusReader {
        RepositoryStatusReader::new(self.runner.clone())
    }

    pub(crate) fn worktrees(&self) -> WorktreeInventoryReader {
        WorktreeInventoryReader::new(self.runner.clone())
    }
}

fn invalid_output() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::InvalidGitOutput,
        "Git returned data that does not satisfy the repository contract.",
    )
}
