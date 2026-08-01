//! Application-owned user confirmation boundary for Epic initiation.

use super::{
    application::OrchestrationApplication,
    domain::{InitiateEpicCommand, InitiateEpicError, InitiateEpicResult},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

pub(crate) const INITIATION_CONFIRMATION_EVENT: &str =
    "orchestration://epic-initiation-confirmation";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum InitiationRequestSource {
    Button,
    Agent {
        agent_session_id: String,
        agent_invocation_id: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InitiationConfirmationState {
    Requested,
    UserConfirmed,
    UserRejected,
    Applied,
    Persisted,
    Projected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitiationConfirmationRequest {
    pub(crate) request_id: String,
    pub(crate) source: InitiationRequestSource,
    pub(crate) epic_planning_draft_id: String,
    pub(crate) state: InitiationConfirmationState,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitiationConfirmationEvent {
    pub(crate) request: InitiationConfirmationRequest,
    pub(crate) state: InitiationConfirmationState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) initiation: Option<InitiateEpicResult>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UserInitiationDecision {
    Confirmed,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InitiationConfirmationResolution {
    pub(crate) request_id: String,
    pub(crate) state: InitiationConfirmationState,
    pub(crate) initiation: Option<InitiateEpicResult>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InitiationConfirmationError {
    Rejected,
    RejectedNotificationFailed(String),
    ConfirmedButNotApplied(String),
    RequestNotFound,
    TimedOut,
    Apply(InitiateEpicError),
    PersistedButIncomplete {
        initiation: InitiateEpicResult,
        projected: Option<bool>,
        reason: String,
    },
    Unavailable(String),
}

pub(crate) trait InitiationConfirmationNotifier: Send + Sync {
    fn notify(&self, event: InitiationConfirmationEvent) -> Result<(), String>;
}

pub(crate) trait PersistedInitiationObserver: Send + Sync {
    fn on_persisted_initiation(&self, initiation: &InitiateEpicResult) -> Result<(), String>;
}

pub(crate) trait ButtonInitiationContextScheduler: Send + Sync {
    fn schedule(&self, initiation: &InitiateEpicResult) -> Result<(), String>;
}

struct PendingRequest {
    request: InitiationConfirmationRequest,
    command: InitiateEpicCommand,
    completion: Arc<(
        Mutex<Option<Result<InitiationConfirmationResolution, InitiationConfirmationError>>>,
        Condvar,
    )>,
}

#[derive(Default)]
struct ConfirmationRegistry {
    pending: HashMap<String, Arc<PendingRequest>>,
    idempotency: HashMap<String, String>,
}

pub(crate) struct InitiationConfirmationCoordinator {
    application: Arc<OrchestrationApplication>,
    notifier: Arc<dyn InitiationConfirmationNotifier>,
    registry: Mutex<ConfirmationRegistry>,
    persisted_observer: Mutex<Option<Arc<dyn PersistedInitiationObserver>>>,
    button_context_scheduler: Mutex<Option<Arc<dyn ButtonInitiationContextScheduler>>>,
}

impl InitiationConfirmationCoordinator {
    pub(crate) fn new(
        application: Arc<OrchestrationApplication>,
        notifier: Arc<dyn InitiationConfirmationNotifier>,
    ) -> Arc<Self> {
        Arc::new(Self {
            application,
            notifier,
            registry: Mutex::new(ConfirmationRegistry::default()),
            persisted_observer: Mutex::new(None),
            button_context_scheduler: Mutex::new(None),
        })
    }

    pub(crate) fn set_button_context_scheduler(
        &self,
        scheduler: Arc<dyn ButtonInitiationContextScheduler>,
    ) -> Result<(), String> {
        let mut current = self
            .button_context_scheduler
            .lock()
            .map_err(|_| "button initiation context scheduler is unavailable".to_string())?;
        if current.is_some() {
            return Err("button initiation context scheduler is already registered".into());
        }
        *current = Some(scheduler);
        Ok(())
    }

    pub(crate) fn set_persisted_observer(
        &self,
        observer: Arc<dyn PersistedInitiationObserver>,
    ) -> Result<(), String> {
        let mut current = self
            .persisted_observer
            .lock()
            .map_err(|_| "persisted initiation observer registry is unavailable".to_string())?;
        if current.is_some() {
            return Err("persisted initiation observer is already registered".into());
        }
        *current = Some(observer);
        Ok(())
    }

    pub(crate) fn request(
        &self,
        source: InitiationRequestSource,
        command: InitiateEpicCommand,
    ) -> Result<InitiationConfirmationRequest, InitiationConfirmationError> {
        command
            .validate()
            .map_err(InitiationConfirmationError::Apply)?;
        let mut registry = self.registry.lock().map_err(|_| {
            InitiationConfirmationError::Unavailable("confirmation registry is unavailable".into())
        })?;
        if let Some(request_id) = registry.idempotency.get(&command.idempotency_key) {
            let existing = registry.pending.get(request_id).ok_or_else(|| {
                InitiationConfirmationError::Unavailable(
                    "confirmation registry is inconsistent".into(),
                )
            })?;
            if existing.command != command || existing.request.source != source {
                return Err(InitiationConfirmationError::Apply(
                    InitiateEpicError::IdempotencyConflict,
                ));
            }
            return Ok(existing.request.clone());
        }
        let request_id = format!("epic-initiation-confirmation-{}", uuid::Uuid::new_v4());
        let request = InitiationConfirmationRequest {
            request_id: request_id.clone(),
            source,
            epic_planning_draft_id: command.epic_planning_draft_id.as_str().to_string(),
            state: InitiationConfirmationState::Requested,
        };
        let pending = Arc::new(PendingRequest {
            request: request.clone(),
            command: command.clone(),
            completion: Arc::new((Mutex::new(None), Condvar::new())),
        });
        registry.pending.insert(request_id.clone(), pending.clone());
        registry
            .idempotency
            .insert(command.idempotency_key.clone(), request_id.clone());
        if let Err(reason) = self.publish(&request, InitiationConfirmationState::Requested, None) {
            registry.pending.remove(&request_id);
            registry.idempotency.remove(&command.idempotency_key);
            return Err(InitiationConfirmationError::Unavailable(reason));
        }
        Ok(request)
    }

    pub(crate) fn resolve(
        &self,
        request_id: &str,
        decision: UserInitiationDecision,
    ) -> Result<InitiationConfirmationResolution, InitiationConfirmationError> {
        let pending = self
            .registry
            .lock()
            .map_err(|_| {
                InitiationConfirmationError::Unavailable(
                    "confirmation registry is unavailable".into(),
                )
            })?
            .pending
            .get(request_id)
            .cloned()
            .ok_or(InitiationConfirmationError::RequestNotFound)?;
        let (completion, ready) = &*pending.completion;
        let mut completion = completion.lock().map_err(|_| {
            InitiationConfirmationError::Unavailable("confirmation result is unavailable".into())
        })?;
        if let Some(existing) = completion.as_ref() {
            return existing.clone();
        }
        let result = match decision {
            UserInitiationDecision::Rejected => {
                match self.publish(
                    &pending.request,
                    InitiationConfirmationState::UserRejected,
                    None,
                ) {
                    Ok(()) => Err(InitiationConfirmationError::Rejected),
                    Err(reason) => Err(InitiationConfirmationError::RejectedNotificationFailed(
                        reason,
                    )),
                }
            }
            UserInitiationDecision::Confirmed => {
                if let Err(reason) = self.publish(
                    &pending.request,
                    InitiationConfirmationState::UserConfirmed,
                    None,
                ) {
                    Err(InitiationConfirmationError::ConfirmedButNotApplied(reason))
                } else {
                    self.apply_confirmed(request_id, &pending)
                }
            }
        };
        *completion = Some(result.clone());
        ready.notify_all();
        result
    }

    pub(crate) fn wait_for_resolution(
        &self,
        request_id: &str,
        timeout: Duration,
    ) -> Result<InitiationConfirmationResolution, InitiationConfirmationError> {
        let pending = self
            .registry
            .lock()
            .map_err(|_| {
                InitiationConfirmationError::Unavailable(
                    "confirmation registry is unavailable".into(),
                )
            })?
            .pending
            .get(request_id)
            .cloned()
            .ok_or(InitiationConfirmationError::RequestNotFound)?;
        let (completion, ready) = &*pending.completion;
        let completion = completion.lock().map_err(|_| {
            InitiationConfirmationError::Unavailable("confirmation result is unavailable".into())
        })?;
        let (mut completion, wait) = ready
            .wait_timeout_while(completion, timeout, |value| value.is_none())
            .map_err(|_| {
                InitiationConfirmationError::Unavailable("confirmation wait is unavailable".into())
            })?;
        if wait.timed_out() && completion.is_none() {
            *completion = Some(Err(InitiationConfirmationError::TimedOut));
            ready.notify_all();
            drop(completion);
            self.remove_registration(&pending);
            return Err(InitiationConfirmationError::TimedOut);
        }
        completion
            .as_ref()
            .cloned()
            .ok_or(InitiationConfirmationError::TimedOut)?
    }

    fn apply_confirmed(
        &self,
        request_id: &str,
        pending: &PendingRequest,
    ) -> Result<InitiationConfirmationResolution, InitiationConfirmationError> {
        let initiation = self
            .application
            .initiate_epic(pending.command.clone())
            .map_err(InitiationConfirmationError::Apply)?;
        let mut notification_failures = Vec::new();
        let observer = self
            .persisted_observer
            .lock()
            .map_err(|_| InitiationConfirmationError::PersistedButIncomplete {
                initiation: initiation.clone(),
                projected: None,
                reason: "persisted initiation observer registry is unavailable".into(),
            })?
            .clone();
        if let Some(observer) = observer {
            if let Err(reason) = observer.on_persisted_initiation(&initiation) {
                notification_failures.push(format!(
                    "post-confirmation transition callback failed: {reason}"
                ));
            }
        }
        for state in [
            InitiationConfirmationState::Applied,
            InitiationConfirmationState::Persisted,
        ] {
            if let Err(reason) = self.publish(&pending.request, state, Some(initiation.clone())) {
                notification_failures.push(format!("{state:?} notification failed: {reason}"));
            }
        }
        let projected = match self
            .application
            .initiation_is_projected(&initiation.initiation_id)
        {
            Ok(true) => {
                if let Err(reason) = self.publish(
                    &pending.request,
                    InitiationConfirmationState::Projected,
                    Some(initiation.clone()),
                ) {
                    notification_failures.push(format!("Projected notification failed: {reason}"));
                }
                Some(true)
            }
            Ok(false) => {
                notification_failures.push("persisted initiation was not projected".into());
                Some(false)
            }
            Err(reason) => {
                notification_failures.push(format!("projection observation failed: {reason}"));
                None
            }
        };
        if projected == Some(true)
            && matches!(pending.request.source, InitiationRequestSource::Button)
        {
            let scheduler = self
                .button_context_scheduler
                .lock()
                .map_err(|_| InitiationConfirmationError::PersistedButIncomplete {
                    initiation: initiation.clone(),
                    projected,
                    reason: "button initiation context scheduler is unavailable".into(),
                })?
                .clone();
            match scheduler {
                Some(scheduler) => {
                    if let Err(reason) = scheduler.schedule(&initiation) {
                        notification_failures.push(format!(
                            "button initiation context scheduling failed: {reason}"
                        ));
                    }
                }
                None => notification_failures
                    .push("button initiation context scheduler is not registered".into()),
            }
        }
        if !notification_failures.is_empty() {
            return Err(InitiationConfirmationError::PersistedButIncomplete {
                initiation,
                projected,
                reason: notification_failures.join("; "),
            });
        }
        Ok(InitiationConfirmationResolution {
            request_id: request_id.to_string(),
            state: InitiationConfirmationState::Projected,
            initiation: Some(initiation),
        })
    }

    fn remove_registration(&self, pending: &PendingRequest) {
        let Ok(mut registry) = self.registry.lock() else {
            return;
        };
        registry.pending.remove(&pending.request.request_id);
        if registry
            .idempotency
            .get(&pending.command.idempotency_key)
            .is_some_and(|request_id| request_id == &pending.request.request_id)
        {
            registry
                .idempotency
                .remove(&pending.command.idempotency_key);
        }
    }

    fn publish(
        &self,
        request: &InitiationConfirmationRequest,
        state: InitiationConfirmationState,
        initiation: Option<InitiateEpicResult>,
    ) -> Result<(), String> {
        self.notifier.notify(InitiationConfirmationEvent {
            request: request.clone(),
            state,
            initiation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{
        domain::{PlanBuilderProposal, ProposedSprint, SaveEpicPlanProposalCommand},
        repository::{
            SqliteOrchestrationRepository, FILE_REVIEW_FACTS_SCHEMA,
            ORCHESTRATION_INITIATION_SCHEMA, ORCHESTRATION_SCHEMA,
        },
    };
    use chrono::{TimeZone, Utc};
    use rusqlite::Connection;
    use std::{
        sync::{mpsc, Barrier},
        thread,
    };

    struct ChannelNotifier(Mutex<mpsc::Sender<InitiationConfirmationEvent>>);
    impl InitiationConfirmationNotifier for ChannelNotifier {
        fn notify(&self, event: InitiationConfirmationEvent) -> Result<(), String> {
            self.0
                .lock()
                .map_err(|_| "channel notifier is unavailable".to_string())?
                .send(event)
                .map_err(|error| error.to_string())
        }
    }

    #[derive(Default)]
    struct ScriptedNotifier {
        state: Mutex<ScriptedNotifierState>,
    }

    #[derive(Default)]
    struct ScriptedNotifierState {
        fail_once: Vec<InitiationConfirmationState>,
        attempts: Vec<InitiationConfirmationEvent>,
        visible: Vec<InitiationConfirmationEvent>,
    }

    impl ScriptedNotifier {
        fn failing_once(state: InitiationConfirmationState) -> Arc<Self> {
            Arc::new(Self {
                state: Mutex::new(ScriptedNotifierState {
                    fail_once: vec![state],
                    ..ScriptedNotifierState::default()
                }),
            })
        }

        fn attempts(&self) -> Vec<InitiationConfirmationEvent> {
            self.state.lock().unwrap().attempts.clone()
        }

        fn visible(&self) -> Vec<InitiationConfirmationEvent> {
            self.state.lock().unwrap().visible.clone()
        }
    }

    impl InitiationConfirmationNotifier for ScriptedNotifier {
        fn notify(&self, event: InitiationConfirmationEvent) -> Result<(), String> {
            let mut state = self.state.lock().unwrap();
            state.attempts.push(event.clone());
            if let Some(index) = state
                .fail_once
                .iter()
                .position(|candidate| *candidate == event.state)
            {
                state.fail_once.remove(index);
                return Err(format!("{:?} notification unavailable", event.state));
            }
            state.visible.push(event);
            Ok(())
        }
    }

    fn fixture_with_notifier(
        notifier: Arc<dyn InitiationConfirmationNotifier>,
    ) -> (
        Arc<InitiationConfirmationCoordinator>,
        Arc<SqliteOrchestrationRepository>,
        InitiateEpicCommand,
    ) {
        let connection = Connection::open_in_memory().unwrap();
        crate::storage::configure_sqlite_connection(&connection).unwrap();
        connection
            .execute_batch(crate::agent_sessions::repository::AGENT_SESSION_SCHEMA)
            .unwrap();
        connection.execute_batch(ORCHESTRATION_SCHEMA).unwrap();
        connection.execute_batch(FILE_REVIEW_FACTS_SCHEMA).unwrap();
        connection
            .execute_batch(ORCHESTRATION_INITIATION_SCHEMA)
            .unwrap();
        connection
            .execute_batch(super::super::repository::PLAN_BUILDER_CONTEXT_DELIVERY_SCHEMA)
            .unwrap();
        let now = Utc.with_ymd_and_hms(2026, 7, 16, 10, 0, 0).unwrap();
        connection.execute("INSERT INTO agent_sessions (id, title, availability, requested_options_json, created_at, updated_at) VALUES ('session-confirmation', 'session', 'available', '{}', ?1, ?1)", [now.to_rfc3339()]).unwrap();
        let repository = Arc::new(SqliteOrchestrationRepository::new(connection).unwrap());
        let (draft, profile, association) = repository
            .bootstrap_managed_plan_builder("session-confirmation")
            .unwrap();
        let saved = repository
            .save_epic_plan_proposal(SaveEpicPlanProposalCommand {
                epic_planning_draft_id: draft.clone(),
                capability_profile_id: profile,
                agent_session_association_id: association,
                agent_session_id: "session-confirmation".into(),
                actor_id: "managed-plan-builder".into(),
                expected_revision: None,
                proposal: PlanBuilderProposal {
                    suggested_epic_name: Some("Confirmation Epic".into()),
                    sprints: vec![ProposedSprint {
                        title: "Prepared Sprint".into(),
                        intended_movement: "Prepare the durable transition.".into(),
                        concern_summaries: vec![],
                    }],
                },
                idempotency_key: "confirmation-proposal".into(),
            })
            .unwrap();
        let application = Arc::new(OrchestrationApplication::new(repository.clone()));
        let coordinator = InitiationConfirmationCoordinator::new(application.clone(), notifier);
        coordinator
            .set_button_context_scheduler(application)
            .unwrap();
        (
            coordinator,
            repository,
            InitiateEpicCommand {
                epic_planning_draft_id: draft,
                expected_revision_token: saved.revision_token,
                actor_id: "application-user".into(),
                idempotency_key: "button-confirmation".into(),
            },
        )
    }

    fn fixture() -> (
        Arc<InitiationConfirmationCoordinator>,
        mpsc::Receiver<InitiationConfirmationEvent>,
        Arc<SqliteOrchestrationRepository>,
        InitiateEpicCommand,
    ) {
        let (sender, receiver) = mpsc::channel();
        let (coordinator, repository, command) =
            fixture_with_notifier(Arc::new(ChannelNotifier(Mutex::new(sender))));
        (coordinator, receiver, repository, command)
    }

    fn initiated_count(repository: &SqliteOrchestrationRepository) -> usize {
        serde_json::to_value(repository.native_query().unwrap()).unwrap()["initiatedEpics"]
            .as_array()
            .unwrap()
            .len()
    }

    fn initiation_command_count(repository: &SqliteOrchestrationRepository) -> usize {
        serde_json::to_value(repository.native_query().unwrap()).unwrap()["initiationCommands"]
            .as_array()
            .unwrap()
            .len()
    }

    #[test]
    fn rejection_never_applies_and_confirmation_emits_distinct_effect_states() {
        let (coordinator, events, repository, command) = fixture();
        let rejected = coordinator
            .request(InitiationRequestSource::Button, command)
            .unwrap();
        assert_eq!(
            events.recv().unwrap().state,
            InitiationConfirmationState::Requested
        );
        assert_eq!(initiated_count(&repository), 0);
        assert_eq!(
            coordinator.resolve(&rejected.request_id, UserInitiationDecision::Rejected),
            Err(InitiationConfirmationError::Rejected)
        );
        assert_eq!(
            events.recv().unwrap().state,
            InitiationConfirmationState::UserRejected
        );
        assert_eq!(initiated_count(&repository), 0);

        let (coordinator, events, repository, command) = fixture();
        let requested = coordinator
            .request(InitiationRequestSource::Button, command.clone())
            .unwrap();
        assert_eq!(
            events.recv().unwrap().state,
            InitiationConfirmationState::Requested
        );
        let resolution = coordinator
            .resolve(&requested.request_id, UserInitiationDecision::Confirmed)
            .unwrap();
        assert_eq!(resolution.state, InitiationConfirmationState::Projected);
        assert_eq!(
            (0..4)
                .map(|_| events.recv().unwrap().state)
                .collect::<Vec<_>>(),
            [
                InitiationConfirmationState::UserConfirmed,
                InitiationConfirmationState::Applied,
                InitiationConfirmationState::Persisted,
                InitiationConfirmationState::Projected,
            ]
        );
        assert_eq!(initiated_count(&repository), 1);
        let replay = coordinator
            .request(InitiationRequestSource::Button, command)
            .unwrap();
        assert_eq!(replay.request_id, requested.request_id);
        assert_eq!(
            coordinator
                .resolve(&replay.request_id, UserInitiationDecision::Confirmed)
                .unwrap()
                .initiation,
            resolution.initiation
        );
        assert_eq!(initiated_count(&repository), 1);
        assert!(repository
            .claim_pending_plan_builder_context(
                "session-confirmation",
                "button-claim",
                "button-target-invocation",
            )
            .unwrap()
            .is_some());
    }

    #[test]
    fn agent_confirmation_does_not_schedule_button_context() {
        let (coordinator, events, repository, command) = fixture();
        let requested = coordinator
            .request(
                InitiationRequestSource::Agent {
                    agent_session_id: "session-confirmation".into(),
                    agent_invocation_id: "agent-invocation".into(),
                },
                command,
            )
            .unwrap();
        assert_eq!(
            events.recv().unwrap().state,
            InitiationConfirmationState::Requested
        );
        coordinator
            .resolve(&requested.request_id, UserInitiationDecision::Confirmed)
            .unwrap();
        assert!(repository
            .claim_pending_plan_builder_context(
                "session-confirmation",
                "agent-claim",
                "agent-target-invocation",
            )
            .unwrap()
            .is_none());
    }

    #[test]
    fn concurrent_duplicate_requests_register_one_visible_confirmation() {
        let notifier = Arc::new(ScriptedNotifier::default());
        let (coordinator, _, command) = fixture_with_notifier(notifier.clone());
        let callers = 8;
        let barrier = Arc::new(Barrier::new(callers + 1));
        let (sender, receiver) = mpsc::channel();
        let mut threads = Vec::new();
        for _ in 0..callers {
            let coordinator = coordinator.clone();
            let command = command.clone();
            let barrier = barrier.clone();
            let sender = sender.clone();
            threads.push(thread::spawn(move || {
                barrier.wait();
                sender
                    .send(coordinator.request(InitiationRequestSource::Button, command))
                    .unwrap();
            }));
        }
        barrier.wait();
        drop(sender);
        let requests = receiver
            .iter()
            .map(Result::unwrap)
            .collect::<Vec<InitiationConfirmationRequest>>();
        for thread in threads {
            thread.join().unwrap();
        }
        assert_eq!(requests.len(), callers);
        assert!(requests
            .iter()
            .all(|request| request.request_id == requests[0].request_id));
        assert_eq!(notifier.attempts().len(), 1);
        assert_eq!(notifier.visible().len(), 1);
        assert_eq!(
            notifier.visible()[0].state,
            InitiationConfirmationState::Requested
        );
    }

    #[test]
    fn failed_requested_publication_rolls_back_and_retry_becomes_visible() {
        let notifier = ScriptedNotifier::failing_once(InitiationConfirmationState::Requested);
        let (coordinator, _, command) = fixture_with_notifier(notifier.clone());

        assert!(matches!(
            coordinator.request(InitiationRequestSource::Button, command.clone()),
            Err(InitiationConfirmationError::Unavailable(_))
        ));
        let failed_request_id = notifier.attempts()[0].request.request_id.clone();
        assert!(notifier.visible().is_empty());

        let retry = coordinator
            .request(InitiationRequestSource::Button, command.clone())
            .unwrap();
        assert_ne!(retry.request_id, failed_request_id);
        assert_eq!(notifier.visible().len(), 1);
        assert_eq!(notifier.visible()[0].request.request_id, retry.request_id);
        assert_eq!(
            coordinator
                .request(InitiationRequestSource::Button, command)
                .unwrap()
                .request_id,
            retry.request_id
        );
        assert_eq!(notifier.visible().len(), 1);
    }

    #[test]
    fn decision_notification_failures_wake_waiters_with_terminal_truth() {
        for (failed_state, decision) in [
            (
                InitiationConfirmationState::UserRejected,
                UserInitiationDecision::Rejected,
            ),
            (
                InitiationConfirmationState::UserConfirmed,
                UserInitiationDecision::Confirmed,
            ),
        ] {
            let notifier = ScriptedNotifier::failing_once(failed_state);
            let (coordinator, repository, command) = fixture_with_notifier(notifier);
            let request = coordinator
                .request(
                    InitiationRequestSource::Agent {
                        agent_session_id: "session-confirmation".into(),
                        agent_invocation_id: "invocation-confirmation".into(),
                    },
                    command,
                )
                .unwrap();
            let waiter = coordinator.clone();
            let request_id = request.request_id.clone();
            let (sender, receiver) = mpsc::channel();
            thread::spawn(move || {
                sender
                    .send(waiter.wait_for_resolution(&request_id, Duration::from_secs(5)))
                    .unwrap();
            });

            let resolved = coordinator.resolve(&request.request_id, decision);
            let waited = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
            assert_eq!(waited, resolved);
            match (failed_state, resolved) {
                (
                    InitiationConfirmationState::UserRejected,
                    Err(InitiationConfirmationError::RejectedNotificationFailed(_)),
                )
                | (
                    InitiationConfirmationState::UserConfirmed,
                    Err(InitiationConfirmationError::ConfirmedButNotApplied(_)),
                ) => {}
                (_, other) => panic!("unexpected terminal result: {other:?}"),
            }
            assert_eq!(initiated_count(&repository), 0);
        }
    }

    #[test]
    fn persisted_initiation_survives_later_notification_failure_without_reapply() {
        let notifier = ScriptedNotifier::failing_once(InitiationConfirmationState::Projected);
        let (coordinator, repository, command) = fixture_with_notifier(notifier.clone());
        let request = coordinator
            .request(InitiationRequestSource::Button, command.clone())
            .unwrap();
        let waiter = coordinator.clone();
        let request_id = request.request_id.clone();
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            sender
                .send(waiter.wait_for_resolution(&request_id, Duration::from_secs(5)))
                .unwrap();
        });

        let resolved = coordinator.resolve(&request.request_id, UserInitiationDecision::Confirmed);
        let waited = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(waited, resolved);
        let initiation = match &resolved {
            Err(InitiationConfirmationError::PersistedButIncomplete {
                initiation,
                projected: Some(true),
                reason,
            }) => {
                assert!(reason.contains("Projected notification failed"));
                initiation.clone()
            }
            other => panic!("unexpected persisted result: {other:?}"),
        };
        assert!(coordinator
            .application
            .initiation_is_projected(&initiation.initiation_id)
            .unwrap());
        assert_eq!(initiated_count(&repository), 1);
        assert_eq!(initiation_command_count(&repository), 1);

        let replay = coordinator
            .request(InitiationRequestSource::Button, command)
            .unwrap();
        assert_eq!(replay.request_id, request.request_id);
        assert_eq!(
            coordinator.resolve(&replay.request_id, UserInitiationDecision::Confirmed),
            resolved
        );
        assert_eq!(initiated_count(&repository), 1);
        assert_eq!(initiation_command_count(&repository), 1);
        assert_eq!(notifier.attempts().len(), 5);
    }
}
