use super::{
    domain::{
        validate_new_invocation, validate_next_event, validate_runtime_binding_update,
        validate_session, AgentDiagnostic, AgentInvocation, AgentInvocationId,
        AgentInvocationInputProvenance, AgentInvocationStatus, AgentInvocationTerminalStatus,
        AgentRuntimeBinding, AgentRuntimeEvent, AgentRuntimeEventId, AgentRuntimeEventSource,
        AgentRuntimeOptions, AgentSession, AgentSessionAvailability, AgentSessionId,
        ContractViolation, ExternalRuntimeContextId, InvocationCompletion, NormalizedRuntimeEvent,
        NormalizedRuntimeEventKind,
    },
    ports::{
        AgentInvocationHistory, AgentRuntime, AgentRuntimeUpdateSink, AgentSessionHistory,
        AgentSessionRepository, AgentSessionSummary, ListAgentSessionsQuery, RepositoryError,
        RepositoryErrorKind, RuntimeEventDraft, RuntimeInvocationMode, RuntimeInvocationOutcome,
        RuntimeInvocationPreflight, RuntimeInvocationRequest, RuntimePortError, RuntimeUpdate,
        RuntimeUpdateDeliveryFailure,
    },
};
use chrono::{DateTime, Utc};
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

#[test]
fn session_serialization_keeps_local_and_external_identity_separate() {
    let session = session("session-local", Some("runtime-external"));

    let serialized = serde_json::to_value(session).expect("serialize session contract");

    assert_eq!(serialized["id"], "session-local");
    assert_eq!(
        serialized["runtimeBinding"]["externalContextId"],
        "runtime-external"
    );
    assert!(serialized["runtimeBinding"].get("kind").is_none());
    assert_eq!(serialized["availability"], "available");
    assert_eq!(serialized["workingDirectory"], "C:/work/session-local");
}

#[test]
fn identifiers_reject_empty_values_at_construction_and_deserialization() {
    assert_eq!(
        AgentSessionId::new("  "),
        Err(ContractViolation::EmptyIdentifier {
            kind: "agent session",
        })
    );
    assert!(serde_json::from_str::<AgentSessionId>(r#"""#).is_err());
}

#[test]
fn runtime_binding_can_be_established_idempotently_but_not_replaced() {
    let unbound = binding(None);
    let bound = binding(Some("runtime-1"));
    let rebound = binding(Some("runtime-2"));

    assert_eq!(validate_runtime_binding_update(&unbound, &bound), Ok(()));
    assert_eq!(validate_runtime_binding_update(&bound, &bound), Ok(()));
    assert_eq!(
        validate_runtime_binding_update(&bound, &unbound),
        Err(ContractViolation::ExternalRuntimeContextChanged)
    );
    assert_eq!(
        validate_runtime_binding_update(&bound, &rebound),
        Err(ContractViolation::ExternalRuntimeContextChanged)
    );
}

#[test]
fn session_validation_rejects_blank_title_and_present_but_blank_working_directory() {
    let mut blank_title = session("session-title", None);
    blank_title.title = "  \t".to_string();
    assert_eq!(
        validate_session(&blank_title),
        Err(ContractViolation::InvalidSessionRecord {
            reason: "session title cannot be blank",
        })
    );

    let mut blank_working_directory = session("session-working-directory", None);
    blank_working_directory.working_directory = Some(" \r\n ".to_string());
    assert_eq!(
        validate_session(&blank_working_directory),
        Err(ContractViolation::InvalidSessionRecord {
            reason: "session working directory cannot be blank when present",
        })
    );

    let mut no_working_directory = session("session-no-working-directory", None);
    no_working_directory.working_directory = None;
    assert_eq!(validate_session(&no_working_directory), Ok(()));
}

#[test]
fn invocation_lifecycle_accepts_proven_transitions_and_rejects_terminal_reentry() {
    let pending = invocation("invocation-1", "session-1", AgentInvocationStatus::Pending);
    let running = pending
        .mark_running(at(1), runtime_options(), at(1))
        .expect("pending to running");
    let completed = running
        .finish(
            InvocationCompletion {
                status: AgentInvocationTerminalStatus::Completed,
                completed_at: at(2),
                exit_code: Some(0),
                signal: None,
                runtime_error: None,
            },
            at(2),
        )
        .expect("running to completed");

    assert_eq!(completed.status, AgentInvocationStatus::Completed);
    assert!(completed.status.is_terminal());
    assert_eq!(completed.session_id, pending.session_id);
    assert_eq!(
        completed.finish(
            InvocationCompletion {
                status: AgentInvocationTerminalStatus::Failed,
                completed_at: at(3),
                exit_code: Some(1),
                signal: None,
                runtime_error: None,
            },
            at(3),
        ),
        Err(ContractViolation::InvalidInvocationTransition {
            from: AgentInvocationStatus::Completed,
            to: AgentInvocationStatus::Failed,
        })
    );

    for terminal_status in [
        AgentInvocationTerminalStatus::Failed,
        AgentInvocationTerminalStatus::Canceled,
        AgentInvocationTerminalStatus::Interrupted,
    ] {
        assert!(pending
            .finish(
                InvocationCompletion {
                    status: terminal_status,
                    completed_at: at(1),
                    exit_code: None,
                    signal: None,
                    runtime_error: None,
                },
                at(1),
            )
            .is_ok());
    }
}

#[test]
fn active_invocation_check_is_session_scoped() {
    let session_one = session("session-1", None);
    let session_two = session("session-2", None);
    let active = invocation("invocation-1", "session-1", AgentInvocationStatus::Pending);
    let same_session = invocation("invocation-2", "session-1", AgentInvocationStatus::Pending);
    let other_session = invocation("invocation-3", "session-2", AgentInvocationStatus::Pending);

    assert_eq!(
        validate_new_invocation(&session_one, Some(&active), &same_session),
        Err(ContractViolation::ActiveInvocationExists {
            invocation_id: invocation_id("invocation-1"),
        })
    );
    assert_eq!(
        validate_new_invocation(&session_two, None, &other_session),
        Ok(())
    );
}

#[test]
fn archived_session_rejects_new_invocation_without_changing_invocation_lifecycle() {
    let mut archived = session("session-archived", None);
    archived.availability = AgentSessionAvailability::Archived;
    let pending = invocation(
        "invocation-pending",
        "session-archived",
        AgentInvocationStatus::Pending,
    );

    assert_eq!(
        validate_new_invocation(&archived, None, &pending),
        Err(ContractViolation::ArchivedSessionCannotStartInvocation {
            session_id: session_id("session-archived"),
        })
    );
    assert_eq!(pending.status, AgentInvocationStatus::Pending);
}

#[test]
fn event_sequence_is_strictly_increasing_without_requiring_contiguous_numbers() {
    let previous = event("event-1", "invocation-1", 4);
    let candidate = event("event-2", "invocation-1", 9);
    let duplicate = event("event-3", "invocation-1", 4);

    assert_eq!(
        validate_next_event(&invocation_id("invocation-1"), Some(&previous), &candidate),
        Ok(())
    );
    assert_eq!(
        validate_next_event(&invocation_id("invocation-1"), Some(&previous), &duplicate),
        Err(ContractViolation::EventSequenceNotIncreasing {
            previous: 4,
            candidate: 4,
        })
    );
}

#[test]
fn fake_repository_proves_one_active_invocation_and_session_availability_are_independent() {
    let repository = FakeRepository::default();
    let session = session("session-1", None);
    repository
        .create_session(session.clone())
        .expect("create session");

    let first = repository
        .create_pending_invocation(invocation(
            "invocation-1",
            "session-1",
            AgentInvocationStatus::Pending,
        ))
        .expect("create first invocation");
    let conflict = repository
        .create_pending_invocation(invocation(
            "invocation-2",
            "session-1",
            AgentInvocationStatus::Pending,
        ))
        .expect_err("second active invocation must conflict");
    assert_eq!(conflict.kind, RepositoryErrorKind::Conflict);

    repository
        .mark_invocation_running(&first.id, at(1), runtime_options(), at(1))
        .expect("mark running");
    let completion = InvocationCompletion {
        status: AgentInvocationTerminalStatus::Completed,
        completed_at: at(2),
        exit_code: Some(0),
        signal: None,
        runtime_error: None,
    };
    let completed = repository
        .finish_invocation(&first.id, completion.clone(), at(2))
        .expect("complete invocation");
    assert_eq!(
        repository
            .finish_invocation(&first.id, completion, at(3))
            .expect("repeated completion is idempotent"),
        completed
    );

    let persisted_session = repository
        .get_session(&session.id)
        .expect("load session")
        .expect("session exists");
    assert_eq!(
        persisted_session.availability,
        AgentSessionAvailability::Available
    );
    repository
        .set_session_availability(&session.id, AgentSessionAvailability::Archived, at(3))
        .expect("archive session");
    assert_eq!(
        repository
            .create_pending_invocation(invocation(
                "invocation-2",
                "session-1",
                AgentInvocationStatus::Pending,
            ))
            .expect_err("archived session must reject a new invocation")
            .kind,
        RepositoryErrorKind::InvalidState
    );
    repository
        .set_session_availability(&session.id, AgentSessionAvailability::Available, at(4))
        .expect("restore session availability");
    assert!(repository
        .create_pending_invocation(invocation(
            "invocation-2",
            "session-1",
            AgentInvocationStatus::Pending,
        ))
        .is_ok());
}

#[test]
fn fake_runtime_requires_external_identity_only_for_resume_and_streams_updates() {
    let runtime = FakeRuntime::default();
    let sink = Arc::new(CollectingUpdateSink::default());
    let request = RuntimeInvocationRequest {
        session_id: session_id("session-local"),
        invocation_id: invocation_id("invocation-1"),
        submitted_text: "Continue the work".to_string(),
        working_directory: Some("C:/work/session-local".to_string()),
        options: runtime_options(),
        launch_extension: None,
    };

    let start_preflight = runtime
        .preflight_invocation(RuntimeInvocationMode::Start, &request.options)
        .expect("start preflight");
    runtime
        .start_invocation(request.clone(), sink.clone())
        .expect("start runtime");
    let resume_preflight = runtime
        .preflight_invocation(RuntimeInvocationMode::Resume, &request.options)
        .expect("resume preflight");
    runtime
        .resume_invocation(
            request.clone(),
            external_context_id("runtime-external"),
            sink.clone(),
        )
        .expect("resume runtime");
    runtime
        .cancel_invocation(&request.invocation_id)
        .expect("cancel runtime");

    assert_eq!(
        runtime.calls.lock().expect("runtime calls").as_slice(),
        &[
            FakeRuntimeCall::Start(request.clone()),
            FakeRuntimeCall::Resume(request.clone(), external_context_id("runtime-external")),
            FakeRuntimeCall::Cancel(request.invocation_id.clone()),
        ]
    );
    let updates = sink.updates.lock().expect("runtime updates");
    assert_eq!(updates.len(), 4);
    assert_eq!(updates[0].0, request.invocation_id);
    assert_eq!(updates[0].1, RuntimeUpdate::Event(runtime_event_draft()));
    assert_eq!(start_preflight.effective_options, request.options);
    assert_eq!(resume_preflight.effective_options, request.options);
    assert_eq!(updates[1].1, RuntimeUpdate::Finished(runtime_outcome()));
}

#[derive(Default)]
struct FakeRepository {
    state: Mutex<FakeRepositoryState>,
}

#[derive(Default)]
struct FakeRepositoryState {
    sessions: BTreeMap<AgentSessionId, AgentSession>,
    invocations: BTreeMap<AgentInvocationId, AgentInvocation>,
    launch_acceptances: BTreeMap<AgentInvocationId, DateTime<Utc>>,
    events: BTreeMap<AgentInvocationId, Vec<AgentRuntimeEvent>>,
}

impl AgentSessionRepository for FakeRepository {
    fn create_session(&self, session: AgentSession) -> Result<AgentSession, RepositoryError> {
        validate_session(&session).map_err(|error| {
            repository_error(RepositoryErrorKind::InvalidState, error.to_string())
        })?;
        let mut state = self.state.lock().expect("fake repository");
        if state.sessions.contains_key(&session.id) {
            return Err(repository_error(
                RepositoryErrorKind::Conflict,
                "session already exists",
            ));
        }
        state.sessions.insert(session.id.clone(), session.clone());
        Ok(session)
    }

    fn get_session(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSession>, RepositoryError> {
        Ok(self
            .state
            .lock()
            .expect("fake repository")
            .sessions
            .get(session_id)
            .cloned())
    }

    fn list_sessions(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSession>, RepositoryError> {
        let state = self.state.lock().expect("fake repository");
        let mut sessions = state
            .sessions
            .values()
            .filter(|session| {
                query
                    .availability
                    .is_none_or(|availability| session.availability == availability)
            })
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        if let Some(limit) = query.limit {
            sessions.truncate(limit as usize);
        }
        Ok(sessions)
    }

    fn load_session_history(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSessionHistory>, RepositoryError> {
        let state = self.state.lock().expect("fake repository");
        let Some(session) = state.sessions.get(session_id).cloned() else {
            return Ok(None);
        };
        let mut invocations = state
            .invocations
            .values()
            .filter(|invocation| &invocation.session_id == session_id)
            .cloned()
            .map(|invocation| AgentInvocationHistory {
                events: state
                    .events
                    .get(&invocation.id)
                    .cloned()
                    .unwrap_or_default(),
                invocation,
            })
            .collect::<Vec<_>>();
        invocations.sort_by(|left, right| {
            left.invocation
                .created_at
                .cmp(&right.invocation.created_at)
                .then_with(|| left.invocation.id.cmp(&right.invocation.id))
        });
        Ok(Some(AgentSessionHistory {
            session,
            invocations,
        }))
    }

    fn list_session_summaries(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSessionSummary>, RepositoryError> {
        let state = self.state.lock().expect("fake repository");
        let mut sessions = state
            .sessions
            .values()
            .filter(|session| {
                query
                    .availability
                    .is_none_or(|availability| session.availability == availability)
            })
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| {
            right
                .updated_at
                .cmp(&left.updated_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        if let Some(limit) = query.limit {
            sessions.truncate(limit as usize);
        }
        Ok(sessions
            .into_iter()
            .map(|session| {
                let mut invocations = state
                    .invocations
                    .values()
                    .filter(|invocation| invocation.session_id == session.id)
                    .collect::<Vec<_>>();
                invocations.sort_by(|left, right| {
                    right
                        .created_at
                        .cmp(&left.created_at)
                        .then_with(|| right.id.cmp(&left.id))
                });
                AgentSessionSummary {
                    session,
                    invocation_count: invocations.len() as u64,
                    latest_invocation_status: invocations.first().map(|value| value.status),
                    latest_submitted_text: invocations
                        .first()
                        .map(|value| value.submitted_text.clone()),
                }
            })
            .collect())
    }

    fn set_session_availability(
        &self,
        session_id: &AgentSessionId,
        availability: AgentSessionAvailability,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        let mut state = self.state.lock().expect("fake repository");
        let session = state
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| repository_error(RepositoryErrorKind::NotFound, "session not found"))?;
        session.availability = availability;
        session.updated_at = updated_at;
        Ok(session.clone())
    }

    fn update_runtime_binding(
        &self,
        session_id: &AgentSessionId,
        binding: AgentRuntimeBinding,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        let mut state = self.state.lock().expect("fake repository");
        let session = state
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| repository_error(RepositoryErrorKind::NotFound, "session not found"))?;
        validate_runtime_binding_update(&session.runtime_binding, &binding).map_err(|error| {
            repository_error(RepositoryErrorKind::InvalidState, error.to_string())
        })?;
        session.runtime_binding = binding;
        session.updated_at = updated_at;
        Ok(session.clone())
    }

    fn create_pending_invocation(
        &self,
        invocation: AgentInvocation,
    ) -> Result<AgentInvocation, RepositoryError> {
        let mut state = self.state.lock().expect("fake repository");
        let session = state
            .sessions
            .get(&invocation.session_id)
            .cloned()
            .ok_or_else(|| repository_error(RepositoryErrorKind::NotFound, "session not found"))?;
        let active = state
            .invocations
            .values()
            .find(|existing| {
                existing.session_id == invocation.session_id && existing.status.is_active()
            })
            .cloned();
        validate_new_invocation(&session, active.as_ref(), &invocation).map_err(|error| {
            let kind = match &error {
                ContractViolation::ActiveInvocationExists { .. } => RepositoryErrorKind::Conflict,
                _ => RepositoryErrorKind::InvalidState,
            };
            repository_error(kind, error.to_string())
        })?;
        state
            .invocations
            .insert(invocation.id.clone(), invocation.clone());
        Ok(invocation)
    }

    fn get_invocation(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<AgentInvocation>, RepositoryError> {
        Ok(self
            .state
            .lock()
            .expect("fake repository")
            .invocations
            .get(invocation_id)
            .cloned())
    }

    fn list_invocations(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Vec<AgentInvocation>, RepositoryError> {
        let mut invocations = self
            .state
            .lock()
            .expect("fake repository")
            .invocations
            .values()
            .filter(|invocation| &invocation.session_id == session_id)
            .cloned()
            .collect::<Vec<_>>();
        invocations.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(invocations)
    }

    fn mark_invocation_running(
        &self,
        invocation_id: &AgentInvocationId,
        started_at: DateTime<Utc>,
        effective_options: AgentRuntimeOptions,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        let mut state = self.state.lock().expect("fake repository");
        let invocation = state.invocations.get_mut(invocation_id).ok_or_else(|| {
            repository_error(RepositoryErrorKind::NotFound, "invocation not found")
        })?;
        let updated = invocation
            .mark_running(started_at, effective_options, updated_at)
            .map_err(|error| {
                repository_error(RepositoryErrorKind::InvalidState, error.to_string())
            })?;
        *invocation = updated.clone();
        Ok(updated)
    }

    fn record_invocation_launch_accepted(
        &self,
        invocation_id: &AgentInvocationId,
        accepted_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let mut state = self.state.lock().expect("fake repository");
        if !state.invocations.contains_key(invocation_id) {
            return Err(repository_error(
                RepositoryErrorKind::NotFound,
                "invocation not found",
            ));
        }
        state
            .launch_acceptances
            .entry(invocation_id.clone())
            .or_insert(accepted_at);
        Ok(())
    }

    fn invocation_launch_accepted_at(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        Ok(self
            .state
            .lock()
            .expect("fake repository")
            .launch_acceptances
            .get(invocation_id)
            .copied())
    }

    fn finish_invocation(
        &self,
        invocation_id: &AgentInvocationId,
        completion: InvocationCompletion,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        let mut state = self.state.lock().expect("fake repository");
        let invocation = state.invocations.get_mut(invocation_id).ok_or_else(|| {
            repository_error(RepositoryErrorKind::NotFound, "invocation not found")
        })?;
        let requested_status = AgentInvocationStatus::from(completion.status);
        if invocation.status == requested_status
            && invocation.completed_at == Some(completion.completed_at)
            && invocation.exit_code == completion.exit_code
            && invocation.signal == completion.signal
            && invocation.runtime_error == completion.runtime_error
        {
            return Ok(invocation.clone());
        }
        let updated = invocation.finish(completion, updated_at).map_err(|error| {
            repository_error(RepositoryErrorKind::InvalidState, error.to_string())
        })?;
        *invocation = updated.clone();
        Ok(updated)
    }

    fn append_invocation_diagnostic(
        &self,
        invocation_id: &AgentInvocationId,
        diagnostic: AgentDiagnostic,
    ) -> Result<AgentInvocation, RepositoryError> {
        let mut state = self.state.lock().expect("fake repository");
        let invocation = state.invocations.get_mut(invocation_id).ok_or_else(|| {
            repository_error(RepositoryErrorKind::NotFound, "invocation not found")
        })?;
        invocation.diagnostics.push(diagnostic);
        Ok(invocation.clone())
    }

    fn append_event(&self, event: AgentRuntimeEvent) -> Result<AgentRuntimeEvent, RepositoryError> {
        let mut state = self.state.lock().expect("fake repository");
        if !state.invocations.contains_key(&event.invocation_id) {
            return Err(repository_error(
                RepositoryErrorKind::NotFound,
                "invocation not found",
            ));
        }
        let events = state.events.entry(event.invocation_id.clone()).or_default();
        validate_next_event(&event.invocation_id, events.last(), &event)
            .map_err(|error| repository_error(RepositoryErrorKind::Conflict, error.to_string()))?;
        events.push(event.clone());
        Ok(event)
    }

    fn list_events(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Vec<AgentRuntimeEvent>, RepositoryError> {
        Ok(self
            .state
            .lock()
            .expect("fake repository")
            .events
            .get(invocation_id)
            .cloned()
            .unwrap_or_default())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FakeRuntimeCall {
    Start(RuntimeInvocationRequest),
    Resume(RuntimeInvocationRequest, ExternalRuntimeContextId),
    Cancel(AgentInvocationId),
}

#[derive(Default)]
struct FakeRuntime {
    calls: Mutex<Vec<FakeRuntimeCall>>,
}

impl AgentRuntime for FakeRuntime {
    fn preflight_invocation(
        &self,
        _mode: RuntimeInvocationMode,
        requested_options: &AgentRuntimeOptions,
    ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
        Ok(RuntimeInvocationPreflight {
            effective_options: requested_options.clone(),
        })
    }

    fn start_invocation(
        &self,
        request: RuntimeInvocationRequest,
        update_sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.calls
            .lock()
            .expect("runtime calls")
            .push(FakeRuntimeCall::Start(request.clone()));
        emit_fake_runtime_updates(update_sink, &request.invocation_id)?;
        Ok(())
    }

    fn resume_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external_context_id: ExternalRuntimeContextId,
        update_sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.calls
            .lock()
            .expect("runtime calls")
            .push(FakeRuntimeCall::Resume(
                request.clone(),
                external_context_id,
            ));
        emit_fake_runtime_updates(update_sink, &request.invocation_id)?;
        Ok(())
    }

    fn cancel_invocation(&self, invocation_id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        self.calls
            .lock()
            .expect("runtime calls")
            .push(FakeRuntimeCall::Cancel(invocation_id.clone()));
        Ok(())
    }
}

#[derive(Default)]
struct CollectingUpdateSink {
    updates: Mutex<Vec<(AgentInvocationId, RuntimeUpdate)>>,
}

impl AgentRuntimeUpdateSink for CollectingUpdateSink {
    fn emit_update(
        &self,
        invocation_id: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        self.updates
            .lock()
            .expect("runtime updates")
            .push((invocation_id.clone(), update));
        Ok(())
    }

    fn report_delivery_failure(
        &self,
        _invocation_id: &AgentInvocationId,
        _failure: RuntimeUpdateDeliveryFailure,
    ) {
    }
}

fn session(id: &str, external_context_id: Option<&str>) -> AgentSession {
    AgentSession {
        id: session_id(id),
        title: "Agent session".to_string(),
        availability: AgentSessionAvailability::Available,
        runtime_binding: binding(external_context_id),
        working_directory: Some(format!("C:/work/{id}")),
        requested_options: runtime_options(),
        created_at: at(0),
        updated_at: at(0),
    }
}

fn binding(external_context_id_value: Option<&str>) -> AgentRuntimeBinding {
    AgentRuntimeBinding {
        external_context_id: external_context_id_value.map(external_context_id),
        runtime_version: Some("runtime-test".to_string()),
    }
}

fn invocation(id: &str, session: &str, status: AgentInvocationStatus) -> AgentInvocation {
    AgentInvocation {
        id: invocation_id(id),
        session_id: session_id(session),
        submitted_text: "Do the work".to_string(),
        input_provenance: AgentInvocationInputProvenance::User,
        status,
        requested_options: runtime_options(),
        effective_options: None,
        started_at: None,
        completed_at: None,
        exit_code: None,
        signal: None,
        runtime_error: None,
        diagnostics: Vec::new(),
        created_at: at(0),
        updated_at: at(0),
    }
}

fn event(id: &str, invocation: &str, sequence: u64) -> AgentRuntimeEvent {
    AgentRuntimeEvent {
        id: AgentRuntimeEventId::new(id).expect("event ID"),
        invocation_id: invocation_id(invocation),
        sequence,
        source: AgentRuntimeEventSource::Stdout,
        raw_payload: json!({"provider": "raw"}),
        normalized: Some(NormalizedRuntimeEvent {
            kind: NormalizedRuntimeEventKind::ProcessingUpdate,
            text: Some("working".to_string()),
            external_context_id: None,
            usage: None,
            details: None,
        }),
        recorded_at: at(1),
    }
}

fn runtime_event_draft() -> RuntimeEventDraft {
    RuntimeEventDraft {
        source: AgentRuntimeEventSource::Stdout,
        raw_payload: json!({"provider": "raw"}),
        normalized: Some(NormalizedRuntimeEvent {
            kind: NormalizedRuntimeEventKind::ProcessingUpdate,
            text: Some("working".to_string()),
            external_context_id: None,
            usage: None,
            details: None,
        }),
    }
}

fn runtime_outcome() -> RuntimeInvocationOutcome {
    RuntimeInvocationOutcome {
        status: AgentInvocationTerminalStatus::Completed,
        exit_code: Some(0),
        signal: None,
        runtime_error: None,
    }
}

fn emit_fake_runtime_updates(
    sink: Arc<dyn AgentRuntimeUpdateSink>,
    invocation_id: &AgentInvocationId,
) -> Result<(), RuntimePortError> {
    sink.emit_update(invocation_id, RuntimeUpdate::Event(runtime_event_draft()))?;
    sink.emit_update(invocation_id, RuntimeUpdate::Finished(runtime_outcome()))
}

fn runtime_options() -> AgentRuntimeOptions {
    AgentRuntimeOptions {
        model: Some("test-model".to_string()),
        sandbox: None,
    }
}

fn session_id(value: &str) -> AgentSessionId {
    AgentSessionId::new(value).expect("session ID")
}

fn invocation_id(value: &str) -> AgentInvocationId {
    AgentInvocationId::new(value).expect("invocation ID")
}

fn external_context_id(value: &str) -> ExternalRuntimeContextId {
    ExternalRuntimeContextId::new(value).expect("external context ID")
}

fn at(second: u32) -> DateTime<Utc> {
    format!("2026-07-10T12:00:{second:02}Z")
        .parse()
        .expect("test timestamp")
}

fn repository_error(kind: RepositoryErrorKind, message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(kind, message)
}
