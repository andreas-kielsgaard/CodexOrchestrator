use super::{RepositoryContextError, RepositoryContextErrorKind};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GitExecutable(PathBuf);

impl GitExecutable {
    pub(crate) fn discover() -> Result<Self, RepositoryContextError> {
        let path = env::var_os("PATH").ok_or_else(|| {
            RepositoryContextError::new(
                RepositoryContextErrorKind::MissingGit,
                "Git is not available on PATH.",
            )
        })?;
        #[cfg(windows)]
        let names = ["git.exe", "git"];
        #[cfg(not(windows))]
        let names = ["git"];
        for directory in env::split_paths(&path) {
            for name in names {
                let candidate = directory.join(name);
                if candidate.is_file() {
                    return fs::canonicalize(candidate)
                        .map(Self)
                        .map_err(|_| unavailable_git());
                }
            }
        }
        Err(unavailable_git())
    }

    pub(crate) fn resolve(path: &Path) -> Result<Self, RepositoryContextError> {
        if path.is_absolute() {
            if !path.is_file() {
                return Err(unavailable_git());
            }
            return fs::canonicalize(path)
                .map(Self)
                .map_err(|_| unavailable_git());
        }
        if path == Path::new("git") || path == Path::new("git.exe") {
            return Self::discover();
        }
        Err(unavailable_git())
    }

    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}

fn unavailable_git() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::MissingGit,
        "Git is unavailable.",
    )
}
