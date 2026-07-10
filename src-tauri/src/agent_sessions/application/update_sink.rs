use super::lifecycle::{
    AgentSessionClock, AgentSessionIdProvider, AgentSessionNotification, AgentSessionNotifier,
};
use crate::agent_sessions::{
    domain::{
        AgentDiagnostic, AgentDiagnosticSeverity, AgentDiagnosticSource, AgentInvocationId,
        AgentRuntimeEvent, InvocationCompletion, NormalizedRuntimeEventKind,
    },
    ports::{
        AgentRuntimeUpdateSink, AgentSessionRepository, RepositoryError, RuntimePortError,
        RuntimePortErrorKind, RuntimeUpdate, RuntimeUpdateDeliveryFailure,
    },
};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub(super) struct InvocationUpdateLanes {
    lanes: Mutex<HashMap<AgentInvocationId, Arc<Mutex<InvocationUpdateState>>>>,
}

#[derive(Default)]
struct InvocationUpdateState {
    next_sequence: Option<u64>,
}

impl InvocationUpdateLanes {
    fn lane(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Arc<Mutex<InvocationUpdateState>>, RuntimePortError> {
        let mut lanes = self
            .lanes
            .lock()
            .map_err(|_| delivery_error("Agent Session update lane registry is poisoned", None))?;
        Ok(lanes
            .entry(invocation_id.clone())
            .or_insert_with(|| Arc::new(Mutex::new(InvocationUpdateState::default())))
            .clone())
    }
}

pub(super) struct PersistedRuntimeUpdateSink {
    repository: Arc<dyn AgentSessionRepository>,
    notifier: Arc<dyn AgentSessionNotifier>,
    clock: Arc<dyn AgentSessionClock>,
    ids: Arc<dyn AgentSessionIdProvider>,
    update_lanes: Arc<InvocationUpdateLanes>,
}

impl PersistedRuntimeUpdateSink {
    pub(super) fn new(
        repository: Arc<dyn AgentSessionRepository>,
        notifier: Arc<dyn AgentSessionNotifier>,
        clock: Arc<dyn AgentSessionClock>,
        ids: Arc<dyn AgentSessionIdProvider>,
        update_lanes: Arc<InvocationUpdateLanes>,
    ) -> Self {
        Self {
            repository,
            notifier,
            clock,
            ids,
            update_lanes,
        }
    }
}

impl AgentRuntimeUpdateSink for PersistedRuntimeUpdateSink {
    fn emit_update(
        &self,
        invocation_id: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        let lane = self.update_lanes.lane(invocation_id)?;
        let mut state = lane.lock().map_err(|_| {
            delivery_error("Agent Session invocation update lane is poisoned", None)
        })?;
        let invocation = self
            .repository
            .get_invocation(invocation_id)
            .map_err(repository_delivery_error(
                "load invocation for runtime update",
            ))?
            .ok_or_else(|| {
                delivery_error("runtime update references an unknown invocation", None)
            })?;

        match update {
            RuntimeUpdate::Event(draft) => {
                let sequence = match state.next_sequence {
                    Some(sequence) => sequence,
                    None => self
                        .repository
                        .list_events(invocation_id)
                        .map_err(repository_delivery_error("load runtime event sequence"))?
                        .last()
                        .map_or(0, |event| event.sequence.saturating_add(1)),
                };
                let recorded_at = self.clock.now();
                let event = AgentRuntimeEvent {
                    id: self.ids.event_id(),
                    invocation_id: invocation_id.clone(),
                    sequence,
                    source: draft.source,
                    raw_payload: draft.raw_payload,
                    normalized: draft.normalized,
                    recorded_at,
                };
                let event = self
                    .repository
                    .append_event(event)
                    .map_err(repository_delivery_error("append runtime event"))?;
                state.next_sequence = Some(sequence.saturating_add(1));

                if let Some(external_context_id) = event
                    .normalized
                    .as_ref()
                    .filter(|normalized| {
                        normalized.kind == NormalizedRuntimeEventKind::RuntimeContextEstablished
                    })
                    .and_then(|normalized| normalized.external_context_id.clone())
                {
                    let session = self
                        .repository
                        .get_session(&invocation.session_id)
                        .map_err(repository_delivery_error("load runtime binding"))?
                        .ok_or_else(|| delivery_error("Agent Session not found", None))?;
                    let mut binding = session.runtime_binding;
                    binding.external_context_id = Some(external_context_id);
                    self.repository
                        .update_runtime_binding(&invocation.session_id, binding, recorded_at)
                        .map_err(repository_delivery_error("persist runtime binding"))?;
                }

                self.notifier
                    .notify(AgentSessionNotification::EventPersisted {
                        session_id: invocation.session_id,
                        event,
                    })
                    .map_err(|error| {
                        delivery_error(
                            "emit persisted runtime event",
                            Some(json!({"transportError": error})),
                        )
                    })
            }
            RuntimeUpdate::Finished(outcome) => {
                if invocation.status.is_terminal() {
                    return Ok(());
                }
                let completed_at = self.clock.now();
                let invocation = self
                    .repository
                    .finish_invocation(
                        invocation_id,
                        InvocationCompletion {
                            status: outcome.status,
                            completed_at,
                            exit_code: outcome.exit_code,
                            signal: outcome.signal,
                            runtime_error: outcome.runtime_error,
                        },
                        completed_at,
                    )
                    .map_err(repository_delivery_error(
                        "persist runtime terminal outcome",
                    ))?;
                self.notifier
                    .notify(AgentSessionNotification::InvocationTerminal {
                        session_id: invocation.session_id.clone(),
                        invocation,
                    })
                    .map_err(|error| {
                        delivery_error(
                            "emit persisted terminal outcome",
                            Some(json!({"transportError": error})),
                        )
                    })
            }
        }
    }

    fn report_delivery_failure(
        &self,
        invocation_id: &AgentInvocationId,
        failure: RuntimeUpdateDeliveryFailure,
    ) {
        let diagnostic = AgentDiagnostic {
            source: if failure
                .error
                .details
                .as_ref()
                .is_some_and(|details| details.get("transportError").is_some())
            {
                AgentDiagnosticSource::Transport
            } else {
                AgentDiagnosticSource::Repository
            },
            severity: AgentDiagnosticSeverity::Error,
            code: "runtime_update_delivery_failed".to_string(),
            message: failure.error.message.clone(),
            details: Some(json!({
                "failedUpdate": failure.update,
                "deliveryError": failure.error,
            })),
            recorded_at: self.clock.now(),
        };
        let Ok(invocation) = self
            .repository
            .append_invocation_diagnostic(invocation_id, diagnostic)
        else {
            return;
        };
        // This is deliberately one bounded best-effort report. A failure here is ignored rather
        // than recursively entering runtime update delivery again.
        let _ = self
            .notifier
            .notify(AgentSessionNotification::DiagnosticRecorded {
                session_id: invocation.session_id.clone(),
                invocation,
            });
    }
}

fn repository_delivery_error(
    stage: &'static str,
) -> impl FnOnce(RepositoryError) -> RuntimePortError {
    move |error| delivery_error(stage, Some(json!({"repositoryError": error.message})))
}

fn delivery_error(message: &str, details: Option<Value>) -> RuntimePortError {
    let error = RuntimePortError::new(RuntimePortErrorKind::EventDeliveryFailed, message);
    match details {
        Some(details) => error.with_details(details),
        None => error,
    }
}
