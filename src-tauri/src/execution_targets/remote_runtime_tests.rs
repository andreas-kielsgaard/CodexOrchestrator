use super::*;
use orchid_engine::contracts::RuntimeInteractionResponse;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

#[derive(Default)]
struct FakeConnection {
    closed: AtomicBool,
    disconnect_on_invoke: AtomicBool,
    calls: Mutex<Vec<HostCommand>>,
    sinks: Mutex<HashMap<AgentInvocationId, Arc<dyn AgentRuntimeUpdateSink>>>,
}

impl HostConnection for FakeConnection {
    fn request(&self, command: HostCommand) -> Result<Value, RuntimePortError> {
        let result = match &command {
            HostCommand::PrepareInvocation {
                request,
                external_context_id,
                ..
            } => serde_json::to_value(RuntimeInvocationReady {
                external_context_id: external_context_id
                    .clone()
                    .unwrap_or_else(|| ExternalRuntimeContextId::new("prepared-thread").unwrap()),
                working_directory: request.working_directory.clone().unwrap(),
            })
            .unwrap(),
            HostCommand::Preflight { options, .. } => {
                serde_json::to_value(RuntimeInvocationPreflight {
                    effective_options: options.clone(),
                })
                .unwrap()
            }
            HostCommand::Invoke { .. } if self.disconnect_on_invoke.load(Ordering::SeqCst) => {
                self.closed.store(true, Ordering::SeqCst);
                Value::Null
            }
            _ => Value::Null,
        };
        self.calls.lock().unwrap().push(command);
        if self.closed.load(Ordering::SeqCst) {
            Err(RuntimePortError::new(
                RuntimePortErrorKind::Unavailable,
                "Fixture connection closed; remote outcome unconfirmed",
            ))
        } else {
            Ok(result)
        }
    }
    fn is_closed(&self) -> Result<bool, RuntimePortError> {
        Ok(self.closed.load(Ordering::SeqCst))
    }
    fn has_active_invocations(&self) -> bool {
        !self.sinks.lock().unwrap().is_empty()
    }
    fn register(&self, id: AgentInvocationId, sink: Arc<dyn AgentRuntimeUpdateSink>) {
        self.sinks.lock().unwrap().insert(id, sink);
    }
    fn unregister(&self, id: &AgentInvocationId) {
        self.sinks.lock().unwrap().remove(id);
    }
    fn shutdown(&self) -> Result<(), RuntimePortError> {
        self.closed.store(true, Ordering::SeqCst);
        Ok(())
    }
}

struct Sink;
impl AgentRuntimeUpdateSink for Sink {
    fn emit_update(&self, _: &AgentInvocationId, _: RuntimeUpdate) -> Result<(), RuntimePortError> {
        Ok(())
    }
    fn report_delivery_failure(&self, _: &AgentInvocationId, _: RuntimeUpdateDeliveryFailure) {}
}

fn invocation(id: &str) -> RuntimeInvocationRequest {
    RuntimeInvocationRequest {
        session_id: AgentSessionId::new("remote-session").unwrap(),
        invocation_id: AgentInvocationId::new(id).unwrap(),
        submitted_text: "Create the requested file".into(),
        working_directory: Some("/srv/project/worktrees/demo".into()),
        options: Default::default(),
        launch_extension: None,
    }
}

#[test]
fn preparation_retains_connection_and_delivery_never_reconnects_or_replays() {
    let original = Arc::new(FakeConnection::default());
    original.closed.store(true, Ordering::SeqCst);
    let replacement = Arc::new(FakeConnection::default());
    let (runtime, attempts) = runtime(original.clone(), replacement.clone());
    let request = invocation("prepared");
    let ready = runtime
        .prepare_invocation(request.clone(), None, Arc::new(Sink))
        .unwrap();
    assert_eq!(ready.external_context_id.as_str(), "prepared-thread");
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert!(replacement.has_active_invocations());
    assert!(matches!(
        replacement.calls.lock().unwrap().as_slice(),
        [HostCommand::PrepareInvocation { .. }]
    ));
    runtime
        .deliver_prepared_invocation(&request.invocation_id)
        .unwrap();
    assert!(matches!(
        replacement.calls.lock().unwrap().last(),
        Some(HostCommand::DeliverPreparedInvocation { .. })
    ));
    replacement.closed.store(true, Ordering::SeqCst);
    assert!(runtime
        .deliver_prepared_invocation(&request.invocation_id)
        .is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

fn runtime(
    original: Arc<FakeConnection>,
    replacement: Arc<FakeConnection>,
) -> (RemoteRuntime, Arc<AtomicUsize>) {
    let attempts = Arc::new(AtomicUsize::new(0));
    let counter = attempts.clone();
    (
        RemoteRuntime {
            connection: Mutex::new(original),
            connect: Arc::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(replacement.clone())
            }),
            binding: ExecutionBinding {
                configuration_ref: "pinned-remote-codex".into(),
                ..Default::default()
            },
        },
        attempts,
    )
}

#[test]
fn closed_idle_connection_is_replaced_before_preflight_and_resume() {
    let original = Arc::new(FakeConnection::default());
    original.closed.store(true, Ordering::SeqCst);
    let replacement = Arc::new(FakeConnection::default());
    let (runtime, attempts) = runtime(original.clone(), replacement.clone());
    runtime
        .preflight_invocation(RuntimeInvocationMode::Resume, &Default::default())
        .unwrap();
    let request = invocation("followup");
    let context = ExternalRuntimeContextId::new("remote-provider-thread").unwrap();
    runtime
        .resume_invocation(request.clone(), context.clone(), Arc::new(Sink))
        .unwrap();

    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert!(original.calls.lock().unwrap().is_empty());
    let calls = replacement.calls.lock().unwrap();
    assert_eq!(calls.len(), 2);
    match &calls[1] {
        HostCommand::Invoke {
            configuration_ref,
            request: sent,
            external_context_id,
            ..
        } => {
            assert_eq!(configuration_ref, "pinned-remote-codex");
            assert_eq!(sent, &request);
            assert_eq!(external_context_id.as_ref(), Some(&context));
        }
        _ => panic!("Expected the new invocation on the replacement connection"),
    }
}

#[test]
fn active_connection_is_not_replaced_for_preflight_or_interaction() {
    let original = Arc::new(FakeConnection::default());
    let replacement = Arc::new(FakeConnection::default());
    let (runtime, attempts) = runtime(original.clone(), replacement.clone());
    let request = invocation("active");
    runtime
        .start_invocation(request.clone(), Arc::new(Sink))
        .unwrap();
    original.closed.store(true, Ordering::SeqCst);

    assert!(runtime
        .preflight_invocation(RuntimeInvocationMode::Start, &Default::default())
        .is_err());
    assert!(runtime.cancel_invocation(&request.invocation_id).is_err());
    assert!(runtime
        .respond(&request.invocation_id, "approval", RuntimeInteractionResponse::Choose { choice_id: "allow".into() })
        .is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 0);
    assert!(replacement.calls.lock().unwrap().is_empty());
    assert_eq!(original.calls.lock().unwrap().len(), 4);

    original.unregister(&request.invocation_id);
    runtime
        .preflight_invocation(RuntimeInvocationMode::Start, &Default::default())
        .unwrap();
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[test]
fn uncertain_invocation_is_never_replayed_and_later_user_invocation_can_connect() {
    let original = Arc::new(FakeConnection::default());
    original.disconnect_on_invoke.store(true, Ordering::SeqCst);
    let replacement = Arc::new(FakeConnection::default());
    let (runtime, attempts) = runtime(original.clone(), replacement.clone());

    assert!(runtime
        .start_invocation(invocation("uncertain"), Arc::new(Sink))
        .is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 0);
    assert!(replacement.calls.lock().unwrap().is_empty());
    assert_eq!(original.calls.lock().unwrap().len(), 1);

    runtime
        .start_invocation(invocation("next-user-send"), Arc::new(Sink))
        .unwrap();
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
    assert_eq!(original.calls.lock().unwrap().len(), 1);
    let calls = replacement.calls.lock().unwrap();
    assert_eq!(calls.len(), 1);
    assert!(matches!(&calls[0], HostCommand::Invoke { request, .. }
        if request.invocation_id.as_str() == "next-user-send"));
}

#[test]
fn healthy_idle_connection_is_reused_and_interaction_never_reconnects() {
    let original = Arc::new(FakeConnection::default());
    let replacement = Arc::new(FakeConnection::default());
    let (runtime, attempts) = runtime(original.clone(), replacement);
    runtime
        .preflight_invocation(RuntimeInvocationMode::Start, &Default::default())
        .unwrap();
    original.closed.store(true, Ordering::SeqCst);
    assert!(runtime
        .cancel_invocation(&invocation("finished").invocation_id)
        .is_err());
    assert_eq!(attempts.load(Ordering::SeqCst), 0);
    assert_eq!(original.calls.lock().unwrap().len(), 2);
}
