use super::{
    association_observer::{observe_association, stable_association_id},
    branch_presentation::{
        associated_worktree_view, association_candidate_view, baseline_view, branch_view,
        commit_view as present_commit, persisted_repository_view, repository_name,
        repository_readiness, repository_view, repository_view_with_readiness,
        AssociateBaselineInput, AssociationCandidateView, BranchView, CommitView,
        WorkspaceOwnershipView, WorktreeAvailabilityView,
    },
    build_service::{CreateBuildInput, ReviewBuildCoordinator, ReviewBuildView},
    domain::{
        AssociationBaselineKind, BranchRef as DomainBranchRef, GitObjectId,
        RepositoryId as DomainRepositoryId, ReviewBranch as StoredBranch,
        ReviewRepository as StoredRepository, WorkspaceId, WorkspaceOwnership, WorktreeAssociation,
        WorktreeAssociationId, WorktreeAssociationLifecycle, WorktreeAssociationProvenance,
    },
    source_materialization::SourceMaterializationService,
    state::WorktreeReviewApplication,
    storage::{
        ReviewRepositoryRepository, WorkspaceRepository, WorktreeAssociationRepository,
        WorktreeReviewDatabase,
    },
};
use crate::repository_context::{
    BranchRef as ObservedBranch, BranchSummary, FullRefName, ObjectId, RepositoryContext,
    RepositoryIdentity, WorktreeLocation, WorktreeObservation,
};
use chrono::Utc;
use std::{collections::HashMap, path::Path, sync::Arc};

pub(crate) use super::branch_presentation::{
    AssociateWorktreeInput, AssociatedWorktreeView, BranchDetailView, BranchHistoryPageView,
    ProductOverviewView,
};

pub(crate) struct BranchFirstReviewService {
    application: Arc<WorktreeReviewApplication>,
    database: Arc<WorktreeReviewDatabase>,
    builds: ReviewBuildCoordinator,
    sources: SourceMaterializationService,
}

impl BranchFirstReviewService {
    pub(crate) fn open(application: Arc<WorktreeReviewApplication>) -> Result<Self, String> {
        let database = application.database().map_err(|error| error.message)?;
        let sources = SourceMaterializationService::new(
            database.clone(),
            application.review_root().to_path_buf(),
        );
        let builds = ReviewBuildCoordinator::open(application.clone())?;
        Ok(Self {
            application,
            database,
            builds,
            sources,
        })
    }

    pub(crate) fn overview(&self) -> Result<ProductOverviewView, String> {
        let application_overview = self.application.overview();
        let Some(persisted) = application_overview.selected_repository.as_ref() else {
            return Ok(ProductOverviewView {
                repositories: Vec::new(),
                selected_repository_id: None,
                branches: Vec::new(),
                active_build_context: self.application.active_build_context(),
            });
        };
        let repository = match self.application.selected_repository() {
            Ok(repository) => repository,
            Err(_) => {
                return Ok(ProductOverviewView {
                    repositories: vec![persisted_repository_view(
                        persisted,
                        &application_overview.capabilities,
                    )],
                    selected_repository_id: Some(persisted.repository_id.clone()),
                    branches: Vec::new(),
                    active_build_context: self.application.active_build_context(),
                });
            }
        };
        let context = self.context()?;
        let branches = self.branches(&context, &repository)?;
        Ok(ProductOverviewView {
            repositories: vec![repository_view_with_readiness(
                &repository,
                repository_readiness(&application_overview.capabilities),
            )],
            selected_repository_id: Some(repository.id.as_str().to_owned()),
            branches,
            active_build_context: self.application.active_build_context(),
        })
    }

    pub(crate) fn select_repository(
        &self,
        repository_id: &str,
    ) -> Result<ProductOverviewView, String> {
        let selected = self.repository(repository_id)?;
        let context = self.context()?;
        Ok(ProductOverviewView {
            repositories: vec![repository_view(&selected)],
            selected_repository_id: Some(selected.id.as_str().to_owned()),
            branches: self.branches(&context, &selected)?,
            active_build_context: self.application.active_build_context(),
        })
    }

    pub(crate) fn branch_detail(
        &self,
        repository_id: &str,
        branch_ref: &str,
    ) -> Result<BranchDetailView, String> {
        let repository = self.repository(repository_id)?;
        let context = self.context()?;
        let summary = find_branch_summary(&context, &repository, branch_ref)?;
        let branch = summary.branch.clone();
        let branch_view = self.branch_summary_view(&repository, &summary)?;
        let (worktrees, association_candidates) =
            self.worktrees_for_branch(&context, &repository, &branch)?;
        Ok(BranchDetailView {
            branch: branch_view,
            worktrees,
            association_candidates,
            builds: self
                .builds
                .list_for_branch(repository.id.as_str(), branch.full_name.as_str())?,
        })
    }

    pub(crate) fn branch_history(
        &self,
        repository_id: &str,
        branch_ref: &str,
        cursor: Option<&str>,
        page_size: usize,
    ) -> Result<BranchHistoryPageView, String> {
        let repository = self.repository(repository_id)?;
        let context = self.context()?;
        let branch = find_branch(&context, &repository, branch_ref)?;
        let cursor = cursor
            .map(ObjectId::parse)
            .transpose()
            .map_err(|error| error.to_string())?;
        if let Some(cursor) = cursor.as_ref() {
            if !context
                .commits()
                .is_ancestor(repository.top_level.path(), cursor, &branch.object_id)
                .map_err(|error| error.to_string())?
            {
                return Err("The history cursor does not belong to the selected branch.".into());
            }
        }
        let (commits, next_cursor) = context
            .commits()
            .first_parent_history_page(
                repository.top_level.path(),
                &branch.full_name,
                cursor.as_ref(),
                page_size,
            )
            .map_err(|error| error.to_string())?;
        Ok(BranchHistoryPageView {
            commits: commits.into_iter().map(present_commit).collect(),
            next_cursor: next_cursor.map(|object| object.as_str().to_owned()),
        })
    }

    pub(crate) fn create_build(&self, input: CreateBuildInput) -> Result<ReviewBuildView, String> {
        self.builds.create_build(input)
    }

    pub(crate) fn open_build(&self, build_id: &str) -> Result<(), String> {
        self.builds.open_build(build_id)
    }

    pub(crate) fn associate_worktree(
        &self,
        input: AssociateWorktreeInput,
    ) -> Result<AssociatedWorktreeView, String> {
        let repository = self.repository(&input.repository_id)?;
        let context = self.context()?;
        let branch = find_branch(&context, &repository, &input.branch_ref)?;
        let observation = context
            .worktrees()
            .list(&repository.id, repository.top_level.path())
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|worktree| worktree.id.as_str() == input.worktree_id)
            .ok_or_else(|| "The selected worktree checkout is unavailable.".to_string())?;
        if observation
            .head_ref
            .as_ref()
            .is_some_and(|reference| reference != &branch.full_name)
        {
            return Err("A worktree attached to another branch cannot be associated here.".into());
        }
        if !context
            .commits()
            .is_ancestor(
                repository.top_level.path(),
                &observation.head,
                &branch.object_id,
            )
            .map_err(|error| error.to_string())?
        {
            return Err("The worktree HEAD does not belong to the selected branch.".into());
        }
        let (baseline, provenance, baseline_kind) = match input.baseline {
            AssociateBaselineInput::ObservedCurrentHead => (
                observation.head.clone(),
                if observation.head_ref.is_some() {
                    WorktreeAssociationProvenance::GitAttachedBranch
                } else {
                    WorktreeAssociationProvenance::UserAssociatedDetached
                },
                AssociationBaselineKind::ObservedAtAssociation,
            ),
            AssociateBaselineInput::SelectedCommit { object_id } => {
                let baseline = ObjectId::parse(object_id).map_err(|error| error.to_string())?;
                if !context
                    .commits()
                    .is_ancestor(repository.top_level.path(), &baseline, &branch.object_id)
                    .map_err(|error| error.to_string())?
                {
                    return Err("The selected baseline does not belong to this branch.".into());
                }
                (
                    baseline,
                    WorktreeAssociationProvenance::UserAssociatedDetached,
                    AssociationBaselineKind::UserSupplied,
                )
            }
        };
        let association = observe_association(
            &context,
            &repository,
            &branch,
            &observation,
            WorktreeAssociationId::new(stable_association_id(
                repository.id.as_str(),
                branch.full_name.as_str(),
                observation.id.as_str(),
            ))
            .map_err(|error| error.to_string())?,
            provenance,
            baseline,
            baseline_kind,
        )?;
        self.database
            .associations()
            .save(&association)
            .map_err(|error| error.to_string())?;
        self.association_view(&context, &repository, &association, Some(&observation))
    }

    pub(crate) fn create_worktree(
        &self,
        repository_id: &str,
        branch_ref: &str,
    ) -> Result<AssociatedWorktreeView, String> {
        let repository = self.repository(repository_id)?;
        let context = self.context()?;
        let branch = find_branch(&context, &repository, branch_ref)?;
        let association_id = WorktreeAssociationId::random();
        let workspace_id = WorkspaceId::random();
        let workspace = self.sources.plan_workspace(
            &repository,
            workspace_id.clone(),
            WorkspaceOwnership::ManagedBranchWorktree {
                association_id: association_id.clone(),
            },
        )?;
        self.database
            .workspaces()
            .save(&workspace)
            .map_err(|error| error.to_string())?;
        let materialized =
            self.sources
                .materialize_branch_workspace(&context, &repository, &branch, workspace)?;
        let association = observe_association(
            &context,
            &repository,
            &branch,
            &materialized.observation,
            association_id.clone(),
            WorktreeAssociationProvenance::ProductCreated,
            branch.object_id.clone(),
            AssociationBaselineKind::CreatedAtObject,
        )?;
        self.database
            .transaction(|transaction| {
                transaction.workspaces().save(&materialized.workspace)?;
                transaction.associations().save(&association)
            })
            .map_err(|error| error.to_string())?;
        self.association_view(
            &context,
            &repository,
            &association,
            Some(&materialized.observation),
        )
    }

    fn context(&self) -> Result<RepositoryContext, String> {
        self.application
            .repository_context()
            .map_err(|error| error.message)
    }

    fn repository(&self, repository_id: &str) -> Result<RepositoryIdentity, String> {
        let repository = self
            .application
            .selected_repository()
            .map_err(|error| error.message)?;
        if repository.id.as_str() != repository_id {
            return Err("The repository selection changed; refresh Worktree Review.".into());
        }
        Ok(repository)
    }

    fn branches(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
    ) -> Result<Vec<BranchView>, String> {
        let now = Utc::now();
        let domain_repository_id = domain_repository_id(repository)?;
        self.database
            .repositories()
            .save_repository(&StoredRepository {
                id: domain_repository_id.clone(),
                label: repository_name(repository.top_level.path()),
                first_seen_at: now,
                last_seen_at: now,
            })
            .map_err(|error| error.to_string())?;
        let default = context
            .references()
            .remote_default_branch(repository.top_level.path())
            .map_err(|error| error.to_string())?;
        context
            .references()
            .local_branch_summaries(repository.top_level.path(), default.as_ref())
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|summary| {
                self.database
                    .repositories()
                    .save_branch(&StoredBranch {
                        repository_id: domain_repository_id.clone(),
                        branch_ref: domain_branch_ref(&summary.branch.full_name)?,
                        observed_tip: domain_object_id(&summary.branch.object_id)?,
                        observed_at: now,
                    })
                    .map_err(|error| error.to_string())?;
                self.branch_summary_view(repository, &summary)
            })
            .collect()
    }

    fn branch_summary_view(
        &self,
        repository: &RepositoryIdentity,
        summary: &BranchSummary,
    ) -> Result<BranchView, String> {
        let repository_id = domain_repository_id(repository)?;
        let branch_ref = domain_branch_ref(&summary.branch.full_name)?;
        let associated_worktree_count = self
            .database
            .associations()
            .list_for_branch(&repository_id, &branch_ref)
            .map_err(|error| error.to_string())?
            .into_iter()
            .filter(|association| association.lifecycle == WorktreeAssociationLifecycle::Active)
            .count();
        Ok(branch_view(
            repository,
            &summary.branch,
            present_commit(summary.tip.clone()),
            summary.ahead,
            summary.behind,
            associated_worktree_count,
        ))
    }

    fn worktrees_for_branch(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        branch: &ObservedBranch,
    ) -> Result<(Vec<AssociatedWorktreeView>, Vec<AssociationCandidateView>), String> {
        let observations = context
            .worktrees()
            .list(&repository.id, repository.top_level.path())
            .map_err(|error| error.to_string())?;
        let by_id = observations
            .iter()
            .map(|worktree| (worktree.id.as_str(), worktree))
            .collect::<HashMap<_, _>>();
        let repository_id = domain_repository_id(repository)?;
        let branch_id = domain_branch_ref(&branch.full_name)?;
        let mut associations = self
            .database
            .associations()
            .list_for_branch(&repository_id, &branch_id)
            .map_err(|error| error.to_string())?;
        let existing_ids = associations
            .iter()
            .map(|association| association.worktree_id.as_str().to_owned())
            .collect::<std::collections::HashSet<_>>();
        let attached_observations = observations
            .iter()
            .filter(|worktree| worktree.head_ref.as_ref() == Some(&branch.full_name))
            .filter(|worktree| !existing_ids.contains(worktree.id.as_str()))
            .collect::<Vec<_>>();
        for observation in attached_observations {
            let association = observe_association(
                context,
                repository,
                branch,
                observation,
                WorktreeAssociationId::new(stable_association_id(
                    repository.id.as_str(),
                    branch.full_name.as_str(),
                    observation.id.as_str(),
                ))
                .map_err(|error| error.to_string())?,
                WorktreeAssociationProvenance::GitAttachedBranch,
                observation.head.clone(),
                AssociationBaselineKind::ObservedAtAssociation,
            )?;
            self.database
                .associations()
                .save(&association)
                .map_err(|error| error.to_string())?;
            associations.push(association);
        }
        let worktrees = associations
            .iter()
            .map(|association| {
                self.association_view(
                    context,
                    repository,
                    association,
                    by_id.get(association.worktree_id.as_str()).copied(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let associated = associations
            .iter()
            .map(|association| association.worktree_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let candidates = observations
            .iter()
            .filter(|worktree| worktree.head_ref.is_none())
            .filter(|worktree| !associated.contains(worktree.id.as_str()))
            .filter(|worktree| {
                context
                    .commits()
                    .is_ancestor(
                        repository.top_level.path(),
                        &worktree.head,
                        &branch.object_id,
                    )
                    .unwrap_or(false)
            })
            .map(|worktree| association_candidate(context, repository, worktree))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((worktrees, candidates))
    }

    fn association_view(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        association: &WorktreeAssociation,
        observation: Option<&WorktreeObservation>,
    ) -> Result<AssociatedWorktreeView, String> {
        let current = observation
            .map(|worktree| worktree.head.clone())
            .unwrap_or_else(|| ObjectId::parse(association.observed_state.head.as_str()).unwrap());
        let baseline_object = association
            .baseline
            .object_id
            .as_ref()
            .ok_or_else(|| "The worktree association baseline is unverified.".to_string())?;
        let baseline = load_commit_view(
            context,
            repository.top_level.path(),
            &ObjectId::parse(baseline_object.as_str()).map_err(|error| error.to_string())?,
        )?;
        let baseline = baseline_view(association.baseline.kind, baseline)?;
        let (name, location_label, availability) = match observation {
            Some(observation) => {
                let path = available_path(observation)?;
                (
                    repository_name(path),
                    path.to_string_lossy().into_owned(),
                    if association.lifecycle == WorktreeAssociationLifecycle::BranchMismatch {
                        WorktreeAvailabilityView::BranchMismatch {
                            detail: "The checkout no longer matches its associated branch.".into(),
                        }
                    } else {
                        WorktreeAvailabilityView::Available
                    },
                )
            }
            None => (
                "Missing worktree".into(),
                association.location.as_str().into(),
                WorktreeAvailabilityView::Missing {
                    detail: "Git no longer reports this exact worktree checkout.".into(),
                },
            ),
        };
        let workspace_ownership = self
            .database
            .workspaces()
            .list_for_worktree(&association.worktree_id)
            .map_err(|error| error.to_string())?;
        let ownership = workspace_ownership_view(
            association.provenance,
            workspace_ownership
                .iter()
                .map(|workspace| &workspace.ownership),
        )?;
        Ok(associated_worktree_view(
            association,
            observation,
            name,
            location_label,
            ownership,
            availability,
            baseline,
            load_commit_view(context, repository.top_level.path(), &current)?,
        ))
    }
}

fn workspace_ownership_view<'a>(
    provenance: WorktreeAssociationProvenance,
    ownerships: impl IntoIterator<Item = &'a WorkspaceOwnership>,
) -> Result<WorkspaceOwnershipView, String> {
    let mut managed = false;
    let mut build_owned = false;
    for ownership in ownerships {
        managed |= matches!(ownership, WorkspaceOwnership::ManagedBranchWorktree { .. });
        build_owned |= matches!(ownership, WorkspaceOwnership::OwnedBuildWorktree { .. });
    }
    if managed {
        Ok(WorkspaceOwnershipView::ManagedBranchWorktree)
    } else if build_owned {
        Ok(WorkspaceOwnershipView::OwnedBuildWorktree)
    } else if provenance == WorktreeAssociationProvenance::ProductCreated {
        Err("The product-created worktree has no durable ownership record.".into())
    } else {
        Ok(WorkspaceOwnershipView::BorrowedExternal)
    }
}

fn find_branch(
    context: &RepositoryContext,
    repository: &RepositoryIdentity,
    branch_ref: &str,
) -> Result<ObservedBranch, String> {
    let full = FullRefName::parse(branch_ref).map_err(|error| error.to_string())?;
    if full.branch_name().is_none() {
        return Err("Worktree Review requires a full local branch ref.".into());
    }
    context
        .references()
        .local_branches(repository.top_level.path())
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|branch| branch.full_name == full)
        .ok_or_else(|| "The selected branch is unavailable.".into())
}

fn find_branch_summary(
    context: &RepositoryContext,
    repository: &RepositoryIdentity,
    branch_ref: &str,
) -> Result<BranchSummary, String> {
    let full = FullRefName::parse(branch_ref).map_err(|error| error.to_string())?;
    if full.branch_name().is_none() {
        return Err("Worktree Review requires a full local branch ref.".into());
    }
    let default = context
        .references()
        .remote_default_branch(repository.top_level.path())
        .map_err(|error| error.to_string())?;
    context
        .references()
        .local_branch_summaries(repository.top_level.path(), default.as_ref())
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|summary| summary.branch.full_name == full)
        .ok_or_else(|| "The selected branch is unavailable.".into())
}

fn load_commit_view(
    context: &RepositoryContext,
    root: &Path,
    object: &ObjectId,
) -> Result<CommitView, String> {
    context
        .commits()
        .facts(root, object)
        .map(present_commit)
        .map_err(|error| error.to_string())
}

fn association_candidate(
    context: &RepositoryContext,
    repository: &RepositoryIdentity,
    worktree: &WorktreeObservation,
) -> Result<AssociationCandidateView, String> {
    let path = available_path(worktree)?;
    Ok(association_candidate_view(
        worktree,
        path,
        load_commit_view(context, repository.top_level.path(), &worktree.head)?,
    ))
}

fn available_path(worktree: &WorktreeObservation) -> Result<&Path, String> {
    match &worktree.location {
        WorktreeLocation::Available(directory) => Ok(directory.path()),
        WorktreeLocation::Unavailable(_) => Err("The worktree checkout is unavailable.".into()),
    }
}

fn domain_repository_id(repository: &RepositoryIdentity) -> Result<DomainRepositoryId, String> {
    DomainRepositoryId::new(repository.id.as_str()).map_err(|error| error.to_string())
}

fn domain_branch_ref(reference: &FullRefName) -> Result<DomainBranchRef, String> {
    DomainBranchRef::new(reference.as_str()).map_err(|error| error.to_string())
}

fn domain_object_id(object: &ObjectId) -> Result<GitObjectId, String> {
    GitObjectId::new(object.as_str()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_review::{
        branch_presentation::CapabilityAvailabilityView,
        domain::ReviewBuildId,
        state::{
            CapabilityReadinessStatus, CapabilityReadinessView, SelectedRepositoryView,
            WorktreeReviewCapabilitiesView,
        },
    };

    #[test]
    fn persisted_unavailable_repository_remains_visible_with_typed_readiness() {
        let repository = SelectedRepositoryView {
            repository_id: "repository-stable-id".into(),
            root: "C:/repositories/missing".into(),
        };
        let capabilities = WorktreeReviewCapabilitiesView {
            repository_browsing: CapabilityReadinessView {
                status: CapabilityReadinessStatus::RepositoryUnavailable,
                message: "The saved repository is unavailable.".into(),
            },
            build_runtime: CapabilityReadinessView {
                status: CapabilityReadinessStatus::NotEvaluated,
                message: "Build tooling is checked on demand.".into(),
            },
            build_output_storage: CapabilityReadinessView {
                status: CapabilityReadinessStatus::Ready,
                message: "Build output storage is ready.".into(),
            },
        };

        let view = persisted_repository_view(&repository, &capabilities);

        assert_eq!(view.repository_id, repository.repository_id);
        assert_eq!(view.readiness.state, "unavailable");
        assert_eq!(
            view.readiness.browse,
            CapabilityAvailabilityView::Unavailable {
                reason: "The saved repository is unavailable.".into()
            }
        );
        assert_eq!(
            view.readiness.build_output_storage,
            CapabilityAvailabilityView::Available
        );
    }

    #[test]
    fn physical_worktree_ownership_comes_from_durable_workspaces_not_association_labels() {
        let build = ReviewBuildId::new("build-one").unwrap();
        let association = WorktreeAssociationId::new("association-one").unwrap();
        let build_owned = WorkspaceOwnership::OwnedBuildWorktree { build_id: build };
        let managed = WorkspaceOwnership::ManagedBranchWorktree {
            association_id: association,
        };

        assert_eq!(
            workspace_ownership_view(
                WorktreeAssociationProvenance::UserAssociatedDetached,
                [&build_owned]
            )
            .unwrap(),
            WorkspaceOwnershipView::OwnedBuildWorktree
        );
        assert_eq!(
            workspace_ownership_view(
                WorktreeAssociationProvenance::ProductCreated,
                [&build_owned, &managed]
            )
            .unwrap(),
            WorkspaceOwnershipView::ManagedBranchWorktree
        );
        assert!(workspace_ownership_view(
            WorktreeAssociationProvenance::ProductCreated,
            std::iter::empty()
        )
        .is_err());
    }
}
