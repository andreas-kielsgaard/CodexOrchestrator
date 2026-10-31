//! Cached worktree facts for sidebar grouping; discovery never prepares or starts a device.
use crate::execution_targets::{domain::WorktreeChoiceScope, ExecutionTargetService};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NavigationBranch {
    pub(crate) worktrees: Vec<NavigationWorktree>,
    pub(crate) id: String,
    pub(crate) repository_id: String,
    pub(crate) branch_ref: String,
    pub(crate) created_at: Option<String>,
    pub(crate) paths: Vec<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NavigationWorktree {
    pub(crate) device_id: String,
    pub(crate) device_name: String,
    pub(crate) path: String,
    pub(crate) head: Option<String>,
    pub(crate) created_at: Option<String>,
    pub(crate) dirty: Option<bool>,
}

#[derive(Default)]
pub(crate) struct BranchInventory {
    branches: Mutex<Vec<NavigationBranch>>,
    sources: Mutex<HashMap<String, Vec<NavigationBranch>>>,
    refreshed: Mutex<Option<Instant>>,
    refreshing: AtomicBool,
}
impl BranchInventory {
    pub(crate) fn invalidate(&self) {
        *self.refreshed.lock().unwrap() = None;
    }
    pub(crate) fn cached(&self) -> Vec<NavigationBranch> {
        self.branches.lock().unwrap().clone()
    }
    pub(crate) fn refresh(
        &self,
        targets: &Arc<ExecutionTargetService>,
        catalog: &crate::repository_catalog::RepositoryCatalog,
    ) -> bool {
        if self.refreshing.swap(true, Ordering::SeqCst) {
            return false;
        }
        {
            let refreshed = self.refreshed.lock().unwrap();
            if refreshed.is_some_and(|at| at.elapsed() < Duration::from_secs(60)) {
                self.refreshing.store(false, Ordering::SeqCst);
                return false;
            }
        }
        let mut result: HashMap<String, NavigationBranch> = HashMap::new();
        let mut sources = self.sources.lock().unwrap().clone();
        let mut active_sources = vec!["local".to_string()];
        // Local worktree existence does not depend on having an agent Capability Profile.
        let local = catalog.list_worktree_targets();
        for target in local.as_ref().into_iter().flatten() {
            let branch_ref = format!("refs/heads/{}", target.branch.name);
            let id = branch_id(&target.repository.id, &branch_ref);
            let created_at =
                std::fs::metadata(std::path::Path::new(&target.worktree.path).join(".git"))
                    .and_then(|metadata| metadata.created())
                    .ok()
                    .map(|time| chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339());
            let entry = result
                .entry(id.clone())
                .or_insert_with(|| NavigationBranch {
                    id,
                    repository_id: target.repository.id.clone(),
                    branch_ref,
                    created_at: None,
                    paths: vec![],
                    worktrees: vec![],
                });
            if created_at > entry.created_at {
                entry.created_at = created_at.clone();
            }
            entry.paths.push(target.worktree.path.clone());
            entry.worktrees.push(NavigationWorktree {
                device_id: "local".into(),
                device_name: "This device".into(),
                path: target.worktree.path.clone(),
                head: None,
                created_at,
                dirty: None,
            });
        }
        if local.is_ok() {
            sources.insert("local".into(), result.into_values().collect());
        }
        let mut result: HashMap<String, NavigationBranch> = HashMap::new();
        let scopes = targets
            .devices()
            .unwrap_or_default()
            .into_iter()
            .filter(|device| device.device_id != "local")
            .map(|device| WorktreeChoiceScope::Device {
                device_id: device.device_id,
            });
        for scope in scopes {
            let WorktreeChoiceScope::Device { ref device_id } = scope else {
                continue;
            };
            let source = device_id.clone();
            active_sources.push(source.clone());
            let choices = match targets.worktree_choices(&scope) {
                Ok(choices) => choices,
                Err(_) => continue,
            };
            let mut discovered = vec![];
            let previous = sources.get(&source).cloned().unwrap_or_default();
            for repository in choices {
                let mut available = false;
                let mut observed = vec![];
                for profile in repository.profiles {
                    if profile.error.is_some() {
                        continue;
                    }
                    available = true;
                    for worktree in profile.instances {
                        let id = branch_id(&repository.repository_id, &worktree.branch_ref);
                        observed.push(NavigationBranch {
                            id,
                            repository_id: repository.repository_id.clone(),
                            branch_ref: worktree.branch_ref,
                            created_at: worktree.created_at.clone(),
                            paths: vec![worktree.path.clone()],
                            worktrees: vec![NavigationWorktree {
                                device_id: profile.profile.execution.device_id.clone(),
                                device_name: profile.profile.execution.device_name.clone(),
                                path: worktree.path.clone(),
                                head: worktree.head,
                                created_at: worktree.created_at.clone(),
                                dirty: Some(worktree.dirty),
                            }],
                        });
                    }
                }
                if available {
                    discovered.extend(observed);
                } else {
                    discovered.extend(
                        previous
                            .iter()
                            .filter(|branch| branch.repository_id == repository.repository_id)
                            .cloned(),
                    );
                }
            }
            sources.insert(source, discovered);
        }
        sources.retain(|source, _| active_sources.contains(source));
        for branch in sources.values().flatten() {
            let entry = result
                .entry(branch.id.clone())
                .or_insert_with(|| NavigationBranch {
                    paths: vec![],
                    worktrees: vec![],
                    ..branch.clone()
                });
            if branch.created_at > entry.created_at {
                entry.created_at = branch.created_at.clone();
            }
            for path in &branch.paths {
                if !entry.paths.contains(path) {
                    entry.paths.push(path.clone());
                }
            }
            for worktree in &branch.worktrees {
                if !entry
                    .worktrees
                    .iter()
                    .any(|w| w.device_id == worktree.device_id && w.path == worktree.path)
                {
                    entry.worktrees.push(worktree.clone());
                }
            }
        }
        *self.sources.lock().unwrap() = sources;
        let mut cache = self.branches.lock().unwrap();
        // Failed remote discoveries retain only that device's last-known worktrees.
        *cache = result.into_values().collect();
        cache.sort_by(|a, b| a.id.cmp(&b.id));
        *self.refreshed.lock().unwrap() = Some(Instant::now());
        self.refreshing.store(false, Ordering::SeqCst);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_targets::domain::{ExecutionBinding, SessionExecutionTarget};

    fn branch(reference: &str, device: &str, path: &str) -> NavigationBranch {
        NavigationBranch {
            id: branch_id("repo", reference),
            repository_id: "repo".into(),
            branch_ref: reference.into(),
            created_at: None,
            paths: vec![path.into()],
            worktrees: vec![NavigationWorktree {
                device_id: device.into(),
                device_name: device.into(),
                path: path.into(),
                head: None,
                created_at: None,
                dirty: None,
            }],
        }
    }

    #[test]
    fn grouping_follows_checkout_changes_instead_of_the_stored_branch() {
        let target = SessionExecutionTarget {
            capability_profile_id: "profile".into(),
            capability_profile_revision: 1,
            execution: ExecutionBinding::default(),
            repository_id: "repo".into(),
            branch_ref: "refs/heads/old".into(),
            worktree_id: "worktree".into(),
            path: "C:/active".into(),
            head: None,
        };
        let branches = vec![
            branch("refs/heads/old", "local", "C:/other"),
            branch("refs/heads/new", "remote", "C:/active"),
            branch("refs/heads/new", "local", "C:/active"),
        ];
        assert_eq!(
            current_session_branch(Some(&target), None, &branches)
                .unwrap()
                .branch_ref,
            "refs/heads/new"
        );
        assert_eq!(
            current_session_branch(None, Some("C:/active/"), &branches)
                .unwrap()
                .branch_ref,
            "refs/heads/new"
        );
        assert_eq!(
            current_session_branch(Some(&target), None, &branches[..1])
                .unwrap()
                .branch_ref,
            "refs/heads/old"
        );
    }

    #[test]
    fn remote_grouping_is_device_scoped_and_does_not_use_the_local_directory() {
        let mut execution = ExecutionBinding::default();
        execution.device_id = "remote".into();
        let target = SessionExecutionTarget {
            capability_profile_id: "profile".into(),
            capability_profile_revision: 1,
            execution,
            repository_id: "repo".into(),
            branch_ref: "refs/heads/old".into(),
            worktree_id: "worktree".into(),
            path: "/active".into(),
            head: None,
        };
        let branches = vec![
            branch("refs/heads/local", "local", "/active"),
            branch("refs/heads/new", "remote", "/active"),
        ];
        assert_eq!(
            current_session_branch(Some(&target), Some("/active"), &branches)
                .unwrap()
                .branch_ref,
            "refs/heads/new"
        );
        assert!(current_session_branch(Some(&target), Some("/active"), &branches[..1]).is_none());
    }

    #[test]
    fn branch_keys_are_repository_scoped_and_stable_across_devices() {
        assert_eq!(
            branch_id("repo", "refs/heads/main"),
            "branch:[\"repo\",\"refs/heads/main\"]"
        );
        assert_ne!(
            branch_id("repo", "refs/heads/main"),
            branch_id("other", "refs/heads/main")
        );
    }
}
pub(crate) fn branch_id(repository: &str, branch: &str) -> String {
    format!(
        "branch:{}",
        serde_json::to_string(&(repository, branch)).unwrap()
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionBranch {
    pub(crate) session_id: String,
    pub(crate) branch_id: String,
}
pub(crate) fn session_branches(
    summaries: &[crate::agent_sessions::ports::AgentSessionSummary],
    branches: &[NavigationBranch],
) -> Vec<SessionBranch> {
    summaries
        .iter()
        .filter_map(|summary| {
            let session = &summary.session;
            let branch = current_session_branch(
                session.execution_target.as_ref(),
                session.working_directory.as_deref(),
                branches,
            )?;
            Some(SessionBranch {
                session_id: session.id.as_str().into(),
                branch_id: branch.id.clone(),
            })
        })
        .collect()
}

fn current_session_branch<'a>(
    target: Option<&crate::execution_targets::domain::SessionExecutionTarget>,
    directory: Option<&str>,
    branches: &'a [NavigationBranch],
) -> Option<&'a NavigationBranch> {
    let device = target.map_or("local", |target| target.execution.device_id.as_str());
    let matches_path = |path: &str| {
        let same = |other: &str| {
            if device == "local" {
                crate::repository_context::PathIdentity::of(std::path::Path::new(path))
                    == crate::repository_context::PathIdentity::of(std::path::Path::new(other))
            } else {
                path.trim_end_matches('/') == other.trim_end_matches('/')
            }
        };
        target.is_some_and(|target| same(&target.path))
            || (device == "local" && directory.is_some_and(same))
    };
    // A checkout can change branch without updating the stored execution snapshot.
    branches
        .iter()
        .find(|branch| {
            target.is_none_or(|target| branch.repository_id == target.repository_id)
                && branch
                    .worktrees
                    .iter()
                    .any(|worktree| worktree.device_id == device && matches_path(&worktree.path))
        })
        .or_else(|| {
            let target = target?;
            let id = branch_id(&target.repository_id, &target.branch_ref);
            branches.iter().find(|branch| branch.id == id)
        })
}
