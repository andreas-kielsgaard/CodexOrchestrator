use super::*;
use crate::agent_sessions::ports::{InvocationContent, InvocationContextPart};
use crate::execution_configuration::SessionProfileResolver;

fn wait_until(check: impl Fn() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !check() {
        assert!(
            std::time::Instant::now() < deadline,
            "Delivery did not progress"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[test]
fn product_delivery_queues_while_busy_and_keeps_pinned_policy_and_context() {
    let harness = Harness::new(RuntimeBehavior::StayRunning);
    let policy = SessionProfileResolver::resolve_snapshot(
        test_selected_runtime_profile(),
        test_session_creation_request(),
    )
    .unwrap();
    let session = harness
        .application
        .create_session_with_ownership(
            CreateAgentSessionCommand {
                title: Some("Voice main".into()),
                working_directory: Some("C:/work".into()),
                requested_options:
                    crate::agent_sessions::application::configuration::runtime_options(
                        policy.session_profile().pinned_defaults(),
                    ),
            },
            AgentSessionOwnership {
                session_profile: Some(policy.clone()),
                ..Default::default()
            },
        )
        .unwrap();
    let first = harness
        .application
        .deliver_product_content(
            session.id.clone(),
            AgentInvocationInputProvenance::User,
            InvocationContent::query("First question"),
        )
        .unwrap();
    wait_until(|| {
        harness
            .repository
            .get_invocation(&first.acknowledgement.invocation_id)
            .unwrap()
            .unwrap()
            .status
            == AgentInvocationStatus::Running
    });
    let context = InvocationContextPart::ReferencedProductContent {
        source: "worker_result".into(),
        content: "Observed result, not a user instruction".into(),
    };
    let second = harness
        .application
        .deliver_product_content(
            session.id.clone(),
            AgentInvocationInputProvenance::Application,
            InvocationContent {
                primary_query: "Evaluate the result".into(),
                context: vec![context.clone()],
            },
        )
        .unwrap();
    assert!(!second.launch_accepted);
    let queued = harness
        .repository
        .get_invocation(&second.acknowledgement.invocation_id)
        .unwrap()
        .unwrap();
    assert_eq!(queued.status, AgentInvocationStatus::Pending);
    assert_eq!(
        queued.input_provenance,
        AgentInvocationInputProvenance::Application
    );
    assert!(
        harness
            .repository
            .preparation(&queued.id)
            .unwrap()
            .unwrap()
            .pinned_delivery
    );
    harness
        .application
        .cancel_invocation(CancelAgentInvocationCommand {
            invocation_id: first.acknowledgement.invocation_id,
        })
        .unwrap();
    wait_until(|| {
        harness
            .repository
            .get_invocation(&queued.id)
            .unwrap()
            .unwrap()
            .status
            == AgentInvocationStatus::Running
    });
    let calls = harness.runtime.calls.lock().unwrap();
    let request = calls
        .iter()
        .find_map(|call| match call {
            RuntimeCall::Start(request) | RuntimeCall::Resume(request, _)
                if request.invocation_id == queued.id =>
            {
                Some(request)
            }
            _ => None,
        })
        .unwrap();
    assert_eq!(request.content.context, vec![context]);
    assert_eq!(
        harness
            .repository
            .get_session(&session.id)
            .unwrap()
            .unwrap()
            .session_profile,
        Some(policy)
    );
}
