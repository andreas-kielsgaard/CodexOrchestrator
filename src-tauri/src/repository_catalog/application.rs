use super::{
    domain::{
        DiscoveryStatusView, GitHubConnectionView, GitHubRepositoryView,
        LocalRepositoryInstanceView, RegisteredRepository, RegisteredRepositoryView,
        RegistrationRepositoryView, RepositoryCatalogOverviewView, RepositoryDisclosure,
        RepositoryDisclosureKind, ResolvedBranchTarget, ResolvedRepoBranchWorktreeTarget,
        ResolvedRepositoryTarget, ResolvedWorktreeTarget,
    },
    repository::SqliteRepositoryCatalogRepository,
};
use crate::{
    persistence::ActiveDatabase,
    repository_context::{
        PathIdentity, RemoteObservation, RepositoryContext, RepositoryIdentity, WorktreeLocation,
    },
    repository_discovery::{
        CodexDirectoryDiscovery, GitHubCatalog, GitHubConnectionState, GitHubRemoteRepository,
    },
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

pub(crate) struct RepositoryCatalog {
    storage: SqliteRepositoryCatalogRepository,
    context: Result<RepositoryContext, String>,
    codex: CodexDirectoryDiscovery,
    github: GitHubCatalog,
}

impl RepositoryCatalog {
    pub(crate) fn new(database: Arc<ActiveDatabase>) -> Self {
        Self {
            storage: SqliteRepositoryCatalogRepository::new(database),
            context: RepositoryContext::discover().map_err(|error| error.to_string()),
            codex: CodexDirectoryDiscovery,
            github: GitHubCatalog,
        }
    }

    pub(crate) fn repository_context(&self) -> Result<RepositoryContext, String> {
        self.context.clone()
    }

    pub(crate) fn list_registered(&self) -> Result<Vec<RegisteredRepository>, String> {
        self.storage.list()
    }

    pub(crate) fn resolve_verified(
        &self,
        repository_id: &str,
    ) -> Result<(RegisteredRepository, RepositoryIdentity), String> {
        let record = self
            .storage
            .find(repository_id)?
            .ok_or_else(|| "The repository is not registered.".to_string())?;
        let identity = self
            .repository_context()?
            .identities()
            .inspect(&record.anchor_root)
            .map_err(|error| error.to_string())?;
        if identity.id.as_str() != record.repository_id
            || identity.common_directory.identity()
                != &PathIdentity::of(&record.git_common_directory)
        {
            return Err(
                "The registered repository identity no longer matches this location.".into(),
            );
        }
        Ok((record, identity))
    }

    pub(crate) fn resolve_main_working_tree(&self, repository_id: &str) -> Result<PathBuf, String> {
        let (_, identity) = self.resolve_verified(repository_id)?;
        self.repository_context()?
            .worktrees()
            .main_working_tree(&identity.id, identity.top_level.path())
            .map(|directory| directory.path().to_path_buf())
            .map_err(|error| error.to_string())
    }

    pub(crate) fn register_directory(
        &self,
        candidate: PathBuf,
    ) -> Result<RegisteredRepositoryView, String> {
        self.register(candidate, RepositoryDisclosureKind::ManualDirectory)
            .map(|repository| RegisteredRepositoryView::from(&repository))
    }

    pub(crate) fn register_codex_repository(
        &self,
        repository_id: &str,
    ) -> Result<RegisteredRepositoryView, String> {
        let context = self.repository_context()?;
        let candidates = codex_git_candidates(
            self.codex
                .list()?
                .into_iter()
                .map(|candidate| candidate.path),
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
        self.register(path, RepositoryDisclosureKind::CodexTask)
            .map(|repository| RegisteredRepositoryView::from(&repository))
    }

    pub(crate) fn overview(&self) -> Result<RepositoryCatalogOverviewView, String> {
        let context = self.repository_context()?;
        let registered = self.storage.list()?;
        let mut local = BTreeMap::<String, LocalRepository>::new();
        for repository in registered {
            let disclosures = self
                .storage
                .list_disclosures(&repository.repository_id)?
                .into_iter()
                .map(|disclosure| disclosure.kind.as_str().to_owned())
                .collect();
            local.insert(
                repository.repository_id.clone(),
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
            repositories.push(registration_repository(
                &remote,
                take_matching_local(&mut local, &key),
            ));
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
        Ok(RepositoryCatalogOverviewView {
            codex,
            github,
            repositories,
        })
    }

    pub(crate) fn list_worktree_targets(
        &self,
    ) -> Result<Vec<ResolvedRepoBranchWorktreeTarget>, String> {
        let context = self.repository_context()?;
        let repositories = self.storage.list()?;
        let mut targets = Vec::new();
        for record in repositories {
            let Ok((record, identity)) = self.resolve_verified(&record.repository_id) else {
                continue;
            };
            let Ok(worktrees) = context
                .worktrees()
                .list(&identity.id, identity.top_level.path())
            else {
                continue;
            };
            for worktree in worktrees {
                let (WorktreeLocation::Available(location), Some(branch_ref)) =
                    (worktree.location, worktree.head_ref)
                else {
                    continue;
                };
                let Some(branch_name) = branch_ref.branch_name() else {
                    continue;
                };
                targets.push(ResolvedRepoBranchWorktreeTarget {
                    repository: ResolvedRepositoryTarget {
                        id: record.repository_id.clone(),
                        name: record.label.clone(),
                        git_common_directory: identity
                            .common_directory
                            .path()
                            .to_string_lossy()
                            .into_owned(),
                    },
                    branch: ResolvedBranchTarget {
                        id: stable_id(
                            "branch",
                            &format!("{}\0{}", record.repository_id, branch_ref.as_str()),
                        ),
                        name: branch_name.to_owned(),
                    },
                    worktree: ResolvedWorktreeTarget {
                        id: worktree.id.as_str().to_owned(),
                        path: location.path().to_string_lossy().into_owned(),
                    },
                });
            }
        }
        targets.sort_by(|left, right| {
            target_sort_key(left)
                .cmp(&target_sort_key(right))
                .then_with(|| left.worktree.id.cmp(&right.worktree.id))
        });
        Ok(targets)
    }

    fn register(
        &self,
        candidate: PathBuf,
        kind: RepositoryDisclosureKind,
    ) -> Result<RegisteredRepository, String> {
        let context = self.repository_context()?;
        let identity = context
            .identities()
            .inspect(&candidate)
            .map_err(|error| error.to_string())?;
        let anchor_root = context
            .worktrees()
            .list(&identity.id, identity.top_level.path())
            .ok()
            .and_then(|worktrees| {
                worktrees
                    .into_iter()
                    .find_map(|worktree| match worktree.location {
                        WorktreeLocation::Available(path) => Some(path.path().to_path_buf()),
                        WorktreeLocation::Unavailable(_) => None,
                    })
            })
            .unwrap_or_else(|| identity.top_level.path().to_path_buf());
        let now = Utc::now();
        let existing = self.storage.find(identity.id.as_str())?;
        let repository = RegisteredRepository {
            repository_id: identity.id.as_str().to_owned(),
            label: path_name(&anchor_root),
            anchor_root,
            git_common_directory: identity.common_directory.path().to_path_buf(),
            first_registered_at: existing
                .as_ref()
                .map(|repository| repository.first_registered_at)
                .unwrap_or(now),
            last_verified_at: now,
        };
        let disclosure = RepositoryDisclosure {
            repository_id: repository.repository_id.clone(),
            kind,
            observed_path: std::fs::canonicalize(&candidate).unwrap_or(candidate),
            first_seen_at: now,
            last_seen_at: now,
        };
        self.storage.save_registration(&repository, &disclosure)?;
        Ok(repository)
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
    fn registered(repository: RegisteredRepository, disclosures: Vec<String>) -> Self {
        Self {
            repository_id: repository.repository_id,
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

fn stable_id(kind: &str, value: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(b"codex-orchestrator/repository-catalog/v1");
    hash.update(value.as_bytes());
    format!("{kind}-{}", &format!("{:x}", hash.finalize())[..24])
}

fn target_sort_key(target: &ResolvedRepoBranchWorktreeTarget) -> (String, String, String) {
    (
        target.repository.name.to_lowercase(),
        target.branch.name.to_lowercase(),
        PathIdentity::of(std::path::Path::new(&target.worktree.path))
            .as_str()
            .to_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{persistence::ActiveDatabase, repository_catalog::REPOSITORY_CATALOG_SCHEMA};
    use rusqlite::Connection;
    use std::{fs, path::Path, process::Command};

    fn catalog() -> RepositoryCatalog {
        let database =
            ActiveDatabase::from_connection(Connection::open_in_memory().unwrap(), |connection| {
                connection
                    .execute_batch(REPOSITORY_CATALOG_SCHEMA)
                    .map_err(|error| error.to_string())
            })
            .unwrap();
        RepositoryCatalog::new(Arc::new(database))
    }

    fn run_git(root: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn create_repository(root: &Path) {
        fs::create_dir_all(root).unwrap();
        run_git(root, &["init", "-b", "main"]);
        run_git(root, &["config", "user.name", "Codex Test"]);
        run_git(
            root,
            &["config", "user.email", "codex-test@example.invalid"],
        );
        run_git(root, &["commit", "--allow-empty", "-m", "Initial commit"]);
    }

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

    #[test]
    fn target_listing_skips_unavailable_registrations_without_hiding_available_worktrees() {
        if Command::new("git").arg("--version").output().is_err() {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        let available = directory.path().join("available");
        let unavailable = directory.path().join("unavailable");
        create_repository(&available);
        create_repository(&unavailable);
        let catalog = catalog();
        let available_registration = catalog.register_directory(available.clone()).unwrap();
        catalog.register_directory(unavailable.clone()).unwrap();
        fs::remove_dir_all(&unavailable).unwrap();

        let targets = catalog.list_worktree_targets().unwrap();

        assert_eq!(targets.len(), 1);
        assert_eq!(
            targets[0].repository.id,
            available_registration.repository_id
        );
        assert_eq!(targets[0].branch.name, "main");
        assert_eq!(
            PathIdentity::of(PathBuf::from(&targets[0].worktree.path).as_path()),
            PathIdentity::of(available.canonicalize().unwrap().as_path())
        );
    }
    #[test]
    fn main_tree_resolution_from_linked_registration_ignores_branch_name_and_target_sorting() {
        let directory = tempfile::tempdir().unwrap();
        let main = directory.path().join("z-original");
        let linked = directory.path().join("a-linked");
        create_repository(&main);
        run_git(&main, &["branch", "-m", "trunk"]);
        run_git(
            &main,
            &["worktree", "add", "-b", "main", linked.to_str().unwrap()],
        );
        let catalog = catalog();
        let registration = catalog.register_directory(linked).unwrap();
        let resolved = catalog
            .resolve_main_working_tree(&registration.repository_id)
            .unwrap();
        assert_eq!(PathIdentity::of(&resolved), PathIdentity::of(&main));
    }
}
