use super::domain::{
    AssociationBaseline, AssociationBaselineKind, BranchRef, GitObjectId, RepositoryId,
    WorktreeAssociation, WorktreeAssociationId, WorktreeAssociationLifecycle,
    WorktreeAssociationProvenance, WorktreeChangeSummary, WorktreeId, WorktreeLocation,
    WorktreeObservedState,
};
use crate::repository_context::{
    BranchRef as ObservedBranch, ObjectId, RepositoryContext, RepositoryIdentity,
    WorktreeLocation as ObservedLocation, WorktreeObservation,
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Constructs the one durable association shape from freshly observed Git facts.
pub(crate) fn observe_association(
    context: &RepositoryContext,
    repository: &RepositoryIdentity,
    branch: &ObservedBranch,
    observation: &WorktreeObservation,
    id: WorktreeAssociationId,
    provenance: WorktreeAssociationProvenance,
    baseline: ObjectId,
    baseline_kind: AssociationBaselineKind,
) -> Result<WorktreeAssociation, String> {
    let path = available_path(observation)?;
    let status = context
        .status()
        .status(path)
        .map_err(|error| error.to_string())?;
    let divergence = context
        .commits()
        .divergence(repository.top_level.path(), &baseline, &observation.head)
        .map_err(|error| error.to_string())?;
    let reachable = context
        .commits()
        .is_ancestor(
            repository.top_level.path(),
            &observation.head,
            &branch.object_id,
        )
        .map_err(|error| error.to_string())?;
    let now = Utc::now();
    Ok(WorktreeAssociation {
        id,
        repository_id: RepositoryId::new(repository.id.as_str())
            .map_err(|error| error.to_string())?,
        branch_ref: BranchRef::new(branch.full_name.as_str()).map_err(|error| error.to_string())?,
        worktree_id: WorktreeId::new(observation.id.as_str()).map_err(|error| error.to_string())?,
        location: WorktreeLocation::new(path.to_string_lossy().into_owned())
            .map_err(|error| error.to_string())?,
        provenance,
        baseline: AssociationBaseline {
            object_id: Some(object_id(&baseline)?),
            kind: baseline_kind,
        },
        observed_state: WorktreeObservedState {
            head: object_id(&observation.head)?,
            commits_ahead_of_baseline: divergence.ahead.try_into().unwrap_or(u32::MAX),
            commits_behind_baseline: divergence.behind.try_into().unwrap_or(u32::MAX),
            changes: WorktreeChangeSummary {
                staged_paths: status.staged_paths.try_into().unwrap_or(u32::MAX),
                unstaged_paths: status.unstaged_paths.try_into().unwrap_or(u32::MAX),
                untracked_paths: status.untracked_paths.try_into().unwrap_or(u32::MAX),
            },
            detached_head: observation.head_ref.is_none(),
            reachable_from_associated_branch: reachable,
            observed_at: now,
        },
        lifecycle: if reachable {
            WorktreeAssociationLifecycle::Active
        } else {
            WorktreeAssociationLifecycle::BranchMismatch
        },
        associated_at: now,
        updated_at: now,
    })
}

pub(crate) fn stable_association_id(repository: &str, branch: &str, worktree: &str) -> String {
    let mut hash = Sha256::new();
    for value in [repository, branch, worktree] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    format!("association-{}", &format!("{:x}", hash.finalize())[..24])
}

fn available_path(worktree: &WorktreeObservation) -> Result<&Path, String> {
    match &worktree.location {
        ObservedLocation::Available(directory) => Ok(directory.path()),
        ObservedLocation::Unavailable(_) => Err("The worktree checkout is unavailable.".into()),
    }
}

fn object_id(object: &ObjectId) -> Result<GitObjectId, String> {
    GitObjectId::new(object.as_str()).map_err(|error| error.to_string())
}
