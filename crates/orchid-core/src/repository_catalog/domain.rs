use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisteredRepository {
    pub repository_id: String,
    pub label: String,
    pub anchor_root: PathBuf,
    pub git_common_directory: PathBuf,
    pub first_registered_at: DateTime<Utc>,
    pub last_verified_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RepositoryDisclosureKind {
    ManualDirectory,
    CodexTask,
}

impl RepositoryDisclosureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ManualDirectory => "manual_directory",
            Self::CodexTask => "codex_task",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "manual_directory" => Some(Self::ManualDirectory),
            "codex_task" => Some(Self::CodexTask),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepositoryDisclosure {
    pub repository_id: String,
    pub kind: RepositoryDisclosureKind,
    pub observed_path: PathBuf,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredRepositoryView {
    pub repository_id: String,
    pub name: String,
    pub location_label: String,
}

impl From<&RegisteredRepository> for RegisteredRepositoryView {
    fn from(repository: &RegisteredRepository) -> Self {
        Self {
            repository_id: repository.repository_id.clone(),
            name: repository.label.clone(),
            location_label: repository.anchor_root.to_string_lossy().into_owned(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryCatalogOverviewView {
    pub codex: DiscoveryStatusView,
    pub github: GitHubConnectionView,
    pub repositories: Vec<RegistrationRepositoryView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryStatusView {
    pub state: String,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubConnectionView {
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub login: Option<String>,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationRepositoryView {
    pub catalog_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github: Option<GitHubRepositoryView>,
    pub local_instances: Vec<LocalRepositoryInstanceView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitHubRepositoryView {
    pub repository_id: String,
    pub name_with_owner: String,
    pub visibility: String,
    pub web_url: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalRepositoryInstanceView {
    pub repository_id: String,
    pub name: String,
    pub location_label: String,
    pub registered: bool,
    pub disclosures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRepoBranchWorktreeTarget {
    pub repository: ResolvedRepositoryTarget,
    pub branch: ResolvedBranchTarget,
    pub worktree: ResolvedWorktreeTarget,
}

impl ResolvedRepoBranchWorktreeTarget {
    pub fn validate(&self) -> Result<(), String> {
        for (value, label) in [
            (&self.repository.id, "repository ID"),
            (&self.repository.name, "repository name"),
            (
                &self.repository.git_common_directory,
                "repository Git identity",
            ),
            (&self.branch.id, "branch ID"),
            (&self.branch.name, "branch name"),
            (&self.worktree.id, "worktree ID"),
            (&self.worktree.path, "worktree path"),
        ] {
            if value.trim().is_empty() {
                return Err(format!(
                    "A repository, branch, and worktree target {label} is required."
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRepositoryTarget {
    pub id: String,
    pub name: String,
    pub git_common_directory: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedBranchTarget {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedWorktreeTarget {
    pub id: String,
    pub path: String,
}
