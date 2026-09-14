use super::{
    branch_presentation::{
        branch_view, commit_view, repository_name, worktree_availability, BranchView,
        WorktreeAvailabilityView,
    },
    domain::{
        BranchRef, GitObjectId, RepositoryId, ReviewBranch, ReviewRepository, ReviewTarget,
        WorktreeAssociationLifecycle,
    },
    storage::{ReviewRepositoryRepository, WorktreeAssociationRepository, WorktreeReviewDatabase},
    worktree_activity::WorktreeActivityService,
};
use crate::repository_context::{RepositoryContext, RepositoryIdentity, WorktreeLocation};
use chrono::Utc;
use std::collections::HashSet;

pub(super) fn inventory(
    context: &RepositoryContext,
    repository: &RepositoryIdentity,
    database: &WorktreeReviewDatabase,
    activity: &WorktreeActivityService,
) -> Result<Vec<BranchView>, String> {
    let now = Utc::now();
    let id = RepositoryId::new(repository.id.as_str()).map_err(|error| error.to_string())?;
    database
        .repositories()
        .save_repository(&ReviewRepository {
            id: id.clone(),
            label: repository_name(repository.top_level.path()),
            anchor_root: repository.top_level.path().into(),
            common_directory: repository.common_directory.path().into(),
            first_seen_at: now,
            last_seen_at: now,
        })
        .map_err(|error| error.to_string())?;
    let observations = context
        .worktrees()
        .list(&repository.id, repository.top_level.path())
        .map_err(|error| error.to_string())?;
    let associations = database
        .associations()
        .list_for_repository(&id)
        .map_err(|error| error.to_string())?;
    let default = context
        .references()
        .remote_default_branch(repository.top_level.path())
        .map_err(|error| error.to_string())?;
    let summaries = context
        .references()
        .local_branch_summaries(repository.top_level.path(), default.as_ref())
        .map_err(|error| error.to_string())?;
    let mut targets = Vec::new();
    for summary in summaries {
        database
            .repositories()
            .save_branch(&ReviewBranch {
                repository_id: id.clone(),
                branch_ref: BranchRef::new(summary.branch.full_name.as_str())
                    .map_err(|error| error.to_string())?,
                observed_tip: GitObjectId::new(summary.branch.object_id.as_str())
                    .map_err(|error| error.to_string())?,
                observed_at: now,
            })
            .map_err(|error| error.to_string())?;
        let related = associations
            .iter()
            .filter(|item| {
                item.branch_ref.as_str() == summary.branch.full_name.as_str()
                    && item.lifecycle != WorktreeAssociationLifecycle::Disassociated
            })
            .collect::<Vec<_>>();
        let mut retained = related
            .iter()
            .map(|item| item.worktree_id.as_str().to_owned())
            .collect::<HashSet<_>>();
        let available = observations
            .iter()
            .filter(|item| {
                if item.head_ref.as_ref() == Some(&summary.branch.full_name) {
                    return matches!(item.location, WorktreeLocation::Available(_));
                }
                related.iter().any(|association| {
                    association.worktree_id.as_str() == item.id.as_str()
                        && matches!(
                            worktree_availability(association, Some(item)),
                            WorktreeAvailabilityView::Available
                        )
                })
            })
            .collect::<Vec<_>>();
        retained.extend(available.iter().map(|item| item.id.as_str().to_owned()));
        let mut view = branch_view(
            repository,
            &summary.branch,
            commit_view(summary.tip),
            summary.ahead,
            summary.behind,
            retained.len(),
        );
        view.available_worktree_count = available.len();
        view.worktree_ids = available
            .iter()
            .map(|item| item.id.as_str().into())
            .collect();
        view.activity = available
            .iter()
            .filter_map(|item| activity.cached(item))
            .max_by(|left, right| left.changed_at.cmp(&right.changed_at));
        targets.push(view);
    }
    for observation in observations.iter().filter(|item| {
        item.head_ref.is_none() && matches!(item.location, WorktreeLocation::Available(_))
    }) {
        let tip = commit_view(
            context
                .commits()
                .facts(repository.top_level.path(), &observation.head)
                .map_err(|error| error.to_string())?,
        );
        let path = match &observation.location {
            WorktreeLocation::Available(path) => path.path(),
            _ => unreachable!(),
        };
        let provenance = associations
            .iter()
            .filter(|item| item.worktree_id.as_str() == observation.id.as_str())
            .map(|item| item.branch_ref.as_str().trim_start_matches("refs/heads/"))
            .collect::<Vec<_>>();
        let label = if provenance.is_empty() {
            format!("Detached · {}", repository_name(path))
        } else {
            format!("Detached · {}", provenance.join(", "))
        };
        targets.push(BranchView {
            target: ReviewTarget::Worktree {
                repository_id: id.as_str().into(),
                worktree_id: observation.id.as_str().into(),
            },
            repository_id: id.as_str().into(),
            branch_ref: None,
            display_name: label,
            tip,
            ahead_of_default: 0,
            behind_default: 0,
            associated_worktree_count: 1,
            available_worktree_count: 1,
            worktree_ids: vec![observation.id.as_str().into()],
            activity: activity.cached(observation),
        });
    }
    Ok(targets)
}
