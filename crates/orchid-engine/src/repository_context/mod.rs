mod command;
mod commits;
mod identity;
mod refs;
mod remotes;
mod status;
mod worktrees;

use command::HardenedGitRunner;
use std::{path::Path, sync::Arc};

pub use crate::git_process::GitExecutable;
pub use commits::{CommitFacts, CommitParents, CommitReader};
pub use identity::{
    CanonicalDirectory, PathIdentity, RepositoryId, RepositoryIdentity, RepositoryIdentityReader,
};
pub use refs::{BranchRef, BranchSummary, FullRefName, ObjectId, ReferenceReader};
pub use remotes::{RemoteObservation, RepositoryRemoteReader};
pub use status::RepositoryStatusReader;
pub use worktrees::{
    WorktreeInventoryReader, WorktreeLocation, WorktreeObservation, WorktreeObservationId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryContextErrorKind {
    MissingGit,
    PathUnavailable,
    RepositoryUnavailable,
    InvalidGitOutput,
    OutputLimitExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryContextError {
    pub kind: RepositoryContextErrorKind,
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
pub struct RepositoryContext {
    executable: GitExecutable,
    runner: Arc<HardenedGitRunner>,
}

impl RepositoryContext {
    pub fn discover() -> Result<Self, RepositoryContextError> {
        Self::from_executable(
            GitExecutable::discover().map_err(command::repository_error_from_process)?,
        )
    }

    pub fn with_git(path: &Path) -> Result<Self, RepositoryContextError> {
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
    pub fn git_executable(&self) -> &GitExecutable {
        &self.executable
    }

    pub fn identities(&self) -> RepositoryIdentityReader {
        RepositoryIdentityReader::new(self.runner.clone())
    }

    pub fn references(&self) -> ReferenceReader {
        ReferenceReader::new(self.runner.clone())
    }

    pub fn commits(&self) -> CommitReader {
        CommitReader::new(self.runner.clone())
    }

    pub fn remotes(&self) -> RepositoryRemoteReader {
        RepositoryRemoteReader::new(self.runner.clone())
    }

    pub fn status(&self) -> RepositoryStatusReader {
        RepositoryStatusReader::new(self.runner.clone())
    }

    pub fn worktrees(&self) -> WorktreeInventoryReader {
        WorktreeInventoryReader::new(self.runner.clone())
    }
}

fn invalid_output() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::InvalidGitOutput,
        "Git returned data that does not satisfy the repository contract.",
    )
}
