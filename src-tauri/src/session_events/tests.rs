use super::*;
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FakeDirectory {
    entries: Mutex<Vec<SessionDirectoryEntry>>,
    creations: Mutex<Vec<SessionCreationSpec>>,
    sequence: Mutex<u64>,
}

impl FakeDirectory {
    fn with_entries(entries: Vec<SessionDirectoryEntry>) -> Self {
        let sequence = entries
            .iter()
            .flat_map(|entry| [Some(entry.created_sequence), entry.last_addressed_sequence])
            .flatten()
            .max()
            .unwrap_or(0);
        Self {
            entries: Mutex::new(entries),
            creations: Mutex::new(Vec::new()),
            sequence: Mutex::new(sequence),
        }
    }
}

impl SessionDirectory for FakeDirectory {
    fn find_exact(
        &self,
        session: &ReferenceIdentity,
    ) -> Result<Option<SessionDirectoryEntry>, SessionDirectoryError> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .iter()
            .find(|entry| &entry.session == session)
            .cloned())
    }

    fn list_at_address(
        &self,
        address: &SessionLogicalAddress,
    ) -> Result<Vec<SessionDirectoryEntry>, SessionDirectoryError> {
        Ok(self
            .entries
            .lock()
            .unwrap()
            .iter()
            .filter(|entry| entry.logical_address.as_ref() == Some(address))
            .cloned()
            .collect())
    }

    fn create_session(
        &self,
        request: SessionCreationSpec,
    ) -> Result<SessionDirectoryEntry, SessionDirectoryError> {
        self.creations.lock().unwrap().push(request.clone());
        let mut sequence = self.sequence.lock().unwrap();
        *sequence += 1;
        let entry = SessionDirectoryEntry {
            session: reference("session", &format!("created-{sequence}")),
            logical_address: Some(request.logical_address),
            running: true,
            created_sequence: *sequence,
            last_addressed_sequence: None,
            created_by_event: Some(request.created_by_event),
            created_by_session: request.created_by_session,
        };
        self.entries.lock().unwrap().push(entry.clone());
        Ok(entry)
    }

    fn mark_addressed(
        &self,
        session: &ReferenceIdentity,
        _event_group_id: &ReferenceIdentity,
    ) -> Result<u64, SessionDirectoryError> {
        let mut sequence = self.sequence.lock().unwrap();
        *sequence += 1;
        let current = *sequence;
        let mut entries = self.entries.lock().unwrap();
        let entry = entries
            .iter_mut()
            .find(|entry| &entry.session == session)
            .ok_or_else(|| SessionDirectoryError::new("missing Session"))?;
        entry.last_addressed_sequence = Some(current);
        Ok(current)
    }
}

#[derive(Default)]
struct FakeDispatcher {
    requests: Mutex<Vec<SessionInvocationRequest>>,
    failing: Mutex<HashSet<ReferenceIdentity>>,
}

impl SessionInvocationDispatcher for FakeDispatcher {
    fn dispatch(
        &self,
        request: SessionInvocationRequest,
    ) -> Result<SessionInvocationReceipt, SessionInvocationError> {
        let should_fail = self
            .failing
            .lock()
            .unwrap()
            .contains(&request.target_session);
        self.requests.lock().unwrap().push(request.clone());
        if should_fail {
            Err(SessionInvocationError::new("planned invocation failure"))
        } else {
            Ok(SessionInvocationReceipt {
                invocation: reference("invocation", request.delivery_id.id()),
            })
        }
    }
}

fn application(
    directory: Arc<FakeDirectory>,
    dispatcher: Arc<FakeDispatcher>,
    store: Arc<InMemorySessionEventStore>,
) -> SessionEventApplication {
    SessionEventApplication::new(directory, dispatcher, store)
}

fn reference(kind: &str, id: &str) -> ReferenceIdentity {
    ReferenceIdentity::new("test", kind, id).unwrap()
}

fn address() -> SessionLogicalAddress {
    SessionLogicalAddress::new(
        reference("workflow_run", "run-1"),
        reference("node", "review"),
    )
}

fn entry(id: &str, created: u64, addressed: Option<u64>, running: bool) -> SessionDirectoryEntry {
    SessionDirectoryEntry {
        session: reference("session", id),
        logical_address: Some(address()),
        running,
        created_sequence: created,
        last_addressed_sequence: addressed,
        created_by_event: None,
        created_by_session: None,
    }
}

fn selection(target: SessionTarget, missing: MissingTargetPolicy) -> TargetSelection {
    TargetSelection {
        target,
        cardinality: TargetCardinality::First,
        ordering: TargetOrdering::Newest,
        running: RunningFilter::Any,
        created_by: None,
        missing,
    }
}

fn command(
    group_id: &str,
    target: SessionTarget,
    missing: MissingTargetPolicy,
) -> SessionEventCommand {
    SessionEventCommand {
        event_group_id: reference("event_group", group_id),
        definition_ref: reference("event_definition", "definition-1"),
        trigger: SessionEventTrigger::ApplicationEvent {
            event: reference("application_event", "event-1"),
        },
        source: SessionEventSource::ApplicationEvent {
            event: reference("application_event", "event-1"),
        },
        prompt_sources: vec![PromptSource::literal("Main context")],
        created_session_prompt_sources: vec![PromptSource::literal("Initial context")],
        creation_configuration: None,
        target: selection(target, missing),
        created_by_session: None,
        direct_user_options: None,
    }
}

#[test]
fn direct_user_options_require_a_user_request_and_exact_target() {
    let session = reference("session", "existing");
    let mut direct = command(
        "direct",
        SessionTarget::Exact {
            session: session.clone(),
        },
        MissingTargetPolicy::Fail,
    );
    direct.trigger = SessionEventTrigger::UserRequest {
        request: reference("request", "request-1"),
    };
    direct.source = SessionEventSource::UserRequest {
        request: reference("request", "request-1"),
    };
    direct.direct_user_options = Some(DirectUserInvocationOptions {
        model: Some("gpt-5".into()),
        reasoning_mode: Some("high".into()),
    });
    assert!(direct.validate().is_ok());

    direct.target.target = SessionTarget::Logical { address: address() };
    assert!(matches!(
        direct.validate(),
        Err(SessionEventDomainError::InvalidInvocationOptions(_))
    ));
}

#[test]
fn direct_user_options_are_forwarded_to_an_unaddressed_exact_session() {
    let session = reference("session", "existing");
    let mut existing = entry("existing", 1, None, true);
    existing.logical_address = None;
    let directory = Arc::new(FakeDirectory::with_entries(vec![existing]));
    let dispatcher = Arc::new(FakeDispatcher::default());
    let store = Arc::new(InMemorySessionEventStore::default());
    let mut direct = command(
        "direct-delivery",
        SessionTarget::Exact {
            session: session.clone(),
        },
        MissingTargetPolicy::Fail,
    );
    direct.trigger = SessionEventTrigger::UserRequest {
        request: reference("request", "request-2"),
    };
    direct.source = SessionEventSource::UserRequest {
        request: reference("request", "request-2"),
    };
    direct.direct_user_options = Some(DirectUserInvocationOptions {
        model: Some("gpt-5".into()),
        reasoning_mode: Some("high".into()),
    });

    let result = application(directory, dispatcher.clone(), store)
        .dispatch(direct)
        .unwrap();

    assert_eq!(result.group.resolved_sessions, vec![session]);
    assert_eq!(result.deliveries[0].addressed_sequence, None);
    assert_eq!(
        dispatcher.requests.lock().unwrap()[0].direct_user_options,
        Some(DirectUserInvocationOptions {
            model: Some("gpt-5".into()),
            reasoning_mode: Some("high".into()),
        })
    );
}

#[test]
fn newest_running_target_is_selected_and_marked_addressed() {
    let older = entry("older", 1, Some(100), true);
    let newest = entry("newest", 2, None, true);
    let stopped = entry("stopped", 3, None, false);
    let directory = Arc::new(FakeDirectory::with_entries(vec![
        older,
        newest.clone(),
        stopped,
    ]));
    let dispatcher = Arc::new(FakeDispatcher::default());
    let store = Arc::new(InMemorySessionEventStore::default());
    let mut event = command(
        "newest",
        SessionTarget::Logical { address: address() },
        MissingTargetPolicy::Fail,
    );
    event.target.running = RunningFilter::RunningOnly;

    let result = application(directory.clone(), dispatcher, store)
        .dispatch(event)
        .unwrap();

    assert_eq!(result.group.resolved_sessions, vec![newest.session.clone()]);
    assert!(result.deliveries[0].addressed_sequence.is_some());
    assert_eq!(
        directory
            .find_exact(&newest.session)
            .unwrap()
            .unwrap()
            .last_addressed_sequence,
        result.deliveries[0].addressed_sequence
    );
}

#[test]
fn last_addressed_ordering_is_functional_after_a_delivery() {
    let newest = entry("newest", 20, None, true);
    let older = entry("older", 10, Some(15), true);
    let directory = Arc::new(FakeDirectory::with_entries(vec![newest.clone(), older]));
    let dispatcher = Arc::new(FakeDispatcher::default());
    let store = Arc::new(InMemorySessionEventStore::default());
    let app = application(directory, dispatcher, store);

    app.dispatch(command(
        "first",
        SessionTarget::Logical { address: address() },
        MissingTargetPolicy::Fail,
    ))
    .unwrap();
    let mut second = command(
        "second",
        SessionTarget::Logical { address: address() },
        MissingTargetPolicy::Fail,
    );
    second.target.ordering = TargetOrdering::LastAddressed;
    let result = app.dispatch(second).unwrap();

    assert_eq!(result.group.resolved_sessions, vec![newest.session]);
}

#[test]
fn create_on_missing_pins_configuration_and_uses_initial_prompt_once() {
    let directory = Arc::new(FakeDirectory::default());
    let dispatcher = Arc::new(FakeDispatcher::default());
    let store = Arc::new(InMemorySessionEventStore::default());
    let app = application(directory.clone(), dispatcher.clone(), store);
    let mut create = command(
        "create",
        SessionTarget::Logical { address: address() },
        MissingTargetPolicy::Create,
    );
    create.created_by_session = Some(reference("session", "parent"));
    create.creation_configuration = Some(SessionCreationConfiguration {
        contract: reference("contract", "session-creation-request-v1"),
        payload: json!({ "capabilityProfile": "capability-v1", "nodeProfile": {} }),
    });

    let created = app.dispatch(create).unwrap();
    assert_eq!(
        created.group.created_session,
        Some(created.deliveries[0].target_session.clone())
    );
    assert_eq!(
        dispatcher.requests.lock().unwrap()[0]
            .initial_prompt
            .as_deref(),
        Some("Initial context")
    );
    let creation = directory.creations.lock().unwrap()[0].clone();
    assert_eq!(
        creation.created_by_event,
        reference("event_group", "create")
    );
    assert_eq!(
        creation.created_by_session,
        Some(reference("session", "parent"))
    );

    app.dispatch(command(
        "reuse",
        SessionTarget::Logical { address: address() },
        MissingTargetPolicy::Fail,
    ))
    .unwrap();
    assert_eq!(dispatcher.requests.lock().unwrap()[1].initial_prompt, None);
}

#[test]
fn dispatches_a_materialized_application_event_through_the_session_kernel() {
    let directory = Arc::new(FakeDirectory::default());
    let dispatcher = Arc::new(FakeDispatcher::default());
    let store = Arc::new(InMemorySessionEventStore::default());
    let app = application(directory, dispatcher.clone(), store);
    let event_kind = reference("application_event_kind", "plan_ready");
    let definition = SessionEventDefinition {
        definition_ref: reference("event_definition", "review-plan"),
        trigger: SessionEventTriggerBinding::ApplicationEvent {
            event_kind: event_kind.clone(),
        },
        target: selection(
            SessionTarget::Logical { address: address() },
            MissingTargetPolicy::Create,
        ),
        prompt_sources: vec![
            PromptSourceDefinition::ApplicationEventField {
                field: "plan".into(),
            },
            PromptSourceDefinition::Literal {
                text: "Review this plan.".into(),
            },
        ],
        created_session_prompt_sources: vec![PromptSourceDefinition::Literal {
            text: "You review plans.".into(),
        }],
        creation_configuration: Some(SessionCreationConfiguration {
            contract: reference("contract", "session-creation-request-v1"),
            payload: json!({ "capabilityProfile": "capability-v1", "nodeProfile": {} }),
        }),
    };
    let mut fields = BTreeMap::new();
    fields.insert("plan".into(), "Plan contents".into());

    let result = app
        .dispatch_occurrence(
            &definition,
            SessionEventOccurrence {
                event_group_id: reference("event_group", "materialized"),
                trigger: SessionEventOccurrenceTrigger::ApplicationEvent {
                    event: reference("application_event", "event-1"),
                    event_kind,
                    fields,
                },
                referenced_content: BTreeMap::new(),
                created_by_session: None,
                direct_user_options: None,
            },
        )
        .unwrap();

    assert_eq!(result.group.outcome, EventGroupOutcome::Delivered);
    let requests = dispatcher.requests.lock().unwrap();
    assert_eq!(requests[0].prompt, "Plan contents\n\nReview this plan.");
    assert_eq!(
        requests[0].initial_prompt.as_deref(),
        Some("You review plans.")
    );
}

#[test]
fn fan_out_retains_contributions_and_partial_delivery_provenance() {
    let first = entry("first", 2, None, true);
    let second = entry("second", 1, None, true);
    let directory = Arc::new(FakeDirectory::with_entries(vec![
        first.clone(),
        second.clone(),
    ]));
    let dispatcher = Arc::new(FakeDispatcher::default());
    dispatcher
        .failing
        .lock()
        .unwrap()
        .insert(second.session.clone());
    let store = Arc::new(InMemorySessionEventStore::default());
    let mut event = command(
        "fanout",
        SessionTarget::Logical { address: address() },
        MissingTargetPolicy::Fail,
    );
    event.target.cardinality = TargetCardinality::All;
    event.prompt_sources = vec![PromptSource::literal("one"), PromptSource::literal("two")];

    let result = application(directory, dispatcher.clone(), store.clone())
        .dispatch(event)
        .unwrap();

    assert_eq!(result.group.outcome, EventGroupOutcome::PartiallyDelivered);
    assert_eq!(result.deliveries.len(), 2);
    assert!(result
        .deliveries
        .iter()
        .all(|delivery| delivery.prompt_contributions.len() == 2));
    assert!(dispatcher
        .requests
        .lock()
        .unwrap()
        .iter()
        .all(|request| request.prompt == "one\n\ntwo" && request.initial_prompt.is_none()));
    assert_eq!(store.records().unwrap().len(), 1);
}

#[test]
fn missing_fail_and_noop_are_recorded_without_dispatch() {
    let directory = Arc::new(FakeDirectory::default());
    let dispatcher = Arc::new(FakeDispatcher::default());
    let store = Arc::new(InMemorySessionEventStore::default());
    let app = application(directory, dispatcher.clone(), store.clone());

    assert!(matches!(
        app.dispatch(command(
            "fail",
            SessionTarget::Logical { address: address() },
            MissingTargetPolicy::Fail,
        )),
        Err(SessionEventApplicationError::NoTarget(_))
    ));
    let result = app
        .dispatch(command(
            "noop",
            SessionTarget::Logical { address: address() },
            MissingTargetPolicy::Noop,
        ))
        .unwrap();

    assert_eq!(result.group.outcome, EventGroupOutcome::Noop);
    assert!(dispatcher.requests.lock().unwrap().is_empty());
    assert_eq!(store.records().unwrap().len(), 2);
}

#[test]
fn definition_supports_closed_mcp_and_application_event_sources() {
    let definition = SessionEventDefinition {
        definition_ref: reference("event_definition", "mcp"),
        trigger: SessionEventTriggerBinding::McpCall {
            server: reference("mcp_server", "orchestrator"),
            tool: reference("mcp_tool", "continue_workflow"),
        },
        target: selection(
            SessionTarget::Logical { address: address() },
            MissingTargetPolicy::Noop,
        ),
        prompt_sources: vec![PromptSourceDefinition::McpArgument {
            name: "result".into(),
        }],
        created_session_prompt_sources: vec![PromptSourceDefinition::ApplicationEventField {
            field: "context".into(),
        }],
        creation_configuration: None,
    };

    assert!(definition.validate().is_ok());
    let encoded = serde_json::to_value(&definition).unwrap();
    assert_eq!(encoded["trigger"]["kind"], "mcp_call");
    assert_eq!(
        serde_json::from_value::<SessionEventDefinition>(encoded).unwrap(),
        definition
    );
}

#[test]
fn delivery_identity_preserves_the_complete_group_identity_without_delimiter_collisions() {
    let first = ReferenceIdentity::new("a-b", "c", "shared").unwrap();
    let second = ReferenceIdentity::new("a", "b-c", "shared").unwrap();

    assert_ne!(
        delivery_identity(&first, 1).unwrap(),
        delivery_identity(&second, 1).unwrap()
    );

    let maximal = ReferenceIdentity::new("n".repeat(256), "k".repeat(256), "i".repeat(256))
        .expect("maximal valid reference");
    assert_eq!(
        delivery_identity(&maximal, u32::MAX).unwrap().id().len(),
        64
    );
}
