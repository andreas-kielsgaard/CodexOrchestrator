use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    ffi::OsString,
    fmt,
    path::PathBuf,
};

const MAX_LAUNCH_ENVIRONMENT_ENTRIES: usize = 16;
const MAX_LAUNCH_ENVIRONMENT_UNITS: usize = 16 * 1024;
const MAX_GIT_ID_UNITS: usize = 128;
const MAX_GIT_REF_UNITS: usize = 512;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct GitCommitId(String);

impl GitCommitId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, WorktreeApplicationError> {
        let value = value.into();
        if !matches!(value.len(), 40 | 64)
            || value.len() > MAX_GIT_ID_UNITS
            || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(invalid_request("The exact Git commit identity is invalid."));
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VirtualCommitCaptureRequest {
    pub(crate) worktree_root: PathBuf,
    pub(crate) expected_head: GitCommitId,
}

impl VirtualCommitCaptureRequest {
    pub(crate) fn new(
        worktree_root: PathBuf,
        expected_head: GitCommitId,
    ) -> Result<Self, WorktreeApplicationError> {
        require_absolute(&worktree_root, "The source worktree root must be absolute.")?;
        Ok(Self {
            worktree_root,
            expected_head,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VirtualCommitCaptureResult {
    pub(crate) worktree_root: PathBuf,
    pub(crate) baseline_commit: GitCommitId,
    pub(crate) captured_tree: String,
    pub(crate) captured_changes: bool,
    pub(crate) virtual_commit: Option<GitCommitId>,
}

impl VirtualCommitCaptureResult {
    pub(crate) fn captured_commit(&self) -> &GitCommitId {
        self.virtual_commit
            .as_ref()
            .unwrap_or(&self.baseline_commit)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PhysicalWorktreeAttachment {
    Detached,
    ExistingBranch { branch_ref: String },
    NewBranch { branch_name: String },
}

impl PhysicalWorktreeAttachment {
    pub(crate) fn existing_branch(
        branch_ref: impl Into<String>,
    ) -> Result<Self, WorktreeApplicationError> {
        let branch_ref = branch_ref.into();
        if !valid_full_branch_ref(&branch_ref) {
            return Err(invalid_request("The existing branch ref is invalid."));
        }
        Ok(Self::ExistingBranch { branch_ref })
    }

    pub(crate) fn new_branch(
        branch_name: impl Into<String>,
    ) -> Result<Self, WorktreeApplicationError> {
        let branch_name = branch_name.into();
        if !valid_new_branch_name(&branch_name) {
            return Err(invalid_request("The new branch name is invalid."));
        }
        Ok(Self::NewBranch { branch_name })
    }

    pub(crate) fn expected_head_ref(&self) -> Option<String> {
        match self {
            Self::Detached => None,
            Self::ExistingBranch { branch_ref } => Some(branch_ref.clone()),
            Self::NewBranch { branch_name } => Some(format!("refs/heads/{branch_name}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PhysicalWorktreeCheckoutRequest {
    pub(crate) repository_root: PathBuf,
    pub(crate) worktree_root: PathBuf,
    pub(crate) commit_id: GitCommitId,
    pub(crate) attachment: PhysicalWorktreeAttachment,
}

impl PhysicalWorktreeCheckoutRequest {
    pub(crate) fn new(
        repository_root: PathBuf,
        worktree_root: PathBuf,
        commit_id: GitCommitId,
        attachment: PhysicalWorktreeAttachment,
    ) -> Result<Self, WorktreeApplicationError> {
        require_absolute(
            &repository_root,
            "The repository root must be an absolute path.",
        )?;
        require_absolute(
            &worktree_root,
            "The checkout root must be an absolute path.",
        )?;
        if repository_root == worktree_root || worktree_root.starts_with(&repository_root) {
            return Err(invalid_request(
                "The checkout must be outside the source repository worktree.",
            ));
        }
        Ok(Self {
            repository_root,
            worktree_root,
            commit_id,
            attachment,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PhysicalWorktreeCheckoutResult {
    pub(crate) repository_root: PathBuf,
    pub(crate) worktree_root: PathBuf,
    pub(crate) commit_id: GitCommitId,
    pub(crate) head_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PhysicalWorktreeDependencyPolicy {
    /// Compile with dependencies already present in a caller-owned live checkout.
    UseExisting,
    /// Prepare dependencies for a Worktree Review-owned checkout using the shared cache.
    Install { cache_root: PathBuf },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PhysicalWorktreeBuildRequest {
    pub(crate) worktree_root: PathBuf,
    pub(crate) attempt_root: PathBuf,
    pub(crate) dependency_policy: PhysicalWorktreeDependencyPolicy,
    pub(crate) cargo_binary_name: String,
}

impl PhysicalWorktreeBuildRequest {
    pub(crate) fn new(
        worktree_root: PathBuf,
        attempt_root: PathBuf,
        dependency_policy: PhysicalWorktreeDependencyPolicy,
        cargo_binary_name: impl Into<String>,
    ) -> Result<Self, WorktreeApplicationError> {
        let cargo_binary_name = cargo_binary_name.into();
        require_absolute(
            &worktree_root,
            "The physical worktree root must be absolute.",
        )?;
        require_absolute(
            &attempt_root,
            "The caller-owned build attempt root must be absolute.",
        )?;
        if let PhysicalWorktreeDependencyPolicy::Install { cache_root } = &dependency_policy {
            require_absolute(
                cache_root,
                "The caller-owned dependency cache root must be absolute.",
            )?;
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
            attempt_root,
            dependency_policy,
            cargo_binary_name,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PhysicalWorktreeBuildResult {
    pub(crate) worktree_root: PathBuf,
    pub(crate) attempt_root: PathBuf,
    pub(crate) output_root: PathBuf,
    pub(crate) log_path: PathBuf,
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
    GitUnavailable,
    SourceChanged,
    CheckoutConflict,
    CheckoutUnavailable,
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

fn require_absolute(
    path: &std::path::Path,
    message: &'static str,
) -> Result<(), WorktreeApplicationError> {
    if path.is_absolute()
        && !path.components().any(|component| {
            matches!(
                component,
                std::path::Component::CurDir | std::path::Component::ParentDir
            )
        })
    {
        Ok(())
    } else {
        Err(invalid_request(message))
    }
}

fn invalid_request(message: &'static str) -> WorktreeApplicationError {
    WorktreeApplicationError::new(WorktreeApplicationErrorKind::InvalidRequest, message)
}

fn valid_full_branch_ref(value: &str) -> bool {
    value.starts_with("refs/heads/")
        && value.len() > "refs/heads/".len()
        && value.len() <= MAX_GIT_REF_UNITS
        && valid_ref_tail(&value["refs/heads/".len()..])
}

fn valid_new_branch_name(value: &str) -> bool {
    !value.starts_with("refs/") && value.len() <= MAX_GIT_REF_UNITS && valid_ref_tail(value)
}

fn valid_ref_tail(value: &str) -> bool {
    !value.is_empty()
        && !value.starts_with('-')
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.ends_with('.')
        && !value.ends_with(".lock")
        && !value.contains("..")
        && !value.contains("@{")
        && !value.contains("//")
        && !value.bytes().any(|byte| {
            byte.is_ascii_control()
                || byte == b' '
                || matches!(byte, b'~' | b'^' | b':' | b'?' | b'*' | b'[' | b'\\')
        })
        && value.split('/').all(|part| !part.is_empty() && part != ".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_name_is_a_file_name_not_an_output_path() {
        let root = std::env::temp_dir().join("worktree-application-request");
        assert!(PhysicalWorktreeBuildRequest::new(
            root.join("worktree"),
            root.join("output"),
            PhysicalWorktreeDependencyPolicy::Install {
                cache_root: root.join("cache")
            },
            "codex-orchestrator"
        )
        .is_ok());
        for invalid in ["", ".", "..", "target/codex-orchestrator"] {
            assert_eq!(
                PhysicalWorktreeBuildRequest::new(
                    root.join("worktree"),
                    root.join("output"),
                    PhysicalWorktreeDependencyPolicy::UseExisting,
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
                PhysicalWorktreeDependencyPolicy::UseExisting,
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
