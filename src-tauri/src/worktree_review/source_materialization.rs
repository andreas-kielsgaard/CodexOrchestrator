use super::{
    association_observer::observe_association,
    build_presentation::{BuildSourceInput, BuildWorkspacePlanInput},
    domain::{
        AssociationBaselineKind, BranchRef, GitObjectId, OperationStage, RepositoryId,
        ReviewBuildId, ReviewSourceSelection, ReviewWorkspace, SourceBinding, SourceFingerprint,
        WorkspaceId, WorkspaceLifecycle, WorkspaceOwnership, WorktreeAssociation,
        WorktreeAssociationId, WorktreeAssociationLifecycle, WorktreeAssociationProvenance,
        WorktreeId, WorktreeLocation as StoredLocation,
    },
    storage::{WorktreeAssociationRepository, WorktreeReviewDatabase},
    workspace_provisioner::{ProvisionedWorktree, WorktreeProvisioner},
};
use crate::repository_context::{
    BranchRef as ObservedBranch, FullRefName, ObjectId, RepositoryContext, RepositoryIdentity,
    WorktreeLocation, WorktreeObservation,
};
use chrono::Utc;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub(super) struct SourceMaterializationService {
    database: Arc<WorktreeReviewDatabase>,
    review_root: PathBuf,
}

impl SourceMaterializationService {
    pub(super) fn new(database: Arc<WorktreeReviewDatabase>, review_root: PathBuf) -> Self {
        Self {
            database,
            review_root,
        }
    }

    pub(super) fn selected_branch(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        branch_ref: &str,
    ) -> Result<ObservedBranch, String> {
        let full = FullRefName::parse(branch_ref).map_err(|error| error.to_string())?;
        if full.branch_name().is_none() {
            return Err("Worktree Review builds require a full local branch ref.".into());
        }
        context
            .references()
            .local_branches(repository.top_level.path())
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|branch| branch.full_name == full)
            .ok_or_else(|| "The selected branch is no longer available.".to_string())
    }

    pub(super) fn prepare(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        branch: &ObservedBranch,
        build_id: &ReviewBuildId,
        workspace_id: WorkspaceId,
        source: &BuildSourceInput,
        plan: &BuildWorkspacePlanInput,
    ) -> Result<PreparedMaterialization, String> {
        match (source, plan) {
            (
                BuildSourceInput::ExistingWorktree {
                    association_id,
                    expected_head,
                    state_fingerprint,
                },
                BuildWorkspacePlanInput::BorrowSelectedWorktree {
                    association_id: planned_association,
                },
            ) if association_id == planned_association => {
                let (association, observation, path, fingerprint) = self
                    .verify_association_source(
                        context,
                        repository,
                        branch,
                        association_id,
                        expected_head,
                        state_fingerprint,
                    )?;
                Ok(PreparedMaterialization {
                    workspace: workspace(
                        workspace_id.clone(),
                        repository,
                        &observation,
                        &path,
                        WorkspaceOwnership::BorrowedExternal {
                            association_id: association.id.clone(),
                        },
                    )?,
                    source: source_binding(
                        repository,
                        branch,
                        workspace_id,
                        ReviewSourceSelection::ExistingWorktree {
                            association_id: association.id,
                            expected_head: git_object(&observation.head)?,
                            captured_state_fingerprint: fingerprint.clone(),
                        },
                        &observation.head,
                        fingerprint,
                    )?,
                    action: MaterializationAction::AlreadyMaterialized,
                })
            }
            (
                BuildSourceInput::WorktreeSnapshot {
                    association_id,
                    base_object_id,
                    state_fingerprint,
                },
                BuildWorkspacePlanInput::CreateOwnedBuildWorktree {
                    originating_association_id,
                    object_id,
                },
            ) if originating_association_id.as_deref() == Some(association_id)
                && object_id == base_object_id =>
            {
                let (association, observation, source_path, fingerprint) = self
                    .verify_association_source(
                        context,
                        repository,
                        branch,
                        association_id,
                        base_object_id,
                        state_fingerprint,
                    )?;
                let workspace = self.provisioner(context, repository)?.plan_workspace(
                    repository,
                    workspace_id.clone(),
                    WorkspaceOwnership::OwnedBuildWorktree {
                        build_id: build_id.clone(),
                    },
                )?;
                Ok(PreparedMaterialization {
                    source: source_binding(
                        repository,
                        branch,
                        workspace_id,
                        ReviewSourceSelection::WorktreeSnapshot {
                            association_id: association.id,
                            baseline_object: git_object(&observation.head)?,
                            captured_state_fingerprint: fingerprint.clone(),
                        },
                        &observation.head,
                        fingerprint.clone(),
                    )?,
                    workspace,
                    action: MaterializationAction::Snapshot {
                        source_path,
                        fingerprint,
                        head: observation.head,
                    },
                })
            }
            (
                BuildSourceInput::BranchCommit {
                    branch_ref,
                    object_id,
                },
                BuildWorkspacePlanInput::CreateManagedBranchWorktree {
                    branch_ref: planned_branch,
                },
            ) if branch_ref == planned_branch && branch_ref == branch.full_name.as_str() => self
                .prepare_commit(
                    context,
                    repository,
                    branch,
                    build_id,
                    workspace_id,
                    object_id,
                    true,
                ),
            (
                BuildSourceInput::BranchCommit {
                    branch_ref,
                    object_id,
                },
                BuildWorkspacePlanInput::CreateOwnedBuildWorktree {
                    originating_association_id: None,
                    object_id: planned_object,
                },
            ) if object_id == planned_object && branch_ref == branch.full_name.as_str() => self
                .prepare_commit(
                    context,
                    repository,
                    branch,
                    build_id,
                    workspace_id,
                    object_id,
                    false,
                ),
            _ => Err(
                "The build source and workspace plan do not describe the same explicit source."
                    .into(),
            ),
        }
    }

    pub(super) fn materialize(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        branch: &ObservedBranch,
        prepared: PreparedMaterialization,
    ) -> Result<MaterializedSource, String> {
        let PreparedMaterialization {
            workspace,
            source,
            action,
        } = prepared;
        match action {
            MaterializationAction::AlreadyMaterialized => Ok(MaterializedSource {
                workspace,
                association: None,
            }),
            MaterializationAction::Snapshot {
                source_path,
                fingerprint,
                head,
            } => {
                let provisioned = self.provisioner(context, repository)?.create_snapshot(
                    repository,
                    workspace.id.as_str(),
                    &source_path,
                    fingerprint.as_str(),
                    &head,
                )?;
                Ok(MaterializedSource {
                    workspace: verified_workspace(workspace, &source, context, &provisioned)?,
                    association: None,
                })
            }
            MaterializationAction::Commit {
                object,
                managed_association_id,
            } => {
                let provisioned = self.provisioner(context, repository)?.create_at_commit(
                    repository,
                    workspace.id.as_str(),
                    &object,
                )?;
                let workspace = verified_workspace(workspace, &source, context, &provisioned)?;
                let association = managed_association_id
                    .map(|association_id| {
                        observe_association(
                            context,
                            repository,
                            branch,
                            &provisioned.observation,
                            association_id,
                            WorktreeAssociationProvenance::ProductCreated,
                            provisioned.observation.head.clone(),
                            AssociationBaselineKind::CreatedAtObject,
                        )
                    })
                    .transpose()?;
                Ok(MaterializedSource {
                    workspace,
                    association,
                })
            }
        }
    }

    fn prepare_commit(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        branch: &ObservedBranch,
        build_id: &ReviewBuildId,
        workspace_id: WorkspaceId,
        object_id: &str,
        managed: bool,
    ) -> Result<PreparedMaterialization, String> {
        let object = ObjectId::parse(object_id).map_err(|error| error.to_string())?;
        if !context
            .commits()
            .is_ancestor(repository.top_level.path(), &object, &branch.object_id)
            .map_err(|error| error.to_string())?
        {
            return Err("The selected commit does not belong to the selected branch.".into());
        }
        let clean_fingerprint = context.status().clean_source_fingerprint(&object);
        let fingerprint = SourceFingerprint::new(clean_fingerprint.as_str())
            .map_err(|error| error.to_string())?;
        let (ownership, association_id) = if managed {
            let association_id = WorktreeAssociationId::random();
            (
                WorkspaceOwnership::ManagedBranchWorktree {
                    association_id: association_id.clone(),
                },
                Some(association_id),
            )
        } else {
            (
                WorkspaceOwnership::OwnedBuildWorktree {
                    build_id: build_id.clone(),
                },
                None,
            )
        };
        Ok(PreparedMaterialization {
            workspace: self.provisioner(context, repository)?.plan_workspace(
                repository,
                workspace_id.clone(),
                ownership,
            )?,
            source: source_binding(
                repository,
                branch,
                workspace_id,
                ReviewSourceSelection::BranchCommit {
                    selected_object: git_object(&object)?,
                },
                &object,
                fingerprint,
            )?,
            action: MaterializationAction::Commit {
                object,
                managed_association_id: association_id,
            },
        })
    }

    fn verify_association_source(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        branch: &ObservedBranch,
        association_id: &str,
        expected_head: &str,
        expected_fingerprint: &str,
    ) -> Result<
        (
            WorktreeAssociation,
            WorktreeObservation,
            PathBuf,
            SourceFingerprint,
        ),
        String,
    > {
        let association_id =
            WorktreeAssociationId::new(association_id).map_err(|error| error.to_string())?;
        let association = self
            .database
            .associations()
            .find(&association_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The selected worktree association is unavailable.".to_string())?;
        if association.repository_id.as_str() != repository.id.as_str()
            || association.branch_ref.as_str() != branch.full_name.as_str()
            || association.lifecycle != WorktreeAssociationLifecycle::Active
        {
            return Err("The selected worktree association is not active for this branch.".into());
        }
        let observation = context
            .worktrees()
            .list(&repository.id, repository.top_level.path())
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|worktree| worktree.id.as_str() == association.worktree_id.as_str())
            .ok_or_else(|| "The selected worktree is no longer registered with Git.".to_string())?;
        let path = available_path_owned(&observation)?;
        let (head, fingerprint) = context
            .status()
            .source_fingerprint(&path)
            .map_err(|error| error.to_string())?;
        if head.as_str() != expected_head || fingerprint.as_str() != expected_fingerprint {
            return Err(
                "The selected worktree changed after it was presented for building.".into(),
            );
        }
        if !context
            .commits()
            .is_ancestor(repository.top_level.path(), &head, &branch.object_id)
            .map_err(|error| error.to_string())?
        {
            return Err("The selected worktree no longer belongs to the selected branch.".into());
        }
        let fingerprint =
            SourceFingerprint::new(fingerprint.as_str()).map_err(|error| error.to_string())?;
        Ok((association, observation, path, fingerprint))
    }

    fn provisioner(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
    ) -> Result<WorktreeProvisioner, String> {
        WorktreeProvisioner::open(
            context.clone(),
            self.review_root
                .join("repositories")
                .join(repository.id.as_str()),
        )
    }
}

pub(super) struct PreparedMaterialization {
    workspace: ReviewWorkspace,
    source: SourceBinding,
    action: MaterializationAction,
}

impl PreparedMaterialization {
    pub(super) fn workspace(&self) -> &ReviewWorkspace {
        &self.workspace
    }

    pub(super) fn source(&self) -> &SourceBinding {
        &self.source
    }

    pub(super) fn stage(&self) -> OperationStage {
        self.action.stage()
    }
}

pub(super) struct MaterializedSource {
    workspace: ReviewWorkspace,
    association: Option<WorktreeAssociation>,
}

impl MaterializedSource {
    pub(super) fn workspace(&self) -> &ReviewWorkspace {
        &self.workspace
    }

    pub(super) fn association(&self) -> Option<&WorktreeAssociation> {
        self.association.as_ref()
    }
}

enum MaterializationAction {
    AlreadyMaterialized,
    Snapshot {
        source_path: PathBuf,
        fingerprint: SourceFingerprint,
        head: ObjectId,
    },
    Commit {
        object: ObjectId,
        managed_association_id: Option<WorktreeAssociationId>,
    },
}

impl MaterializationAction {
    fn stage(&self) -> OperationStage {
        match self {
            Self::AlreadyMaterialized => OperationStage::SourceVerification,
            Self::Snapshot { .. } | Self::Commit { .. } => OperationStage::WorktreeProvisioning,
        }
    }
}

fn workspace(
    id: WorkspaceId,
    repository: &RepositoryIdentity,
    observation: &WorktreeObservation,
    path: &Path,
    ownership: WorkspaceOwnership,
) -> Result<ReviewWorkspace, String> {
    let now = Utc::now();
    Ok(ReviewWorkspace {
        id,
        repository_id: RepositoryId::new(repository.id.as_str())
            .map_err(|error| error.to_string())?,
        worktree_id: WorktreeId::new(observation.id.as_str()).map_err(|error| error.to_string())?,
        location: StoredLocation::new(path.to_string_lossy().into_owned())
            .map_err(|error| error.to_string())?,
        ownership,
        lifecycle: WorkspaceLifecycle::Ready,
        created_at: now,
        updated_at: now,
    })
}

fn verified_workspace(
    mut workspace: ReviewWorkspace,
    source: &SourceBinding,
    context: &RepositoryContext,
    provisioned: &ProvisionedWorktree,
) -> Result<ReviewWorkspace, String> {
    let (head, fingerprint) = context
        .status()
        .source_fingerprint(&provisioned.path)
        .map_err(|error| error.to_string())?;
    if workspace.worktree_id.as_str() != provisioned.observation.id.as_str()
        || source.workspace_id != workspace.id
        || source.materialized_object.as_str() != head.as_str()
        || source.materialized_state_fingerprint.as_str() != fingerprint.as_str()
    {
        return Err("The retained checkout does not match its durable source plan.".into());
    }
    workspace.location = StoredLocation::new(provisioned.path.to_string_lossy().into_owned())
        .map_err(|error| error.to_string())?;
    workspace.lifecycle = WorkspaceLifecycle::Ready;
    workspace.updated_at = Utc::now();
    Ok(workspace)
}

fn source_binding(
    repository: &RepositoryIdentity,
    branch: &ObservedBranch,
    workspace_id: WorkspaceId,
    selection: ReviewSourceSelection,
    materialized_object: &ObjectId,
    materialized_state_fingerprint: SourceFingerprint,
) -> Result<SourceBinding, String> {
    Ok(SourceBinding {
        repository_id: RepositoryId::new(repository.id.as_str())
            .map_err(|error| error.to_string())?,
        branch_ref: BranchRef::new(branch.full_name.as_str()).map_err(|error| error.to_string())?,
        selection,
        workspace_id,
        materialized_object: git_object(materialized_object)?,
        materialized_state_fingerprint,
    })
}

fn available_path_owned(observation: &WorktreeObservation) -> Result<PathBuf, String> {
    match &observation.location {
        WorktreeLocation::Available(directory) => Ok(directory.path().to_path_buf()),
        WorktreeLocation::Unavailable(_) => Err("The selected worktree is unavailable.".into()),
    }
}

fn git_object(object: &ObjectId) -> Result<GitObjectId, String> {
    GitObjectId::new(object.as_str()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_plan_must_repeat_the_selected_source_identity() {
        let source = BuildSourceInput::ExistingWorktree {
            association_id: "association-one".into(),
            expected_head: "a".repeat(40),
            state_fingerprint: "fingerprint".into(),
        };
        let mismatched = BuildWorkspacePlanInput::BorrowSelectedWorktree {
            association_id: "association-two".into(),
        };
        assert!(!matches!(
            (&source, &mismatched),
            (
                BuildSourceInput::ExistingWorktree { association_id, .. },
                BuildWorkspacePlanInput::BorrowSelectedWorktree {
                    association_id: planned
                }
            ) if association_id == planned
        ));
    }
}
