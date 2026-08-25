use super::{RepositoryContextError, RepositoryContextErrorKind};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PathIdentity(String);

impl PathIdentity {
    pub(crate) fn of(path: &Path) -> Self {
        let value = path.to_string_lossy().replace('\\', "/");
        let value = value.strip_prefix("//?/").unwrap_or(&value);
        let value = value.trim_end_matches('/');
        Self(if cfg!(windows) {
            value.to_ascii_lowercase()
        } else {
            value.to_owned()
        })
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CanonicalDirectory {
    path: PathBuf,
    identity: PathIdentity,
}

impl CanonicalDirectory {
    pub(crate) fn resolve(path: &Path) -> Result<Self, RepositoryContextError> {
        let path = fs::canonicalize(path).map_err(|_| unavailable())?;
        if !path.is_dir() {
            return Err(unavailable());
        }
        let identity = PathIdentity::of(&path);
        Ok(Self { path, identity })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn identity(&self) -> &PathIdentity {
        &self.identity
    }
}

fn unavailable() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::PathUnavailable,
        "The selected directory is unavailable.",
    )
}
