use super::{RepositoryContextError, RepositoryContextErrorKind};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ObjectId(String);

impl ObjectId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, RepositoryContextError> {
        let value = value.into().to_ascii_lowercase();
        if (value.len() == 40 || value.len() == 64)
            && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            Ok(Self(value))
        } else {
            Err(invalid_identity())
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct FullRefName(String);

impl FullRefName {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, RepositoryContextError> {
        let value = value.into();
        if value.starts_with("refs/")
            && value.len() <= 512
            && !value.contains("..")
            && !value.contains("@{")
            && !value.contains([' ', '~', '^', ':', '?', '*', '[', '\\'])
            && !value.ends_with(['/', '.'])
            && !value.chars().any(char::is_control)
        {
            Ok(Self(value))
        } else {
            Err(invalid_identity())
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn short_branch(&self) -> Option<&str> {
        self.0.strip_prefix("refs/heads/")
    }
}

fn invalid_identity() -> RepositoryContextError {
    RepositoryContextError::new(
        RepositoryContextErrorKind::InvalidGitOutput,
        "Git returned an invalid identity.",
    )
}
