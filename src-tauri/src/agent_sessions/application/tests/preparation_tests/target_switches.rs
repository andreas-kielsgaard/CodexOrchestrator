use super::*;
use crate::agent_sessions::{
    queue::{SessionQueueEntryItem, SessionQueueEntryState},
    target_operation::*,
};

fn local_selection(fixture: &PreparationFixture, name: &str) -> SessionExecutionSelection {
    let mut selection = fixture.selection.clone();
    let SessionWorkspaceSelection::Existing { target } = &mut selection.workspace else {
        unreachable!()
    };
    let path = fixture._directory.path().join(name);
    std::fs::create_dir(&path).unwrap();
    target.path = path.to_string_lossy().into_owned();
    target.worktree_id = name.into();
    selection
}

#[test]
fn an_empty_queue_holds_target_changes_until_a_message_or_explicit_run() {
    for manual in [false, true] {
        let fixture = PreparationFixture::new();
        let queue = fixture
            .app
            .enqueue_session_target_operation(
                &fixture.session.id,
                TargetSwitchRequest::retarget(local_selection(&fixture, "deferred-switch")),
                None,
            )
            .unwrap();
        let entry = &queue.entries[0];
        let SessionQueueEntryItem::TargetOperation { operation_id } = &entry.item else {
            unreachable!()
        };
        fixture
            .app
            .dispatch_session_queue(&fixture.session.id)
            .unwrap();
        assert_eq!(
            fixture
                .repository
                .target_operation(operation_id)
                .unwrap()
                .unwrap()
                .phase,
            TargetOperationPhase::Pending
        );
        assert_eq!(fixture.runtime.attempts(), 0);
        if manual {
            fixture
                .app
                .run_target_operation_now(&fixture.session.id, &entry.id)
                .unwrap();
        } else {
            fixture
                .app
                .accept_prepared_message(fixture.input("after-deferred-switch"))
                .unwrap();
        }
        fixture.wait(|| {
            fixture
                .repository
                .target_operation(operation_id)
                .unwrap()
                .is_some_and(|op| op.phase == TargetOperationPhase::Completed)
        });
        if manual {
            assert_eq!(fixture.runtime.attempts(), 0);
        } else {
            fixture.wait(|| fixture.runtime.attempts() == 1);
        }
    }
}

#[test]
fn future_switch_waits_for_native_preparation_and_records_actual_execution_order() {
    let fixture = PreparationFixture::new();
    let first = fixture
        .app
        .accept_prepared_message(fixture.input("first-turn"))
        .unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    let snapshot = fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest::retarget(local_selection(&fixture, "second-workspace")),
            None,
        )
        .unwrap();
    let id = snapshot
        .entries
        .iter()
        .filter_map(|e| match &e.item {
            SessionQueueEntryItem::TargetOperation { operation_id }
                if e.state == SessionQueueEntryState::Pending =>
            {
                Some(operation_id)
            }
            _ => None,
        })
        .last()
        .unwrap()
        .clone();
    let waiting = fixture.repository.target_operation(&id).unwrap().unwrap();
    assert_eq!(waiting.phase, TargetOperationPhase::Pending);
    assert!(waiting.actions.is_empty());
    assert!(waiting.staged_target.is_none());
    let projection = waiting.projected_plan.clone();
    fixture.runtime.release(true);
    fixture
        .app
        .accept_prepared_message(fixture.input("message-after-switch"))
        .unwrap();
    fixture.wait(|| {
        fixture
            .repository
            .target_operation(&id)
            .unwrap()
            .is_some_and(|op| op.phase == TargetOperationPhase::Completed)
    });
    let completed = fixture.repository.target_operation(&id).unwrap().unwrap();
    assert_eq!(completed.projected_plan, projection);
    assert!(completed.actions.iter().all(|a| !matches!(
        a.kind,
        TargetOperationActionKind::CaptureSnapshot
            | TargetOperationActionKind::ActivateSister
            | TargetOperationActionKind::PrepareDevice
    )));
    let history = fixture.app.load_session(&fixture.session.id).unwrap();
    let first_position = history
        .timeline
        .positions
        .iter()
        .find(|p| p.invocation_id.as_deref() == Some(first.invocation_id.as_str()))
        .unwrap()
        .ordinal;
    let action_position = history
        .timeline
        .positions
        .iter()
        .find(|p| p.operation_id.as_deref() == Some(id.as_str()))
        .unwrap()
        .ordinal;
    assert!(first_position < action_position);
    fixture.wait(|| {
        fixture
            .notifier
            .notifications
            .lock()
            .unwrap()
            .iter()
            .any(|notification| {
                matches!(notification, AgentSessionNotification::QueueUpdated { timeline, .. }
            if timeline.transitions.iter().any(|receipt| receipt.operation_id == id))
            })
    });
    assert_eq!(
        history
            .timeline
            .transitions
            .last()
            .unwrap()
            .destination
            .worktree_id,
        "second-workspace"
    );
}

#[test]
fn explicit_same_target_is_queued_then_disappears_without_history() {
    let fixture = PreparationFixture::new();
    fixture
        .app
        .accept_prepared_message(fixture.input("active-turn"))
        .unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    let before = fixture
        .app
        .load_session(&fixture.session.id)
        .unwrap()
        .timeline
        .transitions
        .len();
    let queue = fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest::retarget(fixture.selection.clone()),
            None,
        )
        .unwrap();
    let id = queue
        .entries
        .iter()
        .filter_map(|e| match &e.item {
            SessionQueueEntryItem::TargetOperation { operation_id }
                if e.state == SessionQueueEntryState::Pending =>
            {
                Some(operation_id.clone())
            }
            _ => None,
        })
        .last()
        .unwrap();
    assert_eq!(
        fixture
            .repository
            .target_operation(&id)
            .unwrap()
            .unwrap()
            .phase,
        TargetOperationPhase::Pending
    );
    fixture.runtime.release(true);
    fixture
        .app
        .accept_prepared_message(fixture.input("same-target-followup"))
        .unwrap();
    fixture.wait(|| fixture.repository.target_operation(&id).unwrap().is_none());
    assert_eq!(
        fixture
            .app
            .load_session(&fixture.session.id)
            .unwrap()
            .timeline
            .transitions
            .len(),
        before
    );
}

#[test]
fn removing_last_queued_message_deletes_all_unused_switches_but_keeps_delivered_work() {
    let fixture = PreparationFixture::new();
    fixture
        .app
        .accept_prepared_message(fixture.input("running"))
        .unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    fixture
        .app
        .accept_prepared_message(fixture.input("before-switch"))
        .unwrap();
    fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest::retarget(local_selection(&fixture, "future")),
            None,
        )
        .unwrap();
    fixture
        .app
        .accept_prepared_message(fixture.input("after-switch"))
        .unwrap();
    fixture
        .app
        .cancel_queue_entry(&fixture.session.id, "message:after-switch")
        .unwrap();
    assert!(fixture
        .repository
        .get_invocation(&AgentInvocationId::new("after-switch").unwrap())
        .unwrap()
        .is_none());
    assert!(fixture
        .repository
        .preparation(&AgentInvocationId::new("after-switch").unwrap())
        .unwrap()
        .is_none());
    assert!(fixture
        .app
        .load_session_queue(&fixture.session.id)
        .unwrap()
        .entries
        .iter()
        .any(
            |e| matches!(e.item, SessionQueueEntryItem::TargetOperation { .. })
                && e.state == SessionQueueEntryState::Pending
        ));
    fixture
        .app
        .cancel_queue_entry(&fixture.session.id, "message:before-switch")
        .unwrap();
    assert!(!fixture
        .app
        .load_session_queue(&fixture.session.id)
        .unwrap()
        .entries
        .iter()
        .any(
            |e| matches!(e.item, SessionQueueEntryItem::TargetOperation { .. })
                && e.state == SessionQueueEntryState::Pending
        ));
    assert!(fixture
        .repository
        .get_invocation(&AgentInvocationId::new("running").unwrap())
        .unwrap()
        .is_some());
}

#[test]
fn projection_is_frozen_after_edit_and_reorder_until_manual_recalculation() {
    let fixture = PreparationFixture::new();
    fixture
        .app
        .accept_prepared_message(fixture.input("blocking"))
        .unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    let queue = fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest::retarget(local_selection(&fixture, "preview")),
            None,
        )
        .unwrap();
    let entry = queue
        .entries
        .iter()
        .find(|e| {
            e.state == SessionQueueEntryState::Pending
                && matches!(e.item, SessionQueueEntryItem::TargetOperation { .. })
        })
        .unwrap();
    let SessionQueueEntryItem::TargetOperation { operation_id } = &entry.item else {
        unreachable!()
    };
    let original = fixture
        .repository
        .target_operation(operation_id)
        .unwrap()
        .unwrap()
        .projected_plan;
    let queue = fixture
        .app
        .update_target_operation(
            crate::agent_sessions::application::queue::UpdateTargetOperationInput {
                session_id: fixture.session.id.clone(),
                entry_id: entry.id.clone(),
                expected_revision: queue.revision,
                request: TargetSwitchRequest::retarget(local_selection(
                    &fixture,
                    "changed-preview",
                )),
            },
        )
        .unwrap();
    let edited = fixture
        .repository
        .target_operation(operation_id)
        .unwrap()
        .unwrap();
    assert!(edited.projected_plan.stale);
    assert_eq!(edited.projected_plan.facts, original.facts);
    assert_eq!(edited.projected_plan.calculated_at, original.calculated_at);
    fixture
        .app
        .recalculate_target_operation_projection(&fixture.session.id, &entry.id, queue.revision)
        .unwrap();
    let refreshed = fixture
        .repository
        .target_operation(operation_id)
        .unwrap()
        .unwrap();
    assert!(!refreshed.projected_plan.stale);
    assert_eq!(
        refreshed
            .projected_plan
            .facts
            .destination
            .unwrap()
            .worktree_id,
        "changed-preview"
    );
    assert!(refreshed.actions.is_empty());
    assert!(refreshed.staged_target.is_none());
}

#[test]
fn switch_insertion_before_a_message_is_atomic_and_uses_preceding_source() {
    let fixture = PreparationFixture::new();
    fixture
        .app
        .accept_prepared_message(fixture.input("running-insertion"))
        .unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    fixture
        .app
        .accept_prepared_message(fixture.input("queued-insertion"))
        .unwrap();
    let destination = local_selection(&fixture, "inserted-workspace");
    let queue = fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest::retarget(destination),
            Some("message:queued-insertion"),
        )
        .unwrap();
    let position = queue
        .entries
        .iter()
        .position(|e| e.id == "message:queued-insertion")
        .unwrap();
    let SessionQueueEntryItem::TargetOperation { operation_id } = &queue.entries[position - 1].item
    else {
        panic!("switch must precede the message in the returned transaction snapshot")
    };
    let operation = fixture
        .repository
        .target_operation(operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        operation.projected_plan.source.unwrap().worktree_id,
        fixture
            .app
            .load_session(&fixture.session.id)
            .unwrap()
            .session
            .execution_target
            .unwrap()
            .worktree_id
    );
    assert!(operation.actions.is_empty());
    assert!(fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest::retarget(fixture.selection.clone()),
            Some("missing-message")
        )
        .is_err());
    assert_eq!(
        fixture
            .app
            .load_session_queue(&fixture.session.id)
            .unwrap()
            .revision,
        queue.revision
    );
}

fn git(directory: &std::path::Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .current_dir(directory)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}
fn migration_fixture(
    fixture: &PreparationFixture,
) -> (SessionExecutionTarget, SessionExecutionSelection) {
    let root = fixture._directory.path().join("git-source");
    let destination = fixture._directory.path().join("git-destination");
    std::fs::create_dir(&root).unwrap();
    git(&root, &["init", "-b", "main"]);
    git(&root, &["config", "user.name", "Queue test"]);
    git(
        &root,
        &["config", "user.email", "queue-test@example.invalid"],
    );
    std::fs::write(root.join("tracked.txt"), "base").unwrap();
    git(&root, &["add", "tracked.txt"]);
    git(&root, &["commit", "-m", "base"]);
    git(
        &root,
        &[
            "worktree",
            "add",
            "--force",
            destination.to_str().unwrap(),
            "main",
        ],
    );
    let connection =
        rusqlite::Connection::open(fixture._directory.path().join("sessions.sqlite")).unwrap();
    connection
        .execute(
            "INSERT INTO registered_repositories VALUES(?1,?2,?3,?4,?5,?5)",
            rusqlite::params![
                "migration-repo",
                "Migration fixture",
                root.to_str().unwrap(),
                root.join(".git").to_str().unwrap(),
                chrono::Utc::now().to_rfc3339()
            ],
        )
        .unwrap();
    let targets = fixture.app.execution_target_service.as_ref().unwrap();
    let instances = targets
        .endpoints
        .worktrees(
            &fixture.selection.execution,
            root.to_str().unwrap(),
            Some("refs/heads/main"),
        )
        .unwrap();
    let make = |path: &std::path::Path| {
        let instance = instances
            .iter()
            .find(|i| std::path::Path::new(&i.path).file_name() == path.file_name())
            .unwrap();
        SessionExecutionTarget {
            capability_profile_id: fixture.selection.capability_profile_id.clone(),
            capability_profile_revision: fixture.selection.capability_profile_revision,
            execution: fixture.selection.execution.clone(),
            repository_id: "migration-repo".into(),
            branch_ref: instance.branch_ref.clone().unwrap(),
            worktree_id: instance.handle.clone(),
            path: instance.path.clone(),
            head: instance.head.clone(),
        }
    };
    let source = make(&root);
    connection
        .execute(
            "UPDATE agent_sessions SET execution_target_json=?2, working_directory=?3 WHERE id=?1",
            rusqlite::params![
                fixture.session.id.as_str(),
                serde_json::to_string(&source).unwrap(),
                source.path
            ],
        )
        .unwrap();
    (
        source,
        SessionExecutionSelection {
            workspace: SessionWorkspaceSelection::Existing {
                target: make(&destination),
            },
            ..fixture.selection.clone()
        },
    )
}
fn wait_migration(fixture: &PreparationFixture, operation_id: &str) {
    let queue = fixture.app.load_session_queue(&fixture.session.id).unwrap();
    if let Some(entry) = queue.entries.iter().find(|e| matches!(&e.item, SessionQueueEntryItem::TargetOperation {operation_id: id} if id == operation_id)) {
        if entry.state == SessionQueueEntryState::Pending { fixture.app.run_target_operation_now(&fixture.session.id, &entry.id).unwrap(); }
    }
    let started = std::time::Instant::now();
    loop {
        let operation = fixture
            .repository
            .target_operation(operation_id)
            .unwrap()
            .unwrap();
        if matches!(
            operation.phase,
            TargetOperationPhase::Completed | TargetOperationPhase::Blocked
        ) {
            return;
        }
        assert!(
            started.elapsed() < std::time::Duration::from_secs(30),
            "Migration stalled: {:?}",
            operation.actions
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
#[test]
fn explicit_link_migration_carries_real_git_changes_and_preserves_native_owner() {
    let fixture = PreparationFixture::new();
    let (source, destination) = migration_fixture(&fixture);
    fixture
        .repository
        .update_runtime_binding(
            &fixture.session.id,
            AgentRuntimeBinding {
                external_context_id: Some(ExternalRuntimeContextId::new("source-native").unwrap()),
                runtime_version: None,
            },
            chrono::Utc::now(),
        )
        .unwrap();
    let owner = fixture
        .repository
        .current_provider_binding_owner(&fixture.session.id)
        .unwrap();
    std::fs::write(
        std::path::Path::new(&source.path).join("tracked.txt"),
        "modified source",
    )
    .unwrap();
    std::fs::write(
        std::path::Path::new(&source.path).join("untracked.txt"),
        "new source file",
    )
    .unwrap();
    let queue = fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest {
                destination: destination.clone(),
                intent: TargetSwitchIntent::ContinueWork,
                migration_resolution: Some(MigrationResolution {
                    strategy: MigrationStrategy::LinkExisting,
                    repository_id: source.repository_id.clone(),
                    branch_ref: source.branch_ref.clone(),
                }),
            },
            None,
        )
        .unwrap();
    let SessionQueueEntryItem::TargetOperation { operation_id } = &queue.entries[0].item else {
        unreachable!()
    };
    wait_migration(&fixture, operation_id);
    let operation = fixture
        .repository
        .target_operation(operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        operation.phase,
        TargetOperationPhase::Completed,
        "{:?}",
        operation.error
    );
    let SessionWorkspaceSelection::Existing { target } = destination.workspace else {
        unreachable!()
    };
    assert_eq!(
        std::fs::read_to_string(std::path::Path::new(&target.path).join("tracked.txt")).unwrap(),
        "modified source"
    );
    assert_eq!(
        std::fs::read_to_string(std::path::Path::new(&target.path).join("untracked.txt")).unwrap(),
        "new source file"
    );
    assert!(std::path::Path::new(&source.path).exists());
    assert_eq!(
        fixture
            .repository
            .current_provider_binding_owner(&fixture.session.id)
            .unwrap(),
        owner
    );
    assert_eq!(
        fixture
            .app
            .load_session(&fixture.session.id)
            .unwrap()
            .session
            .runtime_binding
            .external_context_id
            .unwrap()
            .as_str(),
        "source-native"
    );
    let sisters = &fixture
        .app
        .execution_target_service
        .as_ref()
        .unwrap()
        .sisters;
    let lock = sisters
        .lock_for(&source.repository_id, &source.branch_ref)
        .unwrap()
        .unwrap();
    assert!(sisters
        .contains_instance(
            &lock.sister_group_id,
            &source.execution.device_id,
            &source.worktree_id
        )
        .unwrap());
    assert!(sisters
        .contains_instance(
            &lock.sister_group_id,
            &target.execution.device_id,
            &target.worktree_id
        )
        .unwrap());
}
#[test]
fn explicit_migration_blocks_a_dirty_destination_without_overwriting_it() {
    let fixture = PreparationFixture::new();
    let (source, destination) = migration_fixture(&fixture);
    let SessionWorkspaceSelection::Existing { target } = &destination.workspace else {
        unreachable!()
    };
    let protected = std::path::Path::new(&target.path).join("tracked.txt");
    std::fs::write(&protected, "destination edits").unwrap();
    let queue = fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest {
                destination,
                intent: TargetSwitchIntent::ContinueWork,
                migration_resolution: Some(MigrationResolution {
                    strategy: MigrationStrategy::LinkExisting,
                    repository_id: source.repository_id.clone(),
                    branch_ref: source.branch_ref.clone(),
                }),
            },
            None,
        )
        .unwrap();
    let SessionQueueEntryItem::TargetOperation { operation_id } = &queue.entries[0].item else {
        unreachable!()
    };
    fixture
        .app
        .run_target_operation_now(&fixture.session.id, &queue.entries[0].id)
        .unwrap();
    fixture.wait(|| {
        fixture
            .repository
            .target_operation(operation_id)
            .unwrap()
            .is_some_and(|op| op.phase == TargetOperationPhase::Blocked)
    });
    assert_eq!(
        std::fs::read_to_string(protected).unwrap(),
        "destination edits"
    );
    assert_eq!(
        fixture
            .app
            .load_session(&fixture.session.id)
            .unwrap()
            .session
            .execution_target
            .unwrap()
            .worktree_id,
        source.worktree_id
    );
    assert!(fixture
        .app
        .load_session(&fixture.session.id)
        .unwrap()
        .timeline
        .transitions
        .is_empty());
}

#[test]
fn a_disappeared_selected_folder_blocks_without_replacement() {
    let fixture = PreparationFixture::new();
    fixture
        .app
        .accept_prepared_message(fixture.input("before-disappearance"))
        .unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    let selection = local_selection(&fixture, "disappearing-folder");
    let SessionWorkspaceSelection::Existing { target } = &selection.workspace else {
        unreachable!()
    };
    let path = target.path.clone();
    let queue = fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest::retarget(selection),
            None,
        )
        .unwrap();
    let id = queue
        .entries
        .iter()
        .filter_map(|entry| match &entry.item {
            SessionQueueEntryItem::TargetOperation { operation_id }
                if entry.state == SessionQueueEntryState::Pending =>
            {
                Some(operation_id.clone())
            }
            _ => None,
        })
        .last()
        .unwrap();
    std::fs::remove_dir(&path).unwrap();
    fixture.runtime.release(true);
    fixture
        .app
        .accept_prepared_message(fixture.input("after-missing-switch"))
        .unwrap();
    fixture.wait(|| {
        fixture
            .repository
            .target_operation(&id)
            .unwrap()
            .is_some_and(|op| op.phase == TargetOperationPhase::Blocked)
    });
    assert!(!std::path::Path::new(&path).exists());
    assert!(fixture
        .repository
        .target_operation(&id)
        .unwrap()
        .unwrap()
        .receipt
        .is_none());
}

#[test]
fn workspace_activation_does_not_prepare_or_resolve_a_provider() {
    let fixture = PreparationFixture::new();
    let mut selection = local_selection(&fixture, "workspace-only-switch");
    selection.execution.provider = "claude".into();
    selection.execution.configuration_ref = "unavailable-inference-configuration".into();
    let queue = fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest::retarget(selection),
            None,
        )
        .unwrap();
    let SessionQueueEntryItem::TargetOperation { operation_id } = &queue.entries[0].item else {
        unreachable!()
    };
    wait_migration(&fixture, operation_id);
    let operation = fixture
        .repository
        .target_operation(operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        operation.phase,
        TargetOperationPhase::Completed,
        "{:?}",
        operation.error
    );
    assert_eq!(fixture.runtime.attempts(), 0);
}

#[test]
fn retargeting_a_detached_worktree_does_not_require_a_selected_head() {
    let fixture = PreparationFixture::new();
    let (_, mut selection) = migration_fixture(&fixture);
    let SessionWorkspaceSelection::Existing { target } = &mut selection.workspace else {
        unreachable!()
    };
    git(
        std::path::Path::new(&target.path),
        &["checkout", "--detach", "HEAD"],
    );
    target.branch_ref.clear();
    target.head = None;
    let queue = fixture
        .app
        .enqueue_session_target_operation(
            &fixture.session.id,
            TargetSwitchRequest::retarget(selection),
            None,
        )
        .unwrap();
    let SessionQueueEntryItem::TargetOperation { operation_id } = &queue.entries[0].item else {
        unreachable!()
    };
    wait_migration(&fixture, operation_id);
    let operation = fixture
        .repository
        .target_operation(operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        operation.phase,
        TargetOperationPhase::Completed,
        "{:?}",
        operation.error
    );
    assert!(operation.staged_target.unwrap().branch_ref.is_empty());
    assert!(operation
        .actions
        .iter()
        .all(|action| action.kind != TargetOperationActionKind::CaptureSnapshot));
}
