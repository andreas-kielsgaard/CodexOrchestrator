use super::*;
use crate::otp_api::*;

fn context(instance: &RecipeInstance, occurrence: &str, node: &str) -> InvocationContext {
    InvocationContext {
        instance_id: instance.id.clone(),
        occurrence_id: occurrence.into(),
        capability: CapabilityRef {
            package: "workflow".into(),
            tool: "stop_session".into(),
        },
        source: None,
        connection_id: None,
        output_node_id: Some(node.into()),
    }
}
#[test]
fn stop_records_exact_invocation_and_session_can_be_prompted_again() {
    let fixture = Fixture::with_runtime(RuntimeBehavior::StayRunning, false);
    let instance = fixture.instance(None);
    let first = fixture
        .execution
        .dispatch_user_request(&instance.recipe.recipe_id, &instance.id, "Work".into())
        .unwrap();
    let session = first.event_groups[0].group.created_session.clone().unwrap();
    let launched = fixture.launches()[0].invocation_id.clone();
    let ctx = context(&instance, "stop-one", "a");
    let stopped = fixture
        .execution
        .dispatch_otp_action(
            &instance,
            &ctx,
            None,
            json!({}),
            json!({}),
            Err("Must not read prompt files".into()),
        )
        .unwrap();
    assert!(stopped.event_groups.is_empty());
    assert_eq!(stopped.stop_outcomes[0].session_id, session.id());
    assert_eq!(
        stopped.stop_outcomes[0].invocation_id.as_deref(),
        Some(launched.as_str())
    );
    assert_eq!(stopped.stop_outcomes[0].status, "requested");
    assert!(!fixture
        .repository
        .get_invocation(&launched)
        .unwrap()
        .unwrap()
        .status
        .is_active());
    assert!(fixture
        .runtime
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|call| matches!(call, RuntimeCall::Cancel(id) if id == &launched)));
    let resumed = fixture
        .execution
        .dispatch_user_request(&instance.recipe.recipe_id, &instance.id, "Continue".into())
        .unwrap();
    assert_eq!(
        resumed.event_groups[0].deliveries[0].target_session,
        session
    );
    let again = fixture
        .execution
        .dispatch_otp_action(&instance, &ctx, None, json!({}), json!({}), Ok(vec![]))
        .unwrap();
    assert_eq!(again.message, "Already processed");
    assert_eq!(
        fixture
            .runtime
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| matches!(call, RuntimeCall::Cancel(_)))
            .count(),
        1
    );
    let reopened =
        WorkflowInstanceStore::open(&fixture.folder.path().join("repair.sqlite")).unwrap();
    let stored = reopened
        .attempts(&instance.id)
        .unwrap()
        .into_iter()
        .find(|a| a.id == stopped.attempt_id)
        .unwrap();
    assert_eq!(stored.stop_outcomes[0].status, "requested");
    fixture
        .sessions
        .cancel_invocation(CancelAgentInvocationCommand {
            invocation_id: fixture.launches().last().unwrap().invocation_id.clone(),
        })
        .unwrap();
}

#[test]
fn stop_entry_without_sessions_is_a_recorded_noop_and_uses_shared_configuration() {
    let fixture = Fixture::new();
    let original = fixture.instance(None);
    let mut draft = fixture
        .authoring
        .load(&original.recipe.recipe_id)
        .unwrap()
        .draft;
    draft.entry_action.tool = "stop_session".into();
    draft.entry_configuration = json!({"ordering":"last_addressed"});
    let saved = fixture.authoring.save_draft(draft).unwrap();
    fixture.authoring.activate(&saved.draft.recipe_id).unwrap();
    let instance = fixture
        .execution
        .create_instance(
            &saved.draft.recipe_id,
            saved.draft.revision,
            "Stop entry".into(),
            fixture.target(),
        )
        .unwrap();
    let result = fixture
        .execution
        .dispatch_user_request(&instance.recipe.recipe_id, &instance.id, String::new())
        .unwrap();
    assert!(result.event_groups.is_empty());
    assert!(result.stop_outcomes.is_empty());
    assert!(result.message.contains("nothing to stop"));
    assert!(fixture.launches().is_empty());
    assert_eq!(
        fixture.execution.instances.attempts(&instance.id).unwrap()[0].message,
        result.message
    );
    let bad = context(&instance, "bad-node", "outside");
    assert!(fixture
        .execution
        .dispatch_otp_action(&instance, &bad, None, json!({}), json!({}), Ok(vec![]))
        .unwrap_err()
        .contains("Bound node"));
}

struct ControlledCancel(Box<dyn Fn(&str) -> Result<bool, String> + Send + Sync>);
impl crate::otp_host::session_control::SessionControl for ControlledCancel {
    fn cancel(&self, id: &str) -> Result<bool, String> {
        (self.0)(id)
    }
}
fn with_control(fixture: &Fixture, control: ControlledCancel) -> WorkflowExecutionService {
    WorkflowExecutionService::new(
        fixture.authoring.clone(),
        fixture.execution.session_events.clone(),
        Arc::new(
            WorkflowInstanceStore::open(&fixture.folder.path().join("repair.sqlite")).unwrap(),
        ),
        fixture.execution.directory.clone(),
        fixture.repository.clone(),
    )
    .with_session_control(Arc::new(control))
}
#[test]
fn stop_failure_is_recorded_after_target_is_pinned() {
    let fixture = Fixture::with_runtime(RuntimeBehavior::StayRunning, false);
    let instance = fixture.instance(None);
    fixture
        .execution
        .dispatch_user_request(&instance.recipe.recipe_id, &instance.id, "Work".into())
        .unwrap();
    let path = fixture.folder.path().join("repair.sqlite");
    let instance_id = instance.id.clone();
    let controlled = with_control(
        &fixture,
        ControlledCancel(Box::new(move |id| {
            let attempts = WorkflowInstanceStore::open(&path)
                .unwrap()
                .attempts(&instance_id)
                .unwrap();
            assert!(attempts.iter().any(|a| a.stop_outcomes.iter().any(|s| s
                .invocation_id
                .as_deref()
                == Some(id)
                && s.status == "pending")));
            Err("Controlled cancellation failure".into())
        })),
    );
    let ctx = context(&instance, "failure", "a");
    assert!(controlled
        .dispatch_otp_action(&instance, &ctx, None, json!({}), json!({}), Ok(vec![]))
        .unwrap_err()
        .contains("Controlled cancellation"));
    let attempts = controlled.instances.attempts(&instance.id).unwrap();
    assert!(attempts.iter().any(|a| a
        .stop_outcomes
        .iter()
        .any(|s| s.status == "failed" && s.invocation_id.is_some())));
    fixture
        .sessions
        .cancel_invocation(CancelAgentInvocationCommand {
            invocation_id: fixture.launches()[0].invocation_id.clone(),
        })
        .unwrap();
}

#[test]
fn a_finished_target_does_not_cancel_a_newer_invocation() {
    let fixture = Fixture::with_runtime(RuntimeBehavior::StayRunning, false);
    let instance = fixture.instance(None);
    fixture
        .execution
        .dispatch_user_request(&instance.recipe.recipe_id, &instance.id, "Work".into())
        .unwrap();
    let application = fixture.sessions.clone();
    let repository = fixture.repository.clone();
    let execution = fixture.execution.clone();
    let instance_id = instance.id.clone();
    let recipe_id = instance.recipe.recipe_id.clone();
    let controlled = with_control(
        &fixture,
        ControlledCancel(Box::new(move |id| {
            application
                .cancel_invocation(CancelAgentInvocationCommand {
                    invocation_id: AgentInvocationId::new(id).unwrap(),
                })
                .unwrap();
            execution
                .dispatch_user_request(&recipe_id, &instance_id, "New turn".into())
                .unwrap();
            let adapter = crate::otp_host::session_control::AgentSessionControl {
                application: application.clone(),
                repository: repository.clone(),
            };
            crate::otp_host::session_control::SessionControl::cancel(&adapter, id)
        })),
    );
    let stopped = controlled
        .dispatch_otp_action(
            &instance,
            &context(&instance, "race", "a"),
            None,
            json!({}),
            json!({}),
            Ok(vec![]),
        )
        .unwrap();
    assert_eq!(stopped.stop_outcomes[0].status, "no_op");
    let newest = fixture.launches().last().unwrap().invocation_id.clone();
    assert!(fixture
        .repository
        .get_invocation(&newest)
        .unwrap()
        .unwrap()
        .status
        .is_active());
    fixture
        .sessions
        .cancel_invocation(CancelAgentInvocationCommand {
            invocation_id: newest,
        })
        .unwrap();
}
