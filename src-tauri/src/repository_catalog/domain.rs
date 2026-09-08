use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RegisteredRepository {
    pub(crate) repository_id: String,
    pub(crate) label: String,
    pub(crate) anchor_root: PathBuf,
    pub(crate) git_common_directory: PathBuf,
    pub(crate) first_registered_at: DateTime<Utc>,
    pub(crate) last_verified_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RepositoryDisclosureKind {
    ManualDirectory,
    CodexTask,
}

impl RepositoryDisclosureKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ManualDirectory => "manual_directory",
            Self::CodexTask => "codex_task",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "manual_directory" => Some(Self::ManualDirectory),
            "codex_task" => Some(Self::CodexTask),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryDisclosure {
    pub(crate) repository_id: String,
    pub(crate) kind: RepositoryDisclosureKind,
    pub(crate) observed_path: PathBuf,
    pub(crate) first_seen_at: DateTime<Utc>,
    pub(crate) last_seen_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisteredRepositoryView {
    pub(crate) repository_id: String,
    pub(crate) name: String,
    pub(crate) location_label: String,
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
pub(crate) struct RepositoryCatalogOverviewView {
    pub(crate) codex: DiscoveryStatusView,
    pub(crate) github: GitHubConnectionView,
    pub(crate) repositories: Vec<RegistrationRepositoryView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiscoveryStatusView {
    pub(crate) state: String,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitHubConnectionView {
    pub(crate) state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) login: Option<String>,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegistrationRepositoryView {
    pub(crate) catalog_id: String,
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) github: Option<GitHubRepositoryView>,
    pub(crate) local_instances: Vec<LocalRepositoryInstanceView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitHubRepositoryView {
    pub(crate) repository_id: String,
    pub(crate) name_with_owner: String,
    pub(crate) visibility: String,
    pub(crate) web_url: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocalRepositoryInstanceView {
    pub(crate) repository_id: String,
    pub(crate) name: String,
    pub(crate) location_label: String,
    pub(crate) registered: bool,
    pub(crate) disclosures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedRepoBranchWorktreeTarget {
    pub(crate) repository: ResolvedRepositoryTarget,
    pub(crate) branch: ResolvedBranchTarget,
    pub(crate) worktree: ResolvedWorktreeTarget,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedRepositoryTarget {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) git_common_directory: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedBranchTarget {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedWorktreeTarget {
    pub(crate) id: String,
    pub(crate) path: String,
}
