use super::*;
use crate::agent_sessions::application::SendAgentSessionMessageResult;

fn failed_preparation(fixture: &PreparationFixture, id: &str) -> SendAgentSessionMessageResult {
    let ack = fixture
        .app
        .accept_prepared_message(fixture.input(id))
        .unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    fixture.runtime.release(false);
    fixture.wait(|| {
        fixture
            .repository
            .get_invocation(&ack.invocation_id)
            .unwrap()
            .unwrap()
            .status
            .is_terminal()
    });
    ack
}

#[test]
fn startup_resumes_a_durable_queued_retry_and_preserves_activated_target() {
    let fixture = PreparationFixture::new();
    let ack = failed_preparation(&fixture, "stopped-setup");
    fixture.wait(|| {
        fixture
            .repository
            .queue_entry_for_invocation(&ack.invocation_id)
            .unwrap()
            .is_some_and(|entry| {
                entry.state == crate::agent_sessions::queue::SessionQueueEntryState::Blocked
            })
    });
    let entry = fixture
        .repository
        .queue_entry_for_invocation(&ack.invocation_id)
        .unwrap()
        .expect("queued message");
    let deadline = Instant::now() + std::time::Duration::from_secs(5);
    let _reserved = loop {
        match fixture.app.preparation_workers.reserve(&ack.invocation_id) {
            Ok(reserved) => break reserved,
            Err(_) => {
                assert!(Instant::now() < deadline, "preparation worker did not stop");
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    };
    fixture
        .repository
        .retry_queue_entry(&entry.id, fixture.app.clock.now())
        .unwrap();
    assert_eq!(fixture.app.reconcile_startup().unwrap(), 0);
    fixture.app.preparation_workers.release(&ack.invocation_id);
    fixture.app.wake_session_queue(ack.session_id.clone());
    fixture.wait(|| fixture.runtime.attempts() == 2);
    assert!(fixture.runtime.state.lock().unwrap().delivered.is_empty());
    assert_eq!(
        fixture
            .repository
            .get_invocation(&ack.invocation_id)
            .unwrap()
            .unwrap()
            .status,
        AgentInvocationStatus::Pending
    );
    assert_eq!(
        fixture
            .app
            .load_session(&ack.session_id)
            .unwrap()
            .session
            .execution_target,
        fixture.preparation(&ack.invocation_id).resolved_target
    );
    fixture.runtime.release(true);
    fixture.wait(|| {
        fixture
            .repository
            .get_invocation(&ack.invocation_id)
            .unwrap()
            .unwrap()
            .status
            .is_terminal()
    });
    assert_eq!(fixture.runtime.state.lock().unwrap().delivered.len(), 1);
}

#[test]
fn startup_turns_interrupted_target_staging_into_a_retryable_blocker() {
    let fixture = PreparationFixture::new();
    let now = fixture.app.clock.now();
    let mut operation = crate::agent_sessions::target_operation::SessionTargetOperation::new(
        "interrupted-target".into(),
        fixture.session.id.clone(),
        fixture.session.execution_target.clone(),
        fixture.selection.clone(),
        now,
    );
    fixture
        .repository
        .enqueue_target_operation(operation.clone(), now)
        .unwrap();
    operation.phase = crate::agent_sessions::target_operation::TargetOperationPhase::Staging;
    operation.actions[0].status =
        crate::agent_sessions::target_operation::TargetOperationActionStatus::Running;
    fixture
        .repository
        .save_target_operation(&operation)
        .unwrap();

    assert_eq!(fixture.app.reconcile_startup().unwrap(), 0);

    let operation = fixture
        .repository
        .target_operation(&operation.id)
        .unwrap()
        .unwrap();
    assert_eq!(
        operation.phase,
        crate::agent_sessions::target_operation::TargetOperationPhase::Blocked
    );
    assert!(operation
        .error
        .as_deref()
        .is_some_and(|error| error.contains("restarted")));
    let entry = fixture
        .repository
        .queue_entry_for_operation(&operation.id)
        .unwrap()
        .unwrap();
    assert_eq!(
        entry.state,
        crate::agent_sessions::queue::SessionQueueEntryState::Blocked
    );
}

#[test]
fn startup_delivery_gap_is_visible_uncertainty_and_never_retryable() {
    for already_terminal in [false, true] {
        let fixture = PreparationFixture::new();
        let ack = failed_preparation(&fixture, "stopped-delivery");
        fixture
            .repository
            .retry_preparation(&ack.invocation_id, fixture.app.clock.now())
            .unwrap();
        let mut preparation = fixture.preparation(&ack.invocation_id);
        preparation.phase = PreparationPhase::Ready;
        preparation.delivery_started = true;
        let delivery = preparation
            .steps
            .iter_mut()
            .find(|step| step.id == "delivery")
            .unwrap();
        delivery.status = crate::agent_sessions::preparation::PreparationStepStatus::Running;
        fixture.repository.save_preparation(&preparation).unwrap();
        fixture
            .repository
            .mark_invocation_running(
                &ack.invocation_id,
                fixture.app.clock.now(),
                AgentRuntimeOptions::default(),
                fixture.app.clock.now(),
            )
            .unwrap();
        if already_terminal {
            fixture
                .repository
                .finish_invocation(
                    &ack.invocation_id,
                    InvocationCompletion {
                        status: AgentInvocationTerminalStatus::Completed,
                        completed_at: fixture.app.clock.now(),
                        exit_code: None,
                        signal: None,
                        runtime_error: None,
                    },
                    fixture.app.clock.now(),
                )
                .unwrap();
        }
        fixture.app.reconcile_startup().unwrap();
        let interrupted = fixture.preparation(&ack.invocation_id);
        assert_eq!(interrupted.phase, PreparationPhase::Failed);
        assert!(!interrupted.can_retry);
        assert!(interrupted.error.unwrap().contains("uncertain"));
        assert!(fixture
            .repository
            .retry_preparation(&ack.invocation_id, fixture.app.clock.now())
            .is_err());
        assert_eq!(fixture.runtime.attempts(), 1);
    }
}

#[test]
fn current_successful_profile_changes_without_relabeling_creation_or_previous_invocation() {
    let fixture = PreparationFixture::new();
    let first = fixture
        .app
        .accept_prepared_message(fixture.input("first-profile"))
        .unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    fixture.runtime.release(true);
    fixture.wait(|| {
        fixture
            .repository
            .get_invocation(&first.invocation_id)
            .unwrap()
            .unwrap()
            .status
            .is_terminal()
    });
    let original = fixture
        .app
        .load_pinned_session_profile(LoadPinnedSessionProfileQuery {
            session_id: first.session_id.clone(),
        })
        .unwrap()
        .creation_resolution;
    let profile = fixture
        .app
        .capability_profiles
        .as_ref()
        .unwrap()
        .create_with_defaults(
            "other-profile".into(),
            "Other profile".into(),
            test_selected_runtime_profile().exposure,
            RuntimeSelections {
                model: Some("node-default".into()),
                reasoning_mode: Some("high".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let mut input = fixture.input("other-profile-invocation");
    let selection = input.execution_selection.as_mut().unwrap();
    selection.capability_profile_id = profile.capability_profile_id.clone();
    selection.capability_profile_revision = profile.revision;
    input.model = None;
    input.reasoning_mode = None;
    let second = fixture.app.accept_prepared_message(input).unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 2);
    assert_eq!(
        fixture
            .app
            .load_current_session_profile(&first.session_id)
            .unwrap()
            .creation_resolution,
        original
    );
    assert!(fixture
        .preparation(&second.invocation_id)
        .steps
        .iter()
        .any(|step| step.id == "delivery"
            && step.status == crate::agent_sessions::preparation::PreparationStepStatus::Pending));
    fixture.runtime.release(true);
    fixture.wait(|| {
        fixture
            .repository
            .get_invocation(&second.invocation_id)
            .unwrap()
            .unwrap()
            .status
            .is_terminal()
    });
    let current = fixture
        .app
        .load_current_session_profile(&first.session_id)
        .unwrap()
        .creation_resolution;
    assert_eq!(
        current.session_profile().capability_profile_id(),
        "other-profile"
    );
    assert_eq!(
        current.session_profile().pinned_defaults().model.as_deref(),
        Some("node-default")
    );
    assert_eq!(
        fixture
            .app
            .load_pinned_session_profile(LoadPinnedSessionProfileQuery {
                session_id: first.session_id
            })
            .unwrap()
            .creation_resolution,
        original
    );
    assert_eq!(
        fixture.preparation(&first.invocation_id).current_resolution,
        Some(original)
    );
}
