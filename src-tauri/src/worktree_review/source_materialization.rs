use super::{
    association_observer::observe_association,
    build_presentation::{BuildWorkspacePlanInput, CreateBuildSourceInput},
    domain::{
        AssociationBaselineKind, BranchRef, GitObjectId, OperationStage, RepositoryId,
        ReviewBuildId, ReviewSourceSelection, ReviewWorkspace, SourceBinding, WorkspaceId,
        WorkspaceLifecycle, WorkspaceOwnership, WorktreeAssociation, WorktreeAssociationId,
        WorktreeAssociationLifecycle, WorktreeAssociationProvenance, WorktreeId,
        WorktreeLocation as StoredLocation,
    },
    storage::{WorktreeAssociationRepository, WorktreeReviewDatabase},
};
use crate::{
    repository_context::{
        BranchRef as ObservedBranch, FullRefName, ObjectId, RepositoryContext, RepositoryIdentity,
        WorktreeLocation, WorktreeObservation, WorktreeObservationId,
    },
    worktree_application::{
        GitCommitId, PhysicalWorktreeApplication, PhysicalWorktreeAttachment,
        PhysicalWorktreeCheckoutRequest, VirtualCommitCaptureRequest,
    },
};
use chrono::Utc;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

pub(super) struct SourceMaterializationService {
    database: Arc<WorktreeReviewDatabase>,
    review_root: PathBuf,
    physical_worktrees: PhysicalWorktreeApplication,
}

impl SourceMaterializationService {
    pub(super) fn new(database: Arc<WorktreeReviewDatabase>, review_root: PathBuf) -> Self {
        Self {
            database,
            review_root,
            physical_worktrees: PhysicalWorktreeApplication,
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
        source: &CreateBuildSourceInput,
        plan: &BuildWorkspacePlanInput,
    ) -> Result<PreparedMaterialization, String> {
        match (source, plan) {
            (
                CreateBuildSourceInput::ExistingWorktree { association_id },
                BuildWorkspacePlanInput::BorrowSelectedWorktree {
                    association_id: planned_association,
                },
            ) if association_id == planned_association => {
                let accepted =
                    self.accept_worktree_source(context, repository, branch, association_id)?;
                Ok(PreparedMaterialization {
                    workspace: borrowed_workspace(
                        workspace_id.clone(),
                        repository,
                        &accepted.observation,
                        &accepted.path,
                        accepted.association.id.clone(),
                    )?,
                    source: source_binding(
                        repository,
                        branch,
                        workspace_id,
                        ReviewSourceSelection::ExistingWorktree {
                            association_id: accepted.association.id,
                            head_object_id: git_object(&accepted.head)?,
                            virtual_commit_id: accepted
                                .virtual_commit
                                .as_ref()
                                .map(git_object)
                                .transpose()?,
                        },
                    )?,
                    action: MaterializationAction::AlreadyMaterialized,
                })
            }
            (
                CreateBuildSourceInput::WorktreeSnapshot { association_id },
                BuildWorkspacePlanInput::CreateOwnedBuildWorktree {
                    originating_association_id,
                },
            ) if originating_association_id.as_deref() == Some(association_id) => {
                let accepted =
                    self.accept_worktree_source(context, repository, branch, association_id)?;
                let captured_object = accepted
                    .virtual_commit
                    .as_ref()
                    .unwrap_or(&accepted.head)
                    .clone();
                Ok(PreparedMaterialization {
                    workspace: self.plan_workspace(
                        repository,
                        workspace_id.clone(),
                        WorkspaceOwnership::OwnedBuildWorktree {
                            build_id: build_id.clone(),
                        },
                    )?,
                    source: source_binding(
                        repository,
                        branch,
                        workspace_id,
                        ReviewSourceSelection::WorktreeSnapshot {
                            association_id: accepted.association.id,
                            head_object_id: git_object(&accepted.head)?,
                            captured_object_id: git_object(&captured_object)?,
                            virtual_commit_id: accepted
                                .virtual_commit
                                .as_ref()
                                .map(git_object)
                                .transpose()?,
                        },
                    )?,
                    action: MaterializationAction::Checkout {
                        object: captured_object,
                        managed_association_id: None,
                    },
                })
            }
            (
                CreateBuildSourceInput::BranchCommit {
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
                CreateBuildSourceInput::BranchCommit {
                    branch_ref,
                    object_id,
                },
                BuildWorkspacePlanInput::CreateOwnedBuildWorktree {
                    originating_association_id: None,
                },
            ) if branch_ref == branch.full_name.as_str() => self.prepare_commit(
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
            source: _,
            action,
        } = prepared;
        match action {
            MaterializationAction::AlreadyMaterialized => Ok(MaterializedSource {
                workspace,
                association: None,
            }),
            MaterializationAction::Checkout {
                object,
                managed_association_id,
            } => {
                let checked_out = self.materialize_workspace(
                    context,
                    repository,
                    workspace,
                    &object,
                    PhysicalWorktreeAttachment::Detached,
                )?;
                let association = managed_association_id
                    .map(|association_id| {
                        observe_association(
                            context,
                            repository,
                            branch,
                            &checked_out.observation,
                            association_id,
                            WorktreeAssociationProvenance::ProductCreated,
                            checked_out.observation.head.clone(),
                            AssociationBaselineKind::CreatedAtObject,
                        )
                    })
                    .transpose()?;
                Ok(MaterializedSource {
                    workspace: checked_out.workspace,
                    association,
                })
            }
        }
    }

    pub(super) fn plan_workspace(
        &self,
        repository: &RepositoryIdentity,
        workspace_id: WorkspaceId,
        ownership: WorkspaceOwnership,
    ) -> Result<ReviewWorkspace, String> {
        let root = self.workspace_root(repository)?;
        let path = root.join(workspace_id.as_str());
        if path.exists() || fs::symlink_metadata(&path).is_ok() {
            return Err("The planned worktree location is already in use.".into());
        }
        let now = Utc::now();
        Ok(ReviewWorkspace {
            id: workspace_id,
            repository_id: RepositoryId::new(repository.id.as_str())
                .map_err(|error| error.to_string())?,
            worktree_id: WorktreeId::new(
                WorktreeObservationId::for_path(&repository.id, &path).as_str(),
            )
            .map_err(|error| error.to_string())?,
            location: StoredLocation::new(path.to_string_lossy().into_owned())
                .map_err(|error| error.to_string())?,
            ownership,
            lifecycle: WorkspaceLifecycle::Unverified,
            created_at: now,
            updated_at: now,
        })
    }

    pub(super) fn materialize_branch_workspace(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        branch: &ObservedBranch,
        workspace: ReviewWorkspace,
    ) -> Result<MaterializedCheckout, String> {
        let attachment =
            PhysicalWorktreeAttachment::existing_branch(branch.full_name.as_str().to_owned())
                .map_err(|error| error.to_string())?;
        self.materialize_workspace(
            context,
            repository,
            workspace,
            &branch.object_id,
            attachment,
        )
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
            workspace: self.plan_workspace(repository, workspace_id.clone(), ownership)?,
            source: source_binding(
                repository,
                branch,
                workspace_id,
                ReviewSourceSelection::BranchCommit {
                    selected_object: git_object(&object)?,
                },
            )?,
            action: MaterializationAction::Checkout {
                object,
                managed_association_id: association_id,
            },
        })
    }

    fn accept_worktree_source(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        branch: &ObservedBranch,
        association_id: &str,
    ) -> Result<AcceptedWorktreeSource, String> {
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
        let capture = self
            .physical_worktrees
            .capture_virtual_commit(
                context.git_executable(),
                &VirtualCommitCaptureRequest::new(
                    path.clone(),
                    physical_object(&observation.head)?,
                )
                .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        let head =
            ObjectId::parse(capture.baseline_commit.as_str()).map_err(|error| error.to_string())?;
        if capture.worktree_root != path
            || capture.captured_changes != capture.virtual_commit.is_some()
            || capture.baseline_commit.as_str() != observation.head.as_str()
        {
            return Err(
                "The captured worktree source does not match its accepted checkout.".into(),
            );
        }
        if !context
            .commits()
            .is_ancestor(repository.top_level.path(), &head, &branch.object_id)
            .map_err(|error| error.to_string())?
        {
            return Err("The selected worktree no longer belongs to the selected branch.".into());
        }
        let virtual_commit = capture
            .virtual_commit
            .as_ref()
            .map(|object| ObjectId::parse(object.as_str()).map_err(|error| error.to_string()))
            .transpose()?;
        Ok(AcceptedWorktreeSource {
            association,
            observation,
            path,
            head,
            virtual_commit,
        })
    }

    fn materialize_workspace(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        mut workspace: ReviewWorkspace,
        object: &ObjectId,
        attachment: PhysicalWorktreeAttachment,
    ) -> Result<MaterializedCheckout, String> {
        let planned_path = PathBuf::from(workspace.location.as_str());
        let expected_path = self.workspace_root(repository)?.join(workspace.id.as_str());
        if planned_path != expected_path
            || workspace.repository_id.as_str() != repository.id.as_str()
        {
            return Err("The durable workspace plan does not match its checkout target.".into());
        }
        let result = self
            .physical_worktrees
            .materialize_checkout(
                context.git_executable(),
                &PhysicalWorktreeCheckoutRequest::new(
                    repository.top_level.path().to_path_buf(),
                    planned_path,
                    physical_object(object)?,
                    attachment,
                )
                .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        let observation = context
            .worktrees()
            .list(&repository.id, repository.top_level.path())
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|worktree| {
                matches!(
                    &worktree.location,
                    WorktreeLocation::Available(directory)
                        if directory.path() == result.worktree_root
                )
            })
            .ok_or_else(|| "Git did not report the materialized checkout.".to_string())?;
        if workspace.worktree_id.as_str() != observation.id.as_str()
            || observation.head.as_str() != result.commit_id.as_str()
            || observation.head_ref.as_ref().map(FullRefName::as_str) != result.head_ref.as_deref()
        {
            return Err(
                "The materialized checkout does not match its durable workspace plan.".into(),
            );
        }
        workspace.location =
            StoredLocation::new(result.worktree_root.to_string_lossy().into_owned())
                .map_err(|error| error.to_string())?;
        workspace.lifecycle = WorkspaceLifecycle::Ready;
        workspace.updated_at = Utc::now();
        Ok(MaterializedCheckout {
            workspace,
            observation,
        })
    }

    fn workspace_root(&self, repository: &RepositoryIdentity) -> Result<PathBuf, String> {
        let root = self
            .review_root
            .join("repositories")
            .join(repository.id.as_str())
            .join("worktrees");
        fs::create_dir_all(&root)
            .map_err(|_| "Worktree Review workspace storage is unavailable.".to_string())?;
        root.canonicalize()
            .map_err(|_| "Worktree Review workspace storage is unavailable.".to_string())
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

pub(super) struct MaterializedCheckout {
    pub(super) workspace: ReviewWorkspace,
    pub(super) observation: WorktreeObservation,
}

struct AcceptedWorktreeSource {
    association: WorktreeAssociation,
    observation: WorktreeObservation,
    path: PathBuf,
    head: ObjectId,
    virtual_commit: Option<ObjectId>,
}

enum MaterializationAction {
    AlreadyMaterialized,
    Checkout {
        object: ObjectId,
        managed_association_id: Option<WorktreeAssociationId>,
    },
}

impl MaterializationAction {
    fn stage(&self) -> OperationStage {
        match self {
            Self::AlreadyMaterialized => OperationStage::SourceVerification,
            Self::Checkout { .. } => OperationStage::WorktreeProvisioning,
        }
    }
}

fn borrowed_workspace(
    id: WorkspaceId,
    repository: &RepositoryIdentity,
    observation: &WorktreeObservation,
    path: &Path,
    association_id: WorktreeAssociationId,
) -> Result<ReviewWorkspace, String> {
    let now = Utc::now();
    Ok(ReviewWorkspace {
        id,
        repository_id: RepositoryId::new(repository.id.as_str())
            .map_err(|error| error.to_string())?,
        worktree_id: WorktreeId::new(observation.id.as_str()).map_err(|error| error.to_string())?,
        location: StoredLocation::new(path.to_string_lossy().into_owned())
            .map_err(|error| error.to_string())?,
        ownership: WorkspaceOwnership::BorrowedExternal { association_id },
        lifecycle: WorkspaceLifecycle::Ready,
        created_at: now,
        updated_at: now,
    })
}

fn source_binding(
    repository: &RepositoryIdentity,
    branch: &ObservedBranch,
    workspace_id: WorkspaceId,
    selection: ReviewSourceSelection,
) -> Result<SourceBinding, String> {
    Ok(SourceBinding {
        repository_id: RepositoryId::new(repository.id.as_str())
            .map_err(|error| error.to_string())?,
        branch_ref: BranchRef::new(branch.full_name.as_str()).map_err(|error| error.to_string())?,
        selection,
        workspace_id,
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

fn physical_object(object: &ObjectId) -> Result<GitCommitId, String> {
    GitCommitId::new(object.as_str().to_owned()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_plan_must_repeat_the_selected_source_identity() {
        let source = CreateBuildSourceInput::ExistingWorktree {
            association_id: "association-one".into(),
        };
        let mismatched = BuildWorkspacePlanInput::BorrowSelectedWorktree {
            association_id: "association-two".into(),
        };
        assert!(!matches!(
            (&source, &mismatched),
            (
                CreateBuildSourceInput::ExistingWorktree { association_id },
                BuildWorkspacePlanInput::BorrowSelectedWorktree {
                    association_id: planned
                }
            ) if association_id == planned
        ));
    }
}
