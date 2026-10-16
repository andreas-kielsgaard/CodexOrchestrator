use super::*;
use crate::execution_targets::domain::*;

fn target(device: &str) -> SessionExecutionTarget {
    SessionExecutionTarget {
        capability_profile_id: "profile".into(),
        capability_profile_revision: 1,
        execution: ExecutionBinding {
            device_id: device.into(),
            device_name: device.into(),
            provider: "codex".into(),
            configuration_ref: "default".into(),
            connection: ExecutionConnection::Local,
        },
        repository_id: "repo".into(),
        branch_ref: "refs/heads/main".into(),
        worktree_id: "tree".into(),
        path: "/repo/tree".into(),
        head: Some("a".repeat(40)),
    }
}
fn selection(target: SessionExecutionTarget) -> SessionExecutionSelection {
    SessionExecutionSelection {
        capability_profile_id: target.capability_profile_id.clone(),
        capability_profile_revision: target.capability_profile_revision,
        execution: target.execution.clone(),
        workspace: SessionWorkspaceSelection::Existing { target },
    }
}
#[test]
fn workspace_identity_ignores_head_provider_configuration_and_display_name() {
    let source = target("local");
    let mut destination = selection(source.clone());
    destination.execution.provider = "claude".into();
    destination.execution.configuration_ref = "other".into();
    destination.execution.device_name = "renamed".into();
    destination.capability_profile_id = "other-profile".into();
    destination.capability_profile_revision = 9;
    let SessionWorkspaceSelection::Existing { target } = &mut destination.workspace else {
        unreachable!()
    };
    target.head = None;
    assert_eq!(
        selection_target_relation(&destination, Some(&source)),
        SelectionTargetRelation::Exact
    );
    let request = TargetSwitchRequest::retarget(destination);
    assert!(plan_target_switch(
        Some(&source),
        &request,
        &TargetSwitchFacts {
            destination_ready: Some(true),
            ..Default::default()
        }
    )
    .is_empty());
}
#[test]
fn retargeting_same_branch_on_another_device_never_plans_migration() {
    let source = target("source");
    let request = TargetSwitchRequest::retarget(selection(target("destination")));
    let actions = plan_target_switch(Some(&source), &request, &TargetSwitchFacts::default());
    assert!(!actions.iter().any(|a| matches!(
        a.kind,
        TargetOperationActionKind::CaptureSnapshot | TargetOperationActionKind::ActivateSister
    )));
    assert!(actions
        .iter()
        .any(|a| a.kind == TargetOperationActionKind::ActivateTarget));
}
#[test]
fn continuing_work_requires_resolution_and_plans_existing_migration_steps() {
    let source = target("source");
    let mut request = TargetSwitchRequest::retarget(selection(target("destination")));
    request.intent = TargetSwitchIntent::ContinueWork;
    assert!(request.validate().is_err());
    request.migration_resolution = Some(MigrationResolution {
        strategy: MigrationStrategy::LinkExisting,
        repository_id: source.repository_id.clone(),
        branch_ref: source.branch_ref.clone(),
    });
    assert!(request.validate().is_ok());
    assert!(
        plan_target_switch(Some(&source), &request, &TargetSwitchFacts::default())
            .iter()
            .any(|a| a.kind == TargetOperationActionKind::CaptureSnapshot)
    );
}
#[test]
fn create_selection_is_never_satisfied_by_an_unrelated_same_branch_instance() {
    let source = target("local");
    let mut destination = selection(source.clone());
    destination.workspace = SessionWorkspaceSelection::Create {
        repository_id: source.repository_id.clone(),
        branch_ref: source.branch_ref.clone(),
        commit: source.head.clone().unwrap(),
        attachment: "branch".into(),
    };
    assert_eq!(
        selection_target_relation(&destination, Some(&source)),
        SelectionTargetRelation::Different
    );
}
