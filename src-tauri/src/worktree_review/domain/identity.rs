use serde::{de::Error as _, Deserialize, Deserializer, Serialize};
use std::{error::Error, fmt};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DomainError {
    message: String,
}

impl DomainError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for DomainError {}

fn validated_text(kind: &str, value: impl Into<String>) -> Result<String, DomainError> {
    let value = value.into();
    if value.trim().is_empty() {
        return Err(DomainError::new(format!("{kind} must not be empty")));
    }
    if value != value.trim() {
        return Err(DomainError::new(format!(
            "{kind} must not have leading or trailing whitespace"
        )));
    }
    Ok(value)
}

macro_rules! opaque_id {
    ($name:ident, $kind:literal, $prefix:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn new(value: impl Into<String>) -> Result<Self, DomainError> {
                Ok(Self(validated_text($kind, value)?))
            }

            pub(crate) fn random() -> Self {
                Self(format!("{}-{}", $prefix, Uuid::new_v4()))
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(D::Error::custom)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

opaque_id!(RepositoryId, "repository ID", "repository");
opaque_id!(WorktreeId, "worktree ID", "worktree");
opaque_id!(
    WorktreeAssociationId,
    "worktree association ID",
    "association"
);
opaque_id!(WorkspaceId, "workspace ID", "workspace");
opaque_id!(ReviewBuildId, "review build ID", "build");
opaque_id!(OperationAttemptId, "operation attempt ID", "attempt");
opaque_id!(BuildOutputId, "build output ID", "output");
opaque_id!(BuildAttentionId, "build attention ID", "attention");
opaque_id!(CleanupJobId, "cleanup job ID", "cleanup");
opaque_id!(CleanupResourceId, "cleanup resource ID", "resource");

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct BranchRef(String);

impl BranchRef {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = validated_text("branch ref", value)?;
        if !value.starts_with("refs/heads/") || value == "refs/heads/" {
            return Err(DomainError::new(
                "branch ref must be a full local branch ref under refs/heads/",
            ));
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for BranchRef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

impl fmt::Display for BranchRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct GitObjectId(String);

impl GitObjectId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = validated_text("Git object ID", value)?.to_ascii_lowercase();
        if (value.len() != 40 && value.len() != 64)
            || !value.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(DomainError::new(
                "Git object ID must be a full 40- or 64-character hexadecimal value",
            ));
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for GitObjectId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

macro_rules! validated_value {
    ($name:ident, $kind:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
        #[serde(transparent)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn new(value: impl Into<String>) -> Result<Self, DomainError> {
                Ok(Self(validated_text($kind, value)?))
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                Self::new(String::deserialize(deserializer)?).map_err(D::Error::custom)
            }
        }
    };
}

validated_value!(RetentionKey, "retention key");
validated_value!(ReviewBuildName, "review build name");
validated_value!(WorktreeLocation, "worktree location");
validated_value!(BuildOutputStorageKey, "build output storage key");
validated_value!(
    ExecutableRelativePath,
    "build output executable relative path"
);
validated_value!(CleanupStorageKey, "cleanup storage key");
validated_value!(ContainmentRoot, "cleanup containment root");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branch_refs_are_full_local_refs() {
        assert!(BranchRef::new("refs/heads/feature").is_ok());
        assert!(BranchRef::new("feature").is_err());
        assert!(BranchRef::new("refs/tags/feature").is_err());
    }

    #[test]
    fn git_object_ids_are_canonical_full_values() {
        let uppercase = "A".repeat(40);
        assert_eq!(
            GitObjectId::new(uppercase).unwrap().as_str(),
            "a".repeat(40)
        );
        assert!(GitObjectId::new("abc123").is_err());
    }
}
