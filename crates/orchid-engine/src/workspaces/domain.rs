use std::{error::Error, fmt, path::PathBuf};
const MAX_GIT_ID_UNITS: usize = 128;
const MAX_GIT_REF_UNITS: usize = 512;
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GitCommitId(String);

impl GitCommitId {
    pub fn new(value: impl Into<String>) -> Result<Self, CheckoutError> {
        let value = value.into();
        if !matches!(value.len(), 40 | 64)
            || value.len() > MAX_GIT_ID_UNITS
            || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(invalid_request("The exact Git commit identity is invalid."));
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PhysicalWorktreeAttachment {
    Detached,
    ExistingBranch { branch_ref: String },
    NewBranch { branch_name: String },
}

impl PhysicalWorktreeAttachment {
    pub fn existing_branch(branch_ref: impl Into<String>) -> Result<Self, CheckoutError> {
        let branch_ref = branch_ref.into();
        if !valid_full_branch_ref(&branch_ref) {
            return Err(invalid_request("The existing branch ref is invalid."));
        }
        Ok(Self::ExistingBranch { branch_ref })
    }

    pub fn new_branch(branch_name: impl Into<String>) -> Result<Self, CheckoutError> {
        let branch_name = branch_name.into();
        if !valid_new_branch_name(&branch_name) {
            return Err(invalid_request("The new branch name is invalid."));
        }
        Ok(Self::NewBranch { branch_name })
    }

    pub fn expected_head_ref(&self) -> Option<String> {
        match self {
            Self::Detached => None,
            Self::ExistingBranch { branch_ref } => Some(branch_ref.clone()),
            Self::NewBranch { branch_name } => Some(format!("refs/heads/{branch_name}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalWorktreeCheckoutRequest {
    pub repository_root: PathBuf,
    pub worktree_root: PathBuf,
    pub commit_id: GitCommitId,
    pub attachment: PhysicalWorktreeAttachment,
}

impl PhysicalWorktreeCheckoutRequest {
    pub fn new(
        repository_root: PathBuf,
        worktree_root: PathBuf,
        commit_id: GitCommitId,
        attachment: PhysicalWorktreeAttachment,
    ) -> Result<Self, CheckoutError> {
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
pub struct PhysicalWorktreeCheckoutResult {
    pub repository_root: PathBuf,
    pub worktree_root: PathBuf,
    pub commit_id: GitCommitId,
    pub head_ref: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CheckoutErrorKind {
    InvalidRequest,
    GitUnavailable,
    CheckoutConflict,
    CheckoutUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckoutError {
    pub kind: CheckoutErrorKind,
    pub message: String,
}

impl CheckoutError {
    pub fn new(kind: CheckoutErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for CheckoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CheckoutError {}

fn require_absolute(path: &std::path::Path, message: &'static str) -> Result<(), CheckoutError> {
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

fn invalid_request(message: &'static str) -> CheckoutError {
    CheckoutError::new(CheckoutErrorKind::InvalidRequest, message)
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
