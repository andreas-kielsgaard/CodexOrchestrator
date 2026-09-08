use super::{
    domain::{RepositoryDisclosureKind, RepositoryId, ReviewRepository},
    state::{CapabilityReadinessStatus, WorktreeReviewApplication},
    storage::ReviewRepositoryRepository,
};
use crate::{
    repository_context::{PathIdentity, RemoteObservation, RepositoryContext, RepositoryIdentity},
    repository_discovery::{
        CodexDirectoryDiscovery, GitHubCatalog, GitHubConnectionState, GitHubRemoteRepository,
    },
};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryRegistrationOverviewView {
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

pub(crate) struct RepositoryRegistrationService {
    application: Arc<WorktreeReviewApplication>,
    codex: CodexDirectoryDiscovery,
    github: GitHubCatalog,
}

impl RepositoryRegistrationService {
    pub(crate) fn new(application: Arc<WorktreeReviewApplication>) -> Self {
        Self {
            application,
            codex: CodexDirectoryDiscovery,
            github: GitHubCatalog,
        }
    }

    pub(crate) fn overview(&self) -> Result<RepositoryRegistrationOverviewView, String> {
        let context = self
            .application
            .repository_context()
            .map_err(|error| error.message)?;
        let registered = self
            .application
            .registered_repositories()
            .map_err(|error| error.message)?;
        let mut local = BTreeMap::<String, LocalRepository>::new();
        for repository in registered {
            let disclosures = self.persisted_disclosures(&repository.id)?;
            local.insert(
                repository.id.as_str().to_owned(),
                LocalRepository::registered(repository, disclosures),
            );
        }

        let (codex, candidates) = match self.codex.list() {
            Ok(candidates) => (
                DiscoveryStatusView {
                    state: "ready".into(),
                    message: "Directories from recent Codex tasks are available.".into(),
                },
                candidates,
            ),
            Err(message) => (
                DiscoveryStatusView {
                    state: "unavailable".into(),
                    message,
                },
                Vec::new(),
            ),
        };
        for candidate in codex_git_candidates(candidates.into_iter().map(|value| value.path)) {
            let Ok(identity) = context.identities().inspect(&candidate) else {
                continue;
            };
            let entry = local
                .entry(identity.id.as_str().to_owned())
                .or_insert_with(|| LocalRepository::candidate(&identity));
            entry.add_disclosure("codex_task");
        }

        for repository in local.values_mut() {
            repository.github_key = github_key_for_local(&context, &repository.anchor_root);
        }

        let connection = self.github.connection();
        let (github_state_value, github_message, remote_repositories) =
            if connection.state == GitHubConnectionState::Connected {
                match self.github.repositories() {
                    Ok(repositories) => (connection.state, connection.message, repositories),
                    Err(message) => (GitHubConnectionState::Unavailable, message, Vec::new()),
                }
            } else {
                (connection.state, connection.message, Vec::new())
            };
        let github = GitHubConnectionView {
            state: github_state(github_state_value).into(),
            login: connection.login,
            message: github_message,
        };
        let mut repositories = Vec::new();
        for remote in remote_repositories {
            let key = remote.name_with_owner.to_ascii_lowercase();
            let instances = take_matching_local(&mut local, &key);
            repositories.push(registration_repository(&remote, instances));
        }
        repositories.extend(local.into_values().map(|repository| {
            let name = repository
                .github_key
                .clone()
                .unwrap_or_else(|| repository.name.clone());
            RegistrationRepositoryView {
                catalog_id: repository.repository_id.clone(),
                name,
                github: None,
                local_instances: vec![repository.view()],
            }
        }));
        repositories.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        });
        Ok(RepositoryRegistrationOverviewView {
            codex,
            github,
            repositories,
        })
    }

    pub(crate) fn register_directory(&self, path: PathBuf) -> Result<(), String> {
        let result = self
            .application
            .register_repository(path, RepositoryDisclosureKind::ManualDirectory);
        selection_result(result.selection.status, result.selection.message)
    }

    pub(crate) fn register_codex_repository(&self, repository_id: &str) -> Result<(), String> {
        let context = self
            .application
            .repository_context()
            .map_err(|error| error.message)?;
        let candidates = codex_git_candidates(
            self.codex
                .list()?
                .into_iter()
                .map(|candidate| candidate.path)
                .collect::<Vec<_>>(),
        );
        let path = candidates
            .into_iter()
            .find_map(|candidate| {
                context
                    .identities()
                    .inspect(&candidate)
                    .ok()
                    .filter(|identity| identity.id.as_str() == repository_id)
                    .map(|_| candidate)
            })
            .ok_or_else(|| "That repository is no longer exposed by a Codex task.".to_string())?;
        let result = self
            .application
            .register_repository(path, RepositoryDisclosureKind::CodexTask);
        selection_result(result.selection.status, result.selection.message)
    }

    fn persisted_disclosures(&self, id: &RepositoryId) -> Result<Vec<String>, String> {
        self.application
            .database()
            .map_err(|error| error.message)?
            .repositories()
            .list_disclosures(id)
            .map_err(|error| error.to_string())
            .map(|values| {
                values
                    .into_iter()
                    .map(|disclosure| disclosure.kind.as_str().to_owned())
                    .collect()
            })
    }
}

fn codex_git_candidates(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut candidates = BTreeMap::<String, PathBuf>::new();
    for path in paths {
        let Some(root) = nearest_git_root(path) else {
            continue;
        };
        candidates
            .entry(PathIdentity::of(&root).as_str().to_owned())
            .or_insert(root);
    }
    candidates.into_values().collect()
}

fn nearest_git_root(path: PathBuf) -> Option<PathBuf> {
    let mut candidate = if path.is_file() {
        path.parent()?.to_path_buf()
    } else {
        path
    };
    loop {
        if candidate.join(".git").exists() {
            return std::fs::canonicalize(&candidate).ok();
        }
        if !candidate.pop() {
            return None;
        }
    }
}

struct LocalRepository {
    repository_id: String,
    name: String,
    anchor_root: PathBuf,
    registered: bool,
    disclosures: Vec<String>,
    github_key: Option<String>,
}

impl LocalRepository {
    fn registered(repository: ReviewRepository, disclosures: Vec<String>) -> Self {
        Self {
            repository_id: repository.id.as_str().to_owned(),
            name: repository.label,
            anchor_root: repository.anchor_root,
            registered: true,
            disclosures,
            github_key: None,
        }
    }

    fn candidate(identity: &RepositoryIdentity) -> Self {
        Self {
            repository_id: identity.id.as_str().to_owned(),
            name: path_name(identity.top_level.path()),
            anchor_root: identity.top_level.path().to_path_buf(),
            registered: false,
            disclosures: Vec::new(),
            github_key: None,
        }
    }

    fn add_disclosure(&mut self, value: &str) {
        if !self.disclosures.iter().any(|existing| existing == value) {
            self.disclosures.push(value.to_owned());
        }
    }

    fn view(self) -> LocalRepositoryInstanceView {
        LocalRepositoryInstanceView {
            repository_id: self.repository_id,
            name: self.name,
            location_label: self.anchor_root.to_string_lossy().into_owned(),
            registered: self.registered,
            disclosures: self.disclosures,
        }
    }
}

fn take_matching_local(
    local: &mut BTreeMap<String, LocalRepository>,
    github_key: &str,
) -> Vec<LocalRepositoryInstanceView> {
    let matches = local
        .iter()
        .filter_map(|(id, repository)| {
            (repository.github_key.as_deref() == Some(github_key)).then(|| id.clone())
        })
        .collect::<Vec<_>>();
    matches
        .into_iter()
        .filter_map(|id| local.remove(&id))
        .map(LocalRepository::view)
        .collect()
}

fn registration_repository(
    remote: &GitHubRemoteRepository,
    local_instances: Vec<LocalRepositoryInstanceView>,
) -> RegistrationRepositoryView {
    RegistrationRepositoryView {
        catalog_id: remote.id.clone(),
        name: remote.name_with_owner.clone(),
        github: Some(GitHubRepositoryView {
            repository_id: remote.id.clone(),
            name_with_owner: remote.name_with_owner.clone(),
            visibility: if remote.private { "private" } else { "public" }.into(),
            web_url: remote.web_url.clone(),
        }),
        local_instances,
    }
}

fn github_key_for_local(context: &RepositoryContext, root: &std::path::Path) -> Option<String> {
    context
        .remotes()
        .list(root)
        .ok()?
        .iter()
        .find_map(github_key_for_remote)
}

fn github_key_for_remote(remote: &RemoteObservation) -> Option<String> {
    remote
        .fetch_url
        .as_deref()
        .and_then(github_key)
        .or_else(|| remote.push_url.as_deref().and_then(github_key))
}

fn github_key(url: &str) -> Option<String> {
    let value = url.trim().trim_end_matches('/').trim_end_matches(".git");
    let path = if let Some(path) = value.strip_prefix("git@github.com:") {
        path
    } else if let Some(path) = value.strip_prefix("ssh://git@github.com/") {
        path
    } else if let Some(path) = value.strip_prefix("https://github.com/") {
        path
    } else if let Some(path) = value.strip_prefix("http://github.com/") {
        path
    } else {
        return None;
    };
    let mut segments = path.split('/');
    let owner = segments.next()?;
    let repository = segments.next()?;
    if owner.is_empty() || repository.is_empty() || segments.next().is_some() {
        return None;
    }
    Some(format!("{owner}/{repository}").to_ascii_lowercase())
}

fn path_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Repository")
        .to_owned()
}

fn github_state(state: GitHubConnectionState) -> &'static str {
    match state {
        GitHubConnectionState::Connected => "connected",
        GitHubConnectionState::CliUnavailable => "cli_unavailable",
        GitHubConnectionState::NotAuthenticated => "not_authenticated",
        GitHubConnectionState::Unavailable => "unavailable",
    }
}

fn selection_result(status: CapabilityReadinessStatus, message: String) -> Result<(), String> {
    (status == CapabilityReadinessStatus::Ready)
        .then_some(())
        .ok_or(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_remote_forms_share_one_catalog_key() {
        for url in [
            "git@github.com:OpenAI/Codex.git",
            "ssh://git@github.com/OpenAI/Codex.git",
            "https://github.com/OpenAI/Codex.git",
        ] {
            assert_eq!(github_key(url).as_deref(), Some("openai/codex"));
        }
    }
}
