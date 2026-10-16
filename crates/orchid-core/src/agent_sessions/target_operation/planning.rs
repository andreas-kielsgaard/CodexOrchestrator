use super::{
    TargetOperationAction, TargetOperationActionKind, TargetOperationActionStatus,
    TargetSwitchFacts, TargetSwitchIntent, TargetSwitchRequest,
};
use crate::execution_targets::domain::{
    SessionExecutionSelection, SessionExecutionTarget, SessionWorkspaceSelection,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionTargetRelation {
    Exact,
    Different,
}

pub fn same_workspace_identity(
    left: &SessionExecutionTarget,
    right: &SessionExecutionTarget,
) -> bool {
    left.execution.device_id == right.execution.device_id
        && left.repository_id == right.repository_id
        && left.branch_ref == right.branch_ref
        && left.worktree_id == right.worktree_id
        && left.path == right.path
}

pub fn selection_target_relation(
    selection: &SessionExecutionSelection,
    target: Option<&SessionExecutionTarget>,
) -> SelectionTargetRelation {
    let Some(target) = target else {
        return SelectionTargetRelation::Different;
    };
    if selection.execution.device_id != target.execution.device_id {
        return SelectionTargetRelation::Different;
    }
    match &selection.workspace {
        SessionWorkspaceSelection::Existing { target: selected }
            if same_workspace_identity(selected, target) =>
        {
            SelectionTargetRelation::Exact
        }
        SessionWorkspaceSelection::Auxiliary if target.repository_id.is_empty() => {
            SelectionTargetRelation::Exact
        }
        _ => SelectionTargetRelation::Different,
    }
}

/// The same rules project future work and calculate the work at the queue head.
pub fn plan_target_switch(
    source: Option<&SessionExecutionTarget>,
    request: &TargetSwitchRequest,
    facts: &TargetSwitchFacts,
) -> Vec<TargetOperationAction> {
    use TargetOperationActionKind as A;
    if facts.destination_ready == Some(true)
        && selection_target_relation(&request.destination, source) == SelectionTargetRelation::Exact
    {
        return vec![];
    }
    let mut kinds = vec![];
    if request.destination.execution.is_remote() && facts.destination_ready != Some(true) {
        kinds.push(A::PrepareDevice);
    }
    match &request.destination.workspace {
        SessionWorkspaceSelection::Create { commit, .. } if facts.destination.is_none() => {
            if commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit()) {
                kinds.push(A::ResolveRevision);
            }
            kinds.push(A::MaterializeWorktree);
        }
        SessionWorkspaceSelection::Auxiliary
            if selection_target_relation(&request.destination, source)
                == SelectionTargetRelation::Different =>
        {
            kinds.push(A::MaterializeWorktree)
        }
        _ => (),
    }
    kinds.push(A::VerifyWorktree);
    if request.intent == TargetSwitchIntent::ContinueWork {
        kinds.extend([
            A::InspectSource,
            A::InspectDestination,
            A::CaptureSnapshot,
            A::TransferSnapshot,
            A::ApplySnapshot,
            A::VerifyMigratedState,
            A::ActivateSister,
        ]);
    }
    kinds.push(A::ActivateTarget);
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
