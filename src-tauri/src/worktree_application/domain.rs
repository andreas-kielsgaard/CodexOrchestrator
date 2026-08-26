use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsString,
    fmt,
    path::PathBuf,
};

const MAX_LAUNCH_ENVIRONMENT_ENTRIES: usize = 16;
const MAX_LAUNCH_ENVIRONMENT_UNITS: usize = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PhysicalWorktreeBuildRequest {
    pub(crate) worktree_root: PathBuf,
    pub(crate) output_root: PathBuf,
    pub(crate) cargo_binary_name: String,
}

impl PhysicalWorktreeBuildRequest {
    pub(crate) fn new(
        worktree_root: PathBuf,
        output_root: PathBuf,
        cargo_binary_name: impl Into<String>,
    ) -> Result<Self, WorktreeApplicationError> {
        let cargo_binary_name = cargo_binary_name.into();
        if !worktree_root.is_absolute() || !output_root.is_absolute() {
            return Err(WorktreeApplicationError::new(
                WorktreeApplicationErrorKind::InvalidRequest,
                "The worktree and output roots must be absolute paths.",
            ));
        }
        let binary_path = std::path::Path::new(&cargo_binary_name);
        if cargo_binary_name.trim().is_empty()
            || binary_path.file_name().and_then(|name| name.to_str()) != Some(&cargo_binary_name)
            || matches!(cargo_binary_name.as_str(), "." | "..")
        {
            return Err(WorktreeApplicationError::new(
                WorktreeApplicationErrorKind::InvalidRequest,
                "The application binary name must be one file name.",
            ));
        }
        Ok(Self {
            worktree_root,
            output_root,
            cargo_binary_name,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PhysicalWorktreeBuildResult {
    pub(crate) worktree_root: PathBuf,
    pub(crate) output_root: PathBuf,
    pub(crate) executable: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorktreeApplicationLaunchContext {
    pub(super) environment: BTreeMap<OsString, OsString>,
}

impl WorktreeApplicationLaunchContext {
    pub(crate) fn new<K, V>(
        environment: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Self, WorktreeApplicationError>
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        let mut bounded = BTreeMap::new();
        let mut key_identities = BTreeSet::new();
        let mut units = 0;
        for (key, value) in environment {
            if bounded.len() == MAX_LAUNCH_ENVIRONMENT_ENTRIES {
                return Err(invalid_launch_context());
            }
            let key = key.into();
            let value = value.into();
            let key_text = key.to_string_lossy();
            let value_text = value.to_string_lossy();
            if key_text.is_empty()
                || key_text.contains('\0')
                || key_text.contains('=')
                || value_text.contains('\0')
            {
                return Err(invalid_launch_context());
            }
            units += key_text.chars().count() + value_text.chars().count();
            let key_identity = if cfg!(windows) {
                key_text.to_ascii_lowercase()
            } else {
                key_text.into_owned()
            };
            if units > MAX_LAUNCH_ENVIRONMENT_UNITS
                || !key_identities.insert(key_identity)
                || bounded.insert(key, value).is_some()
            {
                return Err(invalid_launch_context());
            }
        }
        Ok(Self {
            environment: bounded,
        })
    }
}

fn invalid_launch_context() -> WorktreeApplicationError {
    WorktreeApplicationError::new(
        WorktreeApplicationErrorKind::InvalidRequest,
        "The application launch environment is invalid or too large.",
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OpenOutcome {
    ExistingWindowActivationRequested,
    DetachedLaunchStarted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorktreeApplicationErrorKind {
    InvalidRequest,
    WorktreeUnavailable,
    OutputUnavailable,
    ToolchainUnavailable,
    BuildFailed,
    ExecutableUnavailable,
    OpenFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorktreeApplicationError {
    pub(crate) kind: WorktreeApplicationErrorKind,
    pub(crate) message: String,
}

impl WorktreeApplicationError {
    pub(crate) fn new(kind: WorktreeApplicationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for WorktreeApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for WorktreeApplicationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_name_is_a_file_name_not_an_output_path() {
        let root = std::env::temp_dir().join("worktree-application-request");
        assert!(PhysicalWorktreeBuildRequest::new(
            root.join("worktree"),
            root.join("output"),
            "codex-orchestrator"
        )
        .is_ok());
        for invalid in ["", ".", "..", "target/codex-orchestrator"] {
            assert_eq!(
                PhysicalWorktreeBuildRequest::new(
                    root.join("worktree"),
                    root.join("output"),
                    invalid
                )
                .unwrap_err()
                .kind,
                WorktreeApplicationErrorKind::InvalidRequest
            );
        }
        assert_eq!(
            PhysicalWorktreeBuildRequest::new(
                "relative-worktree".into(),
                root.join("output"),
                "codex-orchestrator"
            )
            .unwrap_err()
            .kind,
            WorktreeApplicationErrorKind::InvalidRequest
        );
    }

    #[test]
    fn launch_context_is_a_bounded_unambiguous_environment_overlay() {
        assert!(WorktreeApplicationLaunchContext::new([
            ("APPLICATION_DATA", "C:/application-data"),
            ("ACTIVE_BUILD", "opaque-build"),
            ("ACTIVE_WORKTREE", "opaque-worktree"),
        ])
        .is_ok());
        assert_eq!(
            WorktreeApplicationLaunchContext::new([("INVALID=KEY", "value")])
                .unwrap_err()
                .kind,
            WorktreeApplicationErrorKind::InvalidRequest
        );
        assert_eq!(
            WorktreeApplicationLaunchContext::new([("DUPLICATE", "one"), ("DUPLICATE", "two"),])
                .unwrap_err()
                .kind,
            WorktreeApplicationErrorKind::InvalidRequest
        );
    }
}
