use crate::repository_context::{
    RepositoryContext, RepositoryIdentity, WorktreeLocation, WorktreeObservation,
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    path::Component,
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivityEstimate {
    pub(crate) changed_at: String,
    pub(crate) observed_at: String,
    pub(crate) basis: String,
    pub(crate) staged_files: usize,
    pub(crate) unstaged_files: usize,
    pub(crate) untracked_files: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeActivityView {
    pub(crate) worktree_id: String,
    pub(crate) head_object_id: String,
    pub(crate) estimate: ActivityEstimate,
}

#[derive(Default)]
pub(super) struct WorktreeActivityService {
    cache: Mutex<HashMap<String, (String, Instant, ActivityEstimate)>>,
}

impl WorktreeActivityService {
    pub(super) fn cached(&self, observation: &WorktreeObservation) -> Option<ActivityEstimate> {
        self.cache
            .lock()
            .ok()?
            .get(observation.id.as_str())
            .filter(|(head, _, _)| head == observation.head.as_str())
            .map(|(_, _, estimate)| estimate.clone())
    }

    /// The caller requests small lazy batches; neither overview nor graph opening scans files.
    pub(super) fn discover(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        ids: &[String],
    ) -> Result<Vec<WorktreeActivityView>, String> {
        if ids.len() > 4 {
            return Err("Request at most four worktree activity estimates at a time.".into());
        }
        let observations = context
            .worktrees()
            .list(&repository.id, repository.top_level.path())
            .map_err(|error| error.to_string())?;
        let mut result = Vec::new();
        for observation in observations
            .iter()
            .filter(|item| ids.iter().any(|id| id == item.id.as_str()))
        {
            let WorktreeLocation::Available(root) = &observation.location else {
                continue;
            };
            let fresh = self
                .cache
                .lock()
                .map_err(|_| "Activity cache unavailable")?
                .get(observation.id.as_str())
                .filter(|(head, observed, _)| {
                    head == observation.head.as_str()
                        && observed.elapsed() < Duration::from_secs(30)
                })
                .map(|(_, _, estimate)| estimate.clone());
            let estimate = if let Some(estimate) = fresh {
                estimate
            } else {
                let facts = context
                    .commits()
                    .facts(repository.top_level.path(), &observation.head)
                    .map_err(|error| error.to_string())?;
                let mut changed_at = DateTime::parse_from_rfc3339(&facts.committed_at)
                    .map_err(|error| error.to_string())?
                    .with_timezone(&Utc);
                let (status, paths) = context
                    .status()
                    .changed_files(root.path())
                    .map_err(|error| error.to_string())?;
                let mut basis = "commit_fallback";
                for path in paths {
                    if !path
                        .components()
                        .all(|part| matches!(part, Component::Normal(_)))
                    {
                        continue;
                    }
                    let candidate = root.path().join(path);
                    // Canonical containment excludes symlinked source outside this checkout.
                    let Ok(canonical) = candidate.canonicalize() else {
                        continue;
                    };
                    if !canonical.starts_with(root.path()) {
                        continue;
                    }
                    if let Ok(time) =
                        fs::metadata(&canonical).and_then(|metadata| metadata.modified())
                    {
                        changed_at = changed_at.max(DateTime::<Utc>::from(time));
                        basis = "changed_file_estimate";
                    }
                }
                if status.staged_paths + status.unstaged_paths > 0 {
                    // The index provides cheap evidence for edits with no remaining file.
                    let git_entry = root.path().join(".git");
                    let git_dir = if git_entry.is_file() {
                        fs::read_to_string(&git_entry).ok().and_then(|text| {
                            text.trim()
                                .strip_prefix("gitdir: ")
                                .map(|path| root.path().join(path))
                        })
                    } else {
                        Some(git_entry)
                    };
                    if let Some(time) = git_dir
                        .and_then(|dir| fs::metadata(dir.join("index")).ok())
                        .and_then(|metadata| metadata.modified().ok())
                    {
                        changed_at = changed_at.max(DateTime::<Utc>::from(time));
                        basis = "changed_file_and_index_estimate";
                    }
                }
                let estimate = ActivityEstimate {
                    changed_at: changed_at.to_rfc3339(),
                    observed_at: Utc::now().to_rfc3339(),
                    basis: basis.into(),
                    staged_files: status.staged_paths,
                    unstaged_files: status.unstaged_paths,
                    untracked_files: status.untracked_paths,
                };
                self.cache
                    .lock()
                    .map_err(|_| "Activity cache unavailable")?
                    .insert(
                        observation.id.as_str().into(),
                        (
                            observation.head.as_str().into(),
                            Instant::now(),
                            estimate.clone(),
                        ),
                    );
                estimate
            };
            result.push(WorktreeActivityView {
                worktree_id: observation.id.as_str().into(),
                head_object_id: observation.head.as_str().into(),
                estimate,
            });
        }
        Ok(result)
    }
}
