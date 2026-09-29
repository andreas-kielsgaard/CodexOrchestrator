use super::{TargetOperationAction, TargetOperationActionKind, TargetOperationActionStatus};
use crate::execution_targets::domain::{
    SessionExecutionSelection, SessionExecutionTarget, SessionWorkspaceSelection,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SelectionTargetRelation {
    Exact,
    SatisfiedByResolvedWorkspace,
    Different,
}

pub(crate) fn selection_target_relation(
    selection: &SessionExecutionSelection,
    target: Option<&SessionExecutionTarget>,
) -> SelectionTargetRelation {
    let Some(target) = target else {
        return SelectionTargetRelation::Different;
    };
    if selection.capability_profile_id != target.capability_profile_id
        || selection.capability_profile_revision != target.capability_profile_revision
        || selection.execution != target.execution
    {
        return SelectionTargetRelation::Different;
    }
    match &selection.workspace {
        SessionWorkspaceSelection::Existing { target: selected } if selected == target => {
            SelectionTargetRelation::Exact
        }
        SessionWorkspaceSelection::Auxiliary if target.repository_id.is_empty() => {
            SelectionTargetRelation::Exact
        }
        SessionWorkspaceSelection::Create {
            repository_id,
            branch_ref,
            commit,
            ..
        } if repository_id == &target.repository_id
            && branch_ref == &target.branch_ref
            && (commit.trim().is_empty() || target.head.as_deref() == Some(commit.as_str())) =>
        {
            SelectionTargetRelation::SatisfiedByResolvedWorkspace
        }
        _ => SelectionTargetRelation::Different,
    }
}

pub(crate) fn planned_actions(
    source: Option<&SessionExecutionTarget>,
    destination: &SessionExecutionSelection,
) -> Vec<TargetOperationAction> {
    use TargetOperationActionKind as Action;

    let mut kinds = vec![Action::PrepareDevice];
    match &destination.workspace {
        SessionWorkspaceSelection::Create { .. } => {
            kinds.extend([
                Action::ResolveRevision,
                Action::MaterializeWorktree,
                Action::VerifyWorktree,
            ]);
        }
        SessionWorkspaceSelection::Existing { .. } | SessionWorkspaceSelection::Auxiliary => {
            kinds.push(Action::VerifyWorktree);
        }
    }

    if requires_sister_migration(source, destination) {
        kinds.extend([
            Action::InspectSource,
            Action::InspectDestination,
            Action::CaptureSnapshot,
            Action::TransferSnapshot,
            Action::ApplySnapshot,
            Action::VerifyMigratedState,
            Action::ActivateSister,
        ]);
    }
    kinds.extend([Action::SelectProviderContext, Action::ActivateTarget]);
    kinds
        .into_iter()
        .map(|kind| TargetOperationAction {
            kind,
            status: TargetOperationActionStatus::Pending,
            detail: None,
            error: None,
        })
        .collect()
}

pub(crate) fn replan_actions(
    existing: &[TargetOperationAction],
    source: Option<&SessionExecutionTarget>,
    destination: &SessionExecutionSelection,
) -> Vec<TargetOperationAction> {
    let mut replanned = planned_actions(source, destination);
    for action in &mut replanned {
        if let Some(previous) = existing.iter().find(|candidate| {
            candidate.kind == action.kind
                && candidate.status == TargetOperationActionStatus::Completed
        }) {
            action.status = previous.status;
            action.detail = previous.detail.clone();
        }
    }
    replanned
}

fn requires_sister_migration(
    source: Option<&SessionExecutionTarget>,
    destination: &SessionExecutionSelection,
) -> bool {
    let Some(source) = source else { return false };
    let (repository_id, branch_ref) = match &destination.workspace {
        SessionWorkspaceSelection::Existing { target } => {
            (&target.repository_id, &target.branch_ref)
        }
        SessionWorkspaceSelection::Create {
            repository_id,
            branch_ref,
            ..
        } => (repository_id, branch_ref),
        SessionWorkspaceSelection::Auxiliary => return false,
    };
    !source.repository_id.is_empty()
        && &source.repository_id == repository_id
        && &source.branch_ref == branch_ref
        && source.execution.device_id != destination.execution.device_id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_targets::domain::{ExecutionBinding, ExecutionConnection};

    fn execution(device: &str) -> ExecutionBinding {
        ExecutionBinding {
            device_id: device.into(),
            device_name: device.into(),
            provider: "codex".into(),
            configuration_ref: "default".into(),
            connection: ExecutionConnection::Local,
        }
    }

    fn target(device: &str) -> SessionExecutionTarget {
        SessionExecutionTarget {
            capability_profile_id: "profile".into(),
            capability_profile_revision: 1,
            execution: execution(device),
            repository_id: "repo".into(),
            branch_ref: "refs/heads/main".into(),
            worktree_id: "tree".into(),
            path: "C:/repo".into(),
            head: Some("a".repeat(40)),
        }
    }

    fn create_selection(device: &str) -> SessionExecutionSelection {
        SessionExecutionSelection {
            capability_profile_id: "profile".into(),
            capability_profile_revision: 1,
            execution: execution(device),
            workspace: SessionWorkspaceSelection::Create {
                repository_id: "repo".into(),
                branch_ref: "refs/heads/main".into(),
                commit: "a".repeat(40),
                attachment: "existing_branch".into(),
            },
        }
    }

    #[test]
    fn resolved_create_selection_is_satisfied_by_active_target() {
        assert_eq!(
            selection_target_relation(&create_selection("local"), Some(&target("local"))),
            SelectionTargetRelation::SatisfiedByResolvedWorkspace
        );
    }

    #[test]
    fn same_device_creation_plans_only_workspace_and_activation_actions() {
        let kinds = planned_actions(Some(&target("local")), &create_selection("local"))
            .into_iter()
            .map(|action| action.kind)
            .collect::<Vec<_>>();
        assert_eq!(
            kinds,
            vec![
                TargetOperationActionKind::PrepareDevice,
                TargetOperationActionKind::ResolveRevision,
                TargetOperationActionKind::MaterializeWorktree,
                TargetOperationActionKind::VerifyWorktree,
                TargetOperationActionKind::SelectProviderContext,
                TargetOperationActionKind::ActivateTarget,
            ]
        );
    }

    #[test]
    fn remote_same_branch_adds_sister_migration_actions() {
        let kinds = planned_actions(Some(&target("source")), &create_selection("destination"))
            .into_iter()
            .map(|action| action.kind)
            .collect::<Vec<_>>();
        assert!(kinds.contains(&TargetOperationActionKind::CaptureSnapshot));
        assert!(kinds.contains(&TargetOperationActionKind::ActivateSister));
    }
}
