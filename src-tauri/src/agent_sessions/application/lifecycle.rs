use super::update_sink::{InvocationUpdateLanes, PersistedRuntimeUpdateSink};
use crate::agent_sessions::{
    domain::{
        AgentDiagnostic, AgentDiagnosticSeverity, AgentDiagnosticSource, AgentInvocation,
        AgentInvocationId, AgentInvocationInputProvenance, AgentInvocationStatus,
        AgentInvocationTerminalStatus, AgentRuntimeBinding, AgentRuntimeEvent, AgentRuntimeEventId,
        AgentRuntimeFailure, AgentRuntimeOptions, AgentSession, AgentSessionAvailability,
        AgentSessionId, InvocationCompletion,
    },
    ports::{
        AgentRuntime, AgentRuntimeUpdateSink, AgentSessionHistory, AgentSessionRepository,
        AgentSessionSummary, ListAgentSessionsQuery, RepositoryError, RuntimeInvocationMode,
        RuntimeInvocationOutcome, RuntimeInvocationRequest, RuntimeLaunchExtension,
        RuntimePortError, RuntimeUpdate,
    },
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use std::{error::Error, fmt, sync::Arc};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub(crate) struct CreateAgentSessionCommand {
    pub(crate) title: Option<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) requested_options: AgentRuntimeOptions,
}

#[derive(Clone, Debug)]
pub(crate) struct CreateApplicationAgentSessionCommand {
    pub(crate) session_id: AgentSessionId,
    pub(crate) session: CreateAgentSessionCommand,
}

#[derive(Clone, Debug)]
pub(crate) struct SendAgentSessionMessageCommand {
    pub(crate) session_id: Option<AgentSessionId>,
    pub(crate) submitted_text: String,
    pub(crate) title: Option<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) requested_options: Option<AgentRuntimeOptions>,
}

#[derive(Clone, Debug)]
pub(crate) struct SendIdempotentApplicationAgentSessionMessageCommand {
    pub(crate) invocation_id: AgentInvocationId,
    pub(crate) message: SendAgentSessionMessageCommand,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SendAgentSessionMessageResult {
    pub(crate) session_id: AgentSessionId,
    pub(crate) invocation_id: AgentInvocationId,
}

pub(crate) struct SendAgentSessionMessageLaunchResult {
    pub(crate) acknowledgement: SendAgentSessionMessageResult,
    pub(crate) launch_accepted: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApplicationInvocationLaunchEvidence {
    NeverPersisted,
    PersistedNotAccepted,
    LaunchAccepted,
}

#[derive(Clone, Debug)]
pub(crate) struct CancelAgentInvocationCommand {
    pub(crate) invocation_id: AgentInvocationId,
}

pub(crate) type ListAgentSessionsResult = Vec<AgentSessionSummary>;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum AgentSessionNotification {
    EventPersisted {
        session_id: AgentSessionId,
        event: AgentRuntimeEvent,
    },
    InvocationTerminal {
        session_id: AgentSessionId,
        invocation: AgentInvocation,
    },
    DiagnosticRecorded {
        session_id: AgentSessionId,
        invocation: AgentInvocation,
    },
}

pub(crate) trait AgentSessionNotifier: Send + Sync {
    fn notify(&self, notification: AgentSessionNotification) -> Result<(), String>;
}

pub(crate) trait AgentSessionClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub(crate) trait AgentSessionIdProvider: Send + Sync {
    fn session_id(&self) -> AgentSessionId;
    fn invocation_id(&self) -> AgentInvocationId;
    fn event_id(&self) -> AgentRuntimeEventId;
}

pub(crate) struct SystemAgentSessionProviders;

impl AgentSessionClock for SystemAgentSessionProviders {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

impl AgentSessionIdProvider for SystemAgentSessionProviders {
    fn session_id(&self) -> AgentSessionId {
        AgentSessionId::new(Uuid::new_v4().to_string()).expect("UUID is a valid session ID")
    }

    fn invocation_id(&self) -> AgentInvocationId {
        AgentInvocationId::new(Uuid::new_v4().to_string()).expect("UUID is a valid invocation ID")
    }

    fn event_id(&self) -> AgentRuntimeEventId {
        AgentRuntimeEventId::new(Uuid::new_v4().to_string()).expect("UUID is a valid event ID")
    }
}

#[derive(Clone)]
pub(crate) struct AgentSessionApplication {
    repository: Arc<dyn AgentSessionRepository>,
    runtime: Arc<dyn AgentRuntime>,
    notifier: Arc<dyn AgentSessionNotifier>,
    clock: Arc<dyn AgentSessionClock>,
    ids: Arc<dyn AgentSessionIdProvider>,
    runtime_version: Option<String>,
    update_lanes: Arc<InvocationUpdateLanes>,
}

impl AgentSessionApplication {
    pub(crate) fn new(
        repository: Arc<dyn AgentSessionRepository>,
        runtime: Arc<dyn AgentRuntime>,
        notifier: Arc<dyn AgentSessionNotifier>,
        clock: Arc<dyn AgentSessionClock>,
        ids: Arc<dyn AgentSessionIdProvider>,
        runtime_version: Option<String>,
    ) -> Self {
        Self {
            repository,
            runtime,
            notifier,
            clock,
            ids,
            runtime_version,
            update_lanes: Arc::new(InvocationUpdateLanes::default()),
        }
    }

    pub(crate) fn create_session(
        &self,
        command: CreateAgentSessionCommand,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        self.create_session_with_id(command, self.ids.session_id())
    }

    pub(crate) fn create_application_session(
        &self,
        command: CreateApplicationAgentSessionCommand,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        if let Some(existing) = self
            .repository
            .get_session(&command.session_id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            let expected_title = normalize_title(command.session.title.as_deref(), "Agent Session");
            let expected_directory = normalize_optional(command.session.working_directory);
            if existing.title != expected_title
                || existing.working_directory != expected_directory
                || existing.requested_options != command.session.requested_options
            {
                return Err(AgentSessionApplicationError::conflict(
                    "application Agent Session identity was already used for different semantics",
                ));
            }
            return Ok(existing);
        }
        self.create_session_with_id(command.session, command.session_id)
    }

    fn create_session_with_id(
        &self,
        command: CreateAgentSessionCommand,
        session_id: AgentSessionId,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        let now = self.clock.now();
        let session = AgentSession {
            id: session_id,
            title: normalize_title(command.title.as_deref(), "Agent Session"),
            availability: AgentSessionAvailability::Available,
            runtime_binding: AgentRuntimeBinding {
                external_context_id: None,
                runtime_version: self.runtime_version.clone(),
            },
            working_directory: normalize_optional(command.working_directory),
            requested_options: command.requested_options,
            created_at: now,
            updated_at: now,
        };
        self.repository
            .create_session(session)
            .map_err(AgentSessionApplicationError::repository)
    }

    pub(crate) fn list_sessions(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<ListAgentSessionsResult, AgentSessionApplicationError> {
        self.repository
            .list_session_summaries(query)
            .map_err(AgentSessionApplicationError::repository)
    }

    pub(crate) fn load_session(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<AgentSessionHistory, AgentSessionApplicationError> {
        self.repository
            .load_session_history(session_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent Session not found"))
    }

    pub(crate) fn send_message(
        &self,
        command: SendAgentSessionMessageCommand,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        self.send_message_with_launch_extension(command, None)
    }

    /// Explicit opt-in for a role-specific application service. Generic callers cannot acquire
    /// an extension accidentally because the normal send path always supplies `None`.
    pub(crate) fn send_message_with_launch_extension(
        &self,
        command: SendAgentSessionMessageCommand,
        launch_extension: Option<RuntimeLaunchExtension>,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        self.send_message_with_provenance(
            command,
            launch_extension,
            AgentInvocationInputProvenance::User,
            None,
        )
        .map(|result| result.acknowledgement)
    }

    pub(crate) fn send_idempotent_application_message_with_launch_observation(
        &self,
        command: SendIdempotentApplicationAgentSessionMessageCommand,
        launch_extension: Option<RuntimeLaunchExtension>,
    ) -> Result<SendAgentSessionMessageLaunchResult, AgentSessionApplicationError> {
        self.send_message_with_provenance(
            command.message,
            launch_extension,
            AgentInvocationInputProvenance::Application,
            Some(command.invocation_id),
        )
    }

    pub(crate) fn send_idempotent_user_message_with_launch_observation(
        &self,
        command: SendIdempotentApplicationAgentSessionMessageCommand,
        launch_extension: Option<RuntimeLaunchExtension>,
    ) -> Result<SendAgentSessionMessageLaunchResult, AgentSessionApplicationError> {
        self.send_message_with_provenance(
            command.message,
            launch_extension,
            AgentInvocationInputProvenance::User,
            Some(command.invocation_id),
        )
    }

    pub(crate) fn allocate_application_invocation_id(&self) -> AgentInvocationId {
        self.ids.invocation_id()
    }

    /// Persist an application-owned invocation without runtime preflight or a launch attempt.
    pub(crate) fn prepare_idempotent_application_invocation(
        &self,
        command: SendIdempotentApplicationAgentSessionMessageCommand,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        if command.message.submitted_text.trim().is_empty() {
            return Err(AgentSessionApplicationError::invalid("submitted text cannot be empty"));
        }
        let session_id = command.message.session_id.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("prepared application invocation requires a Session")
        })?;
        let session = self.repository.get_session(session_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent Session not found"))?;
        if session.availability != AgentSessionAvailability::Available {
            return Err(AgentSessionApplicationError::conflict(
                "archived Agent Sessions cannot accept messages",
            ));
        }
        let requested_options = command.message.requested_options.clone()
            .unwrap_or_else(|| session.requested_options.clone());
        if let Some(existing) = self.repository.get_invocation(&command.invocation_id)
            .map_err(AgentSessionApplicationError::repository)? {
            if existing.session_id != session.id
                || existing.submitted_text != command.message.submitted_text
                || existing.input_provenance != AgentInvocationInputProvenance::Application
                || existing.requested_options != requested_options {
                return Err(AgentSessionApplicationError::conflict(
                    "application Agent Invocation identity was already used for different semantics",
                ));
            }
            return Ok(SendAgentSessionMessageResult { session_id: session.id, invocation_id: existing.id });
        }
        let created_at = self.clock.now();
        let invocation = self.repository.create_pending_invocation(AgentInvocation {
            id: command.invocation_id,
            session_id: session.id.clone(),
            submitted_text: command.message.submitted_text,
            input_provenance: AgentInvocationInputProvenance::Application,
            status: AgentInvocationStatus::Pending,
            requested_options,
            effective_options: None,
            started_at: None,
            completed_at: None,
            exit_code: None,
            signal: None,
            runtime_error: None,
            diagnostics: Vec::new(),
            created_at,
            updated_at: created_at,
        }).map_err(AgentSessionApplicationError::repository)?;
        Ok(SendAgentSessionMessageResult { session_id: session.id, invocation_id: invocation.id })
    }

    pub(crate) fn application_invocation_launch_evidence(
        &self,
        invocation_id: &AgentInvocationId,
        expected_session_id: &AgentSessionId,
    ) -> Result<ApplicationInvocationLaunchEvidence, AgentSessionApplicationError> {
        self.invocation_launch_evidence(
            invocation_id,
            expected_session_id,
            AgentInvocationInputProvenance::Application,
        )
    }

    pub(crate) fn user_invocation_launch_evidence(
        &self,
        invocation_id: &AgentInvocationId,
        expected_session_id: &AgentSessionId,
    ) -> Result<ApplicationInvocationLaunchEvidence, AgentSessionApplicationError> {
        self.invocation_launch_evidence(
            invocation_id,
            expected_session_id,
            AgentInvocationInputProvenance::User,
        )
    }

    fn invocation_launch_evidence(
        &self,
        invocation_id: &AgentInvocationId,
        expected_session_id: &AgentSessionId,
        expected_provenance: AgentInvocationInputProvenance,
    ) -> Result<ApplicationInvocationLaunchEvidence, AgentSessionApplicationError> {
        let Some(invocation) = self
            .repository
            .get_invocation(invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
        else {
            return Ok(ApplicationInvocationLaunchEvidence::NeverPersisted);
        };
        if invocation.session_id != *expected_session_id
            || invocation.input_provenance != expected_provenance
        {
            return Err(AgentSessionApplicationError::conflict(
                "allocated Agent Invocation launch evidence does not match the expected session and provenance",
            ));
        }
        Ok(
            if self
                .repository
                .invocation_launch_accepted_at(invocation_id)
                .map_err(AgentSessionApplicationError::repository)?
                .is_some()
            {
                ApplicationInvocationLaunchEvidence::LaunchAccepted
            } else {
                ApplicationInvocationLaunchEvidence::PersistedNotAccepted
            },
        )
    }

    fn send_message_with_provenance(
        &self,
        command: SendAgentSessionMessageCommand,
        launch_extension: Option<RuntimeLaunchExtension>,
        input_provenance: AgentInvocationInputProvenance,
        requested_invocation_id: Option<AgentInvocationId>,
    ) -> Result<SendAgentSessionMessageLaunchResult, AgentSessionApplicationError> {
        if command.submitted_text.trim().is_empty() {
            return Err(AgentSessionApplicationError::invalid(
                "submitted text cannot be empty",
            ));
        }

        let session = match command.session_id.as_ref() {
            Some(session_id) => self
                .repository
                .get_session(session_id)
                .map_err(AgentSessionApplicationError::repository)?
                .ok_or_else(|| {
                    AgentSessionApplicationError::not_found("Agent Session not found")
                })?,
            None => self.create_session(CreateAgentSessionCommand {
                title: command
                    .title
                    .clone()
                    .or_else(|| Some(title_from_message(&command.submitted_text))),
                working_directory: command.working_directory.clone(),
                requested_options: command.requested_options.clone().unwrap_or_default(),
            })?,
        };
        if session.availability != AgentSessionAvailability::Available {
            return Err(AgentSessionApplicationError::conflict(
                "archived Agent Sessions cannot accept messages",
            ));
        }
        let session = self.repair_missing_runtime_binding(session)?;

        let requested_options = command
            .requested_options
            .clone()
            .unwrap_or_else(|| session.requested_options.clone());
        if let Some(invocation_id) = requested_invocation_id.as_ref() {
            if let Some(existing) = self
                .repository
                .get_invocation(invocation_id)
                .map_err(AgentSessionApplicationError::repository)?
            {
                if existing.session_id != session.id
                    || existing.submitted_text != command.submitted_text
                    || existing.input_provenance != input_provenance
                    || existing.requested_options != requested_options
                {
                    return Err(AgentSessionApplicationError::conflict(
                        "application Agent Invocation identity was already used for different semantics",
                    ));
                }
                let launch_accepted = self
                    .repository
                    .invocation_launch_accepted_at(invocation_id)
                    .map_err(AgentSessionApplicationError::repository)?
                    .is_some();
                return Ok(SendAgentSessionMessageLaunchResult {
                    acknowledgement: SendAgentSessionMessageResult {
                        session_id: session.id,
                        invocation_id: existing.id,
                    },
                    launch_accepted,
                });
            }
        }
        let created_at = self.clock.now();
        let invocation = self
            .repository
            .create_pending_invocation(AgentInvocation {
                id: requested_invocation_id.unwrap_or_else(|| self.ids.invocation_id()),
                session_id: session.id.clone(),
                submitted_text: command.submitted_text.clone(),
                input_provenance,
                status: AgentInvocationStatus::Pending,
                requested_options: requested_options.clone(),
                effective_options: None,
                started_at: None,
                completed_at: None,
                exit_code: None,
                signal: None,
                runtime_error: None,
                diagnostics: Vec::new(),
                created_at,
                updated_at: created_at,
            })
            .map_err(AgentSessionApplicationError::repository)?;
        let acknowledgement = SendAgentSessionMessageResult {
            session_id: session.id.clone(),
            invocation_id: invocation.id.clone(),
        };

        let mode = if session.runtime_binding.external_context_id.is_some() {
            RuntimeInvocationMode::Resume
        } else {
            RuntimeInvocationMode::Start
        };
        let preflight = match self.runtime.preflight_invocation(mode, &requested_options) {
            Ok(preflight) => preflight,
            Err(error) => {
                self.finish_preflight_failure(&invocation, error)?;
                return Ok(SendAgentSessionMessageLaunchResult {
                    acknowledgement,
                    launch_accepted: false,
                });
            }
        };

        let started_at = self.clock.now();
        self.repository
            .mark_invocation_running(
                &invocation.id,
                started_at,
                preflight.effective_options.clone(),
                started_at,
            )
            .map_err(AgentSessionApplicationError::repository)?;

        let request = RuntimeInvocationRequest {
            session_id: session.id.clone(),
            invocation_id: invocation.id.clone(),
            submitted_text: launch_extension
                .as_ref()
                .and_then(|extension| extension.initial_prompt_prefix.as_ref())
                .map(|context| context.render_before_user_query(&invocation.submitted_text))
                .unwrap_or_else(|| invocation.submitted_text.clone()),
            // Product-specific callers may select a neutral per-invocation discovery root without
            // mutating the provider-neutral Agent Session identity. Ordinary sends continue to
            // inherit the durable session directory because they do not supply an override.
            working_directory: command
                .working_directory
                .clone()
                .or_else(|| session.working_directory.clone()),
            options: preflight.effective_options,
            launch_extension,
        };
        let sink: Arc<dyn AgentRuntimeUpdateSink> = Arc::new(PersistedRuntimeUpdateSink::new(
            self.repository.clone(),
            self.notifier.clone(),
            self.clock.clone(),
            self.ids.clone(),
            self.update_lanes.clone(),
        ));
        let launch = match session.runtime_binding.external_context_id {
            Some(external_context_id) => {
                self.runtime
                    .resume_invocation(request, external_context_id, sink)
            }
            None => self.runtime.start_invocation(request, sink),
        };
        let launch_accepted = match launch {
            Ok(()) => {
                self.repository
                    .record_invocation_launch_accepted(&invocation.id, self.clock.now())
                    .map_err(AgentSessionApplicationError::repository)?;
                true
            }
            Err(error) => {
                self.handle_launch_error(&invocation.id, error)?;
                false
            }
        };
        Ok(SendAgentSessionMessageLaunchResult {
            acknowledgement,
            launch_accepted,
        })
    }

    pub(crate) fn cancel_invocation(
        &self,
        command: CancelAgentInvocationCommand,
    ) -> Result<AgentInvocation, AgentSessionApplicationError> {
        let invocation = self
            .repository
            .get_invocation(&command.invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent invocation not found"))?;
        if !invocation.status.is_active() {
            return Ok(invocation);
        }
        if let Err(error) = self.runtime.cancel_invocation(&command.invocation_id) {
            self.record_diagnostic(
                &command.invocation_id,
                AgentDiagnosticSource::Runtime,
                "runtime_cancellation_failed",
                error.message.clone(),
                serde_json::to_value(&error).ok(),
            );
            return Err(AgentSessionApplicationError::runtime(error));
        }
        self.repository
            .get_invocation(&command.invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent invocation not found"))
    }

    pub(crate) fn reconcile_startup(&self) -> Result<usize, AgentSessionApplicationError> {
        let mut reconciled = 0;
        for summary in self
            .repository
            .list_session_summaries(ListAgentSessionsQuery::default())
            .map_err(AgentSessionApplicationError::repository)?
        {
            let history = self
                .repository
                .load_session_history(&summary.session.id)
                .map_err(AgentSessionApplicationError::repository)?
                .ok_or_else(|| {
                    AgentSessionApplicationError::not_found(
                        "Agent Session disappeared during startup reconciliation",
                    )
                })?;
            for invocation_history in history.invocations {
                let invocation = invocation_history.invocation;
                if !invocation.status.is_active() {
                    continue;
                }
                if let Some((outcome, completed_at)) = known_terminal_delivery(&invocation) {
                    let updated = self
                        .repository
                        .finish_invocation(
                            &invocation.id,
                            InvocationCompletion {
                                status: outcome.status,
                                completed_at,
                                exit_code: outcome.exit_code,
                                signal: outcome.signal,
                                runtime_error: outcome.runtime_error,
                            },
                            completed_at,
                        )
                        .map_err(AgentSessionApplicationError::repository)?;
                    self.update_lanes.remove_invocation(&invocation.id);
                    reconciled += 1;
                    self.notify_or_record(AgentSessionNotification::InvocationTerminal {
                        session_id: history.session.id.clone(),
                        invocation: updated,
                    });
                    continue;
                }
                let completed_at = self.clock.now();
                let updated = self
                    .repository
                    .finish_invocation(
                        &invocation.id,
                        InvocationCompletion {
                            status: AgentInvocationTerminalStatus::Interrupted,
                            completed_at,
                            exit_code: None,
                            signal: None,
                            runtime_error: None,
                        },
                        completed_at,
                    )
                    .map_err(AgentSessionApplicationError::repository)?;
                self.update_lanes.remove_invocation(&invocation.id);
                reconciled += 1;
                self.notify_or_record(AgentSessionNotification::InvocationTerminal {
                    session_id: history.session.id.clone(),
                    invocation: updated,
                });
            }
        }
        Ok(reconciled)
    }

    pub(crate) fn shutdown_runtime(&self) -> Result<(), AgentSessionApplicationError> {
        self.runtime
            .shutdown()
            .map_err(AgentSessionApplicationError::runtime)
    }

    #[cfg(test)]
    pub(super) fn update_lane_count(&self) -> usize {
        self.update_lanes.len()
    }

    fn finish_preflight_failure(
        &self,
        invocation: &AgentInvocation,
        error: RuntimePortError,
    ) -> Result<(), AgentSessionApplicationError> {
        let completed_at = self.clock.now();
        let updated = self
            .repository
            .finish_invocation(
                &invocation.id,
                InvocationCompletion {
                    status: AgentInvocationTerminalStatus::Failed,
                    completed_at,
                    exit_code: None,
                    signal: None,
                    runtime_error: Some(AgentRuntimeFailure {
                        code: "runtime_preflight_failed".to_string(),
                        message: error.message.clone(),
                        details: serde_json::to_value(&error).ok(),
                    }),
                },
                completed_at,
            )
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_or_record(AgentSessionNotification::InvocationTerminal {
            session_id: invocation.session_id.clone(),
            invocation: updated,
        });
        Ok(())
    }

    fn handle_launch_error(
        &self,
        invocation_id: &AgentInvocationId,
        error: RuntimePortError,
    ) -> Result<(), AgentSessionApplicationError> {
        let invocation = self
            .repository
            .get_invocation(invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent invocation not found"))?;
        if invocation.status.is_active() {
            let completed_at = self.clock.now();
            let updated = self
                .repository
                .finish_invocation(
                    invocation_id,
                    InvocationCompletion {
                        status: AgentInvocationTerminalStatus::Failed,
                        completed_at,
                        exit_code: None,
                        signal: None,
                        runtime_error: Some(AgentRuntimeFailure {
                            code: "runtime_launch_failed".to_string(),
                            message: error.message.clone(),
                            details: serde_json::to_value(&error).ok(),
                        }),
                    },
                    completed_at,
                )
                .map_err(AgentSessionApplicationError::repository)?;
            self.update_lanes.remove_invocation(invocation_id);
            self.notify_or_record(AgentSessionNotification::InvocationTerminal {
                session_id: updated.session_id.clone(),
                invocation: updated,
            });
        }
        self.record_diagnostic(
            invocation_id,
            AgentDiagnosticSource::Runtime,
            "runtime_launch_returned_error",
            error.message.clone(),
            serde_json::to_value(&error).ok(),
        );
        Ok(())
    }

    fn repair_missing_runtime_binding(
        &self,
        session: AgentSession,
    ) -> Result<AgentSession, AgentSessionApplicationError> {
        if session.runtime_binding.external_context_id.is_some() {
            return Ok(session);
        }
        let history = self
            .repository
            .load_session_history(&session.id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent Session not found"))?;
        let mut recovered = None;
        for event in history
            .invocations
            .iter()
            .flat_map(|history| history.events.iter())
        {
            let Some(external_context_id) = event
                .normalized
                .as_ref()
                .filter(|normalized| {
                    normalized.kind
                        == crate::agent_sessions::domain::NormalizedRuntimeEventKind::RuntimeContextEstablished
                })
                .and_then(|normalized| normalized.external_context_id.clone())
            else {
                continue;
            };
            if recovered
                .as_ref()
                .is_some_and(|current| current != &external_context_id)
            {
                return Err(AgentSessionApplicationError::conflict(
                    "durable runtime context evidence contains conflicting external identities",
                ));
            }
            recovered = Some(external_context_id);
        }
        let Some(external_context_id) = recovered else {
            return Ok(session);
        };
        let mut binding = session.runtime_binding.clone();
        binding.external_context_id = Some(external_context_id);
        self.repository
            .update_runtime_binding(&session.id, binding, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)
    }

    fn notify_or_record(&self, notification: AgentSessionNotification) {
        let (invocation_id, session_id) = notification_ids(&notification);
        if let Err(error) = self.notifier.notify(notification) {
            self.record_diagnostic(
                &invocation_id,
                AgentDiagnosticSource::Transport,
                "agent_session_notification_failed",
                error,
                Some(json!({"sessionId": session_id})),
            );
        }
    }

    fn record_diagnostic(
        &self,
        invocation_id: &AgentInvocationId,
        source: AgentDiagnosticSource,
        code: &str,
        message: String,
        details: Option<Value>,
    ) {
        let diagnostic = AgentDiagnostic {
            source,
            severity: AgentDiagnosticSeverity::Error,
            code: code.to_string(),
            message,
            details,
            recorded_at: self.clock.now(),
        };
        let Ok(invocation) = self
            .repository
            .append_invocation_diagnostic(invocation_id, diagnostic)
        else {
            return;
        };
        let _ = self
            .notifier
            .notify(AgentSessionNotification::DiagnosticRecorded {
                session_id: invocation.session_id.clone(),
                invocation,
            });
    }
}

fn known_terminal_delivery(
    invocation: &AgentInvocation,
) -> Option<(RuntimeInvocationOutcome, DateTime<Utc>)> {
    invocation.diagnostics.iter().rev().find_map(|diagnostic| {
        if diagnostic.source != AgentDiagnosticSource::Repository
            || diagnostic.code != "runtime_update_delivery_failed"
            || diagnostic.recorded_at < invocation.created_at
            || invocation
                .started_at
                .is_some_and(|started_at| diagnostic.recorded_at < started_at)
        {
            return None;
        }
        let failed_update = diagnostic.details.as_ref()?.get("failedUpdate")?.clone();
        match serde_json::from_value::<RuntimeUpdate>(failed_update).ok()? {
            RuntimeUpdate::Finished(outcome) => {
                invocation
                    .finish(
                        InvocationCompletion {
                            status: outcome.status,
                            completed_at: diagnostic.recorded_at,
                            exit_code: outcome.exit_code,
                            signal: outcome.signal.clone(),
                            runtime_error: outcome.runtime_error.clone(),
                        },
                        diagnostic.recorded_at,
                    )
                    .ok()?;
                Some((outcome, diagnostic.recorded_at))
            }
            RuntimeUpdate::Event(_) => None,
        }
    })
}

fn notification_ids(
    notification: &AgentSessionNotification,
) -> (AgentInvocationId, AgentSessionId) {
    match notification {
        AgentSessionNotification::EventPersisted { session_id, event } => {
            (event.invocation_id.clone(), session_id.clone())
        }
        AgentSessionNotification::InvocationTerminal {
            session_id,
            invocation,
        }
        | AgentSessionNotification::DiagnosticRecorded {
            session_id,
            invocation,
        } => (invocation.id.clone(), session_id.clone()),
    }
}

fn normalize_title(title: Option<&str>, fallback: &str) -> String {
    title
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn title_from_message(message: &str) -> String {
    let title = message.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.chars().count() <= 80 {
        title
    } else {
        format!("{}...", title.chars().take(77).collect::<String>())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AgentSessionApplicationErrorKind {
    InvalidInput,
    NotFound,
    Conflict,
    Repository,
    Runtime,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AgentSessionApplicationError {
    pub(crate) kind: AgentSessionApplicationErrorKind,
    pub(crate) message: String,
}

impl AgentSessionApplicationError {
    fn invalid(message: impl Into<String>) -> Self {
        Self::new(AgentSessionApplicationErrorKind::InvalidInput, message)
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::new(AgentSessionApplicationErrorKind::NotFound, message)
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self::new(AgentSessionApplicationErrorKind::Conflict, message)
    }

    fn repository(error: RepositoryError) -> Self {
        Self::new(AgentSessionApplicationErrorKind::Repository, error.message)
    }

    fn runtime(error: RuntimePortError) -> Self {
        Self::new(AgentSessionApplicationErrorKind::Runtime, error.message)
    }

    fn new(kind: AgentSessionApplicationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for AgentSessionApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AgentSessionApplicationError {}
