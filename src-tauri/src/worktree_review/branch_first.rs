use super::{
    association_observer::{observe_association, stable_association_id},
    branch_presentation::{
        associated_worktree_view, association_candidate_view, available_repository_readiness,
        baseline_view, commit_view as present_commit, registered_repository_view, repository_name,
        repository_readiness, repository_view_with_readiness, AssociateBaselineInput,
        AssociationCandidateView, BranchView, CommitView, WorkspaceOwnershipView,
        WorktreeAvailabilityView,
    },
    build_service::{CreateBuildInput, ReviewBuildCoordinator, ReviewBuildView},
    domain::{
        AssociationBaselineKind, BranchRef as DomainBranchRef, RepositoryId as DomainRepositoryId,
        WorkspaceId, WorkspaceOwnership, WorktreeAssociation, WorktreeAssociationId,
        WorktreeAssociationProvenance,
    },
    source_materialization::SourceMaterializationService,
    state::{CapabilityReadinessStatus, WorktreeReviewApplication},
    storage::{WorkspaceRepository, WorktreeAssociationRepository, WorktreeReviewDatabase},
};
use crate::repository_context::{
    BranchRef as ObservedBranch, BranchSummary, FullRefName, ObjectId, RepositoryContext,
    RepositoryIdentity, WorktreeLocation, WorktreeObservation,
};
#[cfg(test)]
use chrono::Utc;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Arc,
};

pub(crate) use super::branch_presentation::{
    AssociateWorktreeInput, AssociatedWorktreeView, BranchDetailView, ProductOverviewView,
};

pub(crate) struct BranchFirstReviewService {
    application: Arc<WorktreeReviewApplication>,
    database: Arc<WorktreeReviewDatabase>,
    builds: ReviewBuildCoordinator,
    sources: SourceMaterializationService,
    activity: super::worktree_activity::WorktreeActivityService,
    history: super::branch_history::BranchHistoryService,
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
            activity: Default::default(),
            history: Default::default(),
        })
    }

    pub(crate) fn overview(&self) -> Result<ProductOverviewView, String> {
        let application_overview = self.application.overview();
        let registered = self
            .application
            .registered_repositories()
            .map_err(|error| error.message)?;
        let selected_id = application_overview
            .selected_repository
            .as_ref()
            .map(|repository| repository.repository_id.clone());
        let Some(persisted) = application_overview.selected_repository.as_ref() else {
            return Ok(ProductOverviewView {
                repositories: registered
                    .iter()
                    .map(|repository| {
                        registered_repository_view(repository, available_repository_readiness())
                    })
                    .collect(),
                selected_repository_id: None,
                branches: Vec::new(),
                active_build_context: self.application.active_build_context(),
            });
        };
        let repository = match self.application.selected_repository() {
            Ok(repository) => repository,
            Err(_) => {
                return Ok(ProductOverviewView {
                    repositories: registered
                        .iter()
                        .map(|repository| {
                            let readiness = if repository.id.as_str() == persisted.repository_id {
                                repository_readiness(&application_overview.capabilities)
                            } else {
                                available_repository_readiness()
                            };
                            registered_repository_view(repository, readiness)
                        })
                        .collect(),
                    selected_repository_id: Some(persisted.repository_id.clone()),
                    branches: Vec::new(),
                    active_build_context: self.application.active_build_context(),
                });
            }
        };
        let context = self.context()?;
        let branches = self.branches(&context, &repository)?;
        Ok(ProductOverviewView {
            repositories: registered
                .iter()
                .map(|record| {
                    if record.id.as_str() == repository.id.as_str() {
                        repository_view_with_readiness(
                            &repository,
                            repository_readiness(&application_overview.capabilities),
                        )
                    } else {
                        registered_repository_view(record, available_repository_readiness())
                    }
                })
                .collect(),
            selected_repository_id: selected_id,
            branches,
            active_build_context: self.application.active_build_context(),
        })
    }

    pub(crate) fn select_repository(
        &self,
        repository_id: &str,
    ) -> Result<ProductOverviewView, String> {
        let selected = self.application.select_repository(repository_id);
        if selected.selection.status != CapabilityReadinessStatus::Ready {
            return Err(selected.selection.message);
        }
        self.overview()
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
        let observations = context
            .worktrees()
            .list(&repository.id, repository.top_level.path())
            .map_err(|error| error.to_string())?;
        let (worktrees, association_candidates) =
            self.worktrees_for_branch(&context, &repository, &branch, &observations)?;
        let branch_view = self.branch_summary_view(&repository, &summary, &observations)?;
        Ok(BranchDetailView {
            branch: branch_view,
            worktrees,
            association_candidates,
            builds: self
                .builds
                .list_for_branch(repository.id.as_str(), branch.full_name.as_str())?,
        })
    }

    pub(crate) fn target_detail(
        &self,
        target: super::domain::ReviewTarget,
    ) -> Result<BranchDetailView, String> {
        use super::domain::{CommitSourceContext, ReviewTarget};
        if let ReviewTarget::Branch {
            repository_id,
            branch_ref,
        } = &target
        {
            return self.branch_detail(repository_id, branch_ref);
        }
        let repository = self.repository(target.repository_id())?;
        let context = self.context()?;
        let inventory = self.branches(&context, &repository)?;
        let observations = context
            .worktrees()
            .list(&repository.id, repository.top_level.path())
            .map_err(|error| error.to_string())?;
        let (object, display_name, worktree_id) = match &target {
            ReviewTarget::Worktree { worktree_id, .. } => {
                let observation = observations
                    .iter()
                    .find(|item| item.id.as_str() == worktree_id)
                    .ok_or("The selected worktree is no longer available")?;
                let label = inventory
                    .iter()
                    .find(|item| item.target == target)
                    .map(|item| item.display_name.clone())
                    .unwrap_or_else(|| "Worktree".into());
                (observation.head.clone(), label, Some(worktree_id.as_str()))
            }
            ReviewTarget::Commit {
                object_id,
                context: source_context,
                ..
            } => {
                let tip = match source_context {
                    CommitSourceContext::Branch { tip_object_id, .. }
                    | CommitSourceContext::Worktree { tip_object_id, .. } => tip_object_id,
                };
                let object = ObjectId::parse(object_id).map_err(|error| error.to_string())?;
                if !context
                    .commits()
                    .is_ancestor(
                        repository.top_level.path(),
                        &object,
                        &ObjectId::parse(tip).map_err(|error| error.to_string())?,
                    )
                    .map_err(|error| error.to_string())?
                {
                    return Err("The selected commit is outside its pinned source history.".into());
                }
                (object, format!("Commit {}", &object_id[..8]), None)
            }
            ReviewTarget::Branch { .. } => unreachable!(),
        };
        let tip = load_commit_view(&context, repository.top_level.path(), &object)?;
        let worktrees = observations
            .iter()
            .filter(|item| matches!(item.location, WorktreeLocation::Available(_)))
            .filter(|item| {
                worktree_id
                    .map(|id| item.id.as_str() == id)
                    .unwrap_or(item.head == object)
            })
            .map(|item| self.physical_worktree_view(&context, &repository, item))
            .collect::<Result<Vec<_>, _>>()?;
        let branch = BranchView {
            target: target.clone(),
            repository_id: repository.id.as_str().into(),
            branch_ref: None,
            display_name,
            tip,
            ahead_of_default: 0,
            behind_default: 0,
            associated_worktree_count: worktrees.len(),
            available_worktree_count: worktrees.len(),
            worktree_ids: worktrees
                .iter()
                .map(|item| item.worktree_id.clone())
                .collect(),
            activity: None,
        };
        Ok(BranchDetailView {
            branch,
            worktrees,
            association_candidates: Vec::new(),
            builds: self.builds.list_for_target(&target)?,
        })
    }

    fn physical_worktree_view(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        observation: &WorktreeObservation,
    ) -> Result<AssociatedWorktreeView, String> {
        use super::branch_presentation::{BaselineView, WorktreeChangesView};
        let path = available_path(observation)?;
        let head = load_commit_view(context, repository.top_level.path(), &observation.head)?;
        let status = context
            .status()
            .status(path)
            .map_err(|error| error.to_string())?;
        let id = super::domain::WorktreeId::new(observation.id.as_str())
            .map_err(|error| error.to_string())?;
        let workspaces = self
            .database
            .workspaces()
            .list_for_worktree(&id)
            .map_err(|error| error.to_string())?;
        let ownership = workspace_ownership_view(
            WorktreeAssociationProvenance::GitAttachedBranch,
            workspaces.iter().map(|workspace| &workspace.ownership),
        )?;
        Ok(AssociatedWorktreeView {
            association_id: None,
            worktree_id: id.as_str().into(),
            branch_ref: observation
                .head_ref
                .as_ref()
                .map(|reference| reference.as_str().into()),
            name: repository_name(path),
            location_label: path.to_string_lossy().into_owned(),
            provenance: "physical_worktree".into(),
            ownership,
            baseline: BaselineView::ObservedAtAssociation {
                commit: head.clone(),
            },
            current_head: head,
            changes: WorktreeChangesView {
                commits_ahead_of_baseline: 0,
                commits_behind_baseline: 0,
                staged_files: status.staged_paths as u32,
                unstaged_files: status.unstaged_paths as u32,
                untracked_files: status.untracked_paths as u32,
            },
            detached_head: observation.head_ref.is_none(),
            branch_reachability: "unassociated".into(),
            availability: WorktreeAvailabilityView::Available,
        })
    }

    pub(crate) fn commit_history(
        &self,
        query: super::branch_history::CommitHistoryQuery,
        cursor: Option<&str>,
        page_size: usize,
    ) -> Result<super::branch_history::CommitHistoryPageView, String> {
        let repository = self.repository(query.target.repository_id())?;
        self.history
            .history(&self.context()?, &repository, query, cursor, page_size)
    }

    pub(crate) fn branch_graph(
        &self,
        repository_id: &str,
        limit: usize,
        snapshot_id: Option<&str>,
    ) -> Result<super::branch_graph::BranchGraphView, String> {
        let repository = self.repository(repository_id)?;
        let context = self.context()?;
        self.history
            .graph(&context, &repository, limit, snapshot_id, || {
                self.branches(&context, &repository)
            })
    }

    pub(crate) fn worktree_activity(
        &self,
        repository_id: &str,
        ids: &[String],
    ) -> Result<Vec<super::worktree_activity::WorktreeActivityView>, String> {
        self.activity
            .discover(&self.context()?, &self.repository(repository_id)?, ids)
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
        super::branch_inventory::inventory(context, repository, &self.database, &self.activity)
    }

    fn branch_summary_view(
        &self,
        repository: &RepositoryIdentity,
        summary: &BranchSummary,
        _observations: &[WorktreeObservation],
    ) -> Result<BranchView, String> {
        self.branches(&self.context()?, repository)?
            .into_iter()
            .find(|view| view.branch_ref.as_deref() == Some(summary.branch.full_name.as_str()))
            .ok_or_else(|| "The selected branch is no longer available.".into())
    }

    fn worktrees_for_branch(
        &self,
        context: &RepositoryContext,
        repository: &RepositoryIdentity,
        branch: &ObservedBranch,
        observations: &[WorktreeObservation],
    ) -> Result<(Vec<AssociatedWorktreeView>, Vec<AssociationCandidateView>), String> {
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
            .collect::<HashSet<_>>();
        let attached_observations = observations
            .iter()
            .filter(|worktree| matches!(worktree.location, WorktreeLocation::Available(_)))
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
            .collect::<HashSet<_>>();
        let candidates = observations
            .iter()
            .filter(|worktree| matches!(worktree.location, WorktreeLocation::Available(_)))
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
        let location_label = observation
            .map(|item| match &item.location {
                WorktreeLocation::Available(directory) => {
                    directory.path().to_string_lossy().into_owned()
                }
                WorktreeLocation::Unavailable(path) => path.to_string_lossy().into_owned(),
            })
            .unwrap_or_else(|| association.location.as_str().into());
        let name = repository_name(Path::new(&location_label));
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
        let refreshed = observation
            .filter(|item| matches!(item.location, WorktreeLocation::Available(_)))
            .and_then(|item| {
                find_branch(context, repository, association.branch_ref.as_str())
                    .ok()
                    .map(|branch| (item, branch))
            })
            .map(|(item, branch)| {
                observe_association(
                    context,
                    repository,
                    &branch,
                    item,
                    association.id.clone(),
                    association.provenance,
                    ObjectId::parse(baseline_object.as_str()).map_err(|error| error.to_string())?,
                    association.baseline.kind,
                )
            })
            .transpose()?;
        if let Some(refreshed) = &refreshed {
            self.database
                .associations()
                .save(refreshed)
                .map_err(|error| error.to_string())?;
        }
        let current_association = refreshed.as_ref().unwrap_or(association);
        let availability =
            super::branch_presentation::worktree_availability(current_association, observation);
        Ok(associated_worktree_view(
            current_association,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_review::{
        branch_presentation::CapabilityAvailabilityView,
        domain::{RepositoryId, ReviewBuildId, ReviewRepository},
        state::{
            CapabilityReadinessStatus, CapabilityReadinessView, WorktreeReviewCapabilitiesView,
        },
    };

    #[test]
    fn persisted_unavailable_repository_remains_visible_with_typed_readiness() {
        let repository = ReviewRepository {
            id: RepositoryId::new("repository-stable-id").unwrap(),
            label: "missing".into(),
            anchor_root: "C:/repositories/missing".into(),
            common_directory: "C:/repositories/missing/.git".into(),
            first_seen_at: Utc::now(),
            last_seen_at: Utc::now(),
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

        let view = registered_repository_view(&repository, repository_readiness(&capabilities));

        assert_eq!(view.repository_id, repository.id.as_str());
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
