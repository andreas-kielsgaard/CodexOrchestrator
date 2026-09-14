use super::{
    command::{HardenedGitRunner, SMALL_OUTPUT_LIMIT},
    invalid_output, RepositoryContextError, RepositoryContextErrorKind,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PathIdentity(String);

impl PathIdentity {
    pub fn of(path: &Path) -> Self {
        let value = path.to_string_lossy().replace('\\', "/");
        let value = value
            .strip_prefix("//?/")
            .unwrap_or(&value)
            .trim_end_matches('/');
        Self(if cfg!(windows) {
            value.to_ascii_lowercase()
        } else {
            value.to_owned()
        })
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalDirectory {
    path: PathBuf,
    identity: PathIdentity,
}

impl CanonicalDirectory {
    pub fn resolve(path: &Path) -> Result<Self, RepositoryContextError> {
        let path = fs::canonicalize(path).map_err(|_| path_unavailable())?;
        if !path.is_dir() {
            return Err(path_unavailable());
        }
        let identity = PathIdentity::of(&path);
        Ok(Self { path, identity })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn identity(&self) -> &PathIdentity {
        &self.identity
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RepositoryId(String);

impl RepositoryId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[cfg(test)]
    pub(super) fn test_value(value: &str) -> Self {
        Self(value.to_owned())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryIdentity {
    pub id: RepositoryId,
    pub top_level: CanonicalDirectory,
    pub common_directory: CanonicalDirectory,
}

#[derive(Clone)]
pub struct RepositoryIdentityReader {
    runner: Arc<HardenedGitRunner>,
}

impl RepositoryIdentityReader {
    pub(super) fn new(runner: Arc<HardenedGitRunner>) -> Self {
        Self { runner }
    }

    pub fn inspect(&self, candidate: &Path) -> Result<RepositoryIdentity, RepositoryContextError> {
        let candidate = CanonicalDirectory::resolve(candidate)?;
        let top_level = self.canonical_git_path(
            candidate.path(),
            ["rev-parse", "--path-format=absolute", "--show-toplevel"],
        )?;
        let common_directory = self.canonical_git_path(
            candidate.path(),
            ["rev-parse", "--path-format=absolute", "--git-common-dir"],
        )?;
        let mut hash = Sha256::new();
        hash.update(b"codex-orchestrator/repository/v1");
        hash.update(common_directory.identity().as_str().as_bytes());
        Ok(RepositoryIdentity {
            id: RepositoryId(format!(
                "repository-{}",
                &format!("{:x}", hash.finalize())[..24]
            )),
            top_level,
            common_directory,
        })
    }

    fn canonical_git_path<I, S>(
        &self,
        root: &Path,
        arguments: I,
    ) -> Result<CanonicalDirectory, RepositoryContextError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        let text = text(self.runner.required(root, arguments, SMALL_OUTPUT_LIMIT)?)?;
        let path = PathBuf::from(text);
        let resolved = if path.is_absolute() {
            path
        } else {
            root.join(path)
        };
        CanonicalDirectory::resolve(&resolved)
    }
}

fn text(bytes: Vec<u8>) -> Result<String, RepositoryContextError> {
    let value = std::str::from_utf8(&bytes)
        .map_err(|_| invalid_output())?
        .trim();
    if value.is_empty() || value.contains('\0') {
        Err(invalid_output())
    } else {
        Ok(value.to_owned())
    }
}

fn path_unavailable() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::PathUnavailable,
        "The selected directory is unavailable.",
    )
}
