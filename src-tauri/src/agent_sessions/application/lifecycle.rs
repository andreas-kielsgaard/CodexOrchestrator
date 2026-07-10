use super::update_sink::{InvocationUpdateLanes, PersistedRuntimeUpdateSink};
use crate::agent_sessions::{
    domain::{
        AgentDiagnostic, AgentDiagnosticSeverity, AgentDiagnosticSource, AgentInvocation,
        AgentInvocationId, AgentInvocationStatus, AgentInvocationTerminalStatus,
        AgentRuntimeBinding, AgentRuntimeEvent, AgentRuntimeEventId, AgentRuntimeFailure,
        AgentRuntimeKind, AgentRuntimeOptions, AgentSession, AgentSessionAvailability,
        AgentSessionId, InvocationCompletion,
    },
    ports::{
        AgentRuntime, AgentRuntimeUpdateSink, AgentSessionRepository, ListAgentSessionsQuery,
        RepositoryError, RuntimeInvocationMode, RuntimeInvocationRequest, RuntimePortError,
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
    pub(crate) runtime_kind: Option<AgentRuntimeKind>,
    pub(crate) requested_options: AgentRuntimeOptions,
}

#[derive(Clone, Debug)]
pub(crate) struct SendAgentSessionMessageCommand {
    pub(crate) session_id: Option<AgentSessionId>,
    pub(crate) submitted_text: String,
    pub(crate) title: Option<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) requested_options: Option<AgentRuntimeOptions>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SendAgentSessionMessageResult {
    pub(crate) session_id: AgentSessionId,
    pub(crate) invocation_id: AgentInvocationId,
}

#[derive(Clone, Debug)]
pub(crate) struct CancelAgentInvocationCommand {
    pub(crate) invocation_id: AgentInvocationId,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AgentSessionSummaryResult {
    pub(crate) session: AgentSession,
    pub(crate) has_active_invocation: bool,
}

pub(crate) type ListAgentSessionsResult = Vec<AgentSessionSummaryResult>;

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
        if command
            .runtime_kind
            .is_some_and(|kind| kind != AgentRuntimeKind::CodexCli)
        {
            return Err(AgentSessionApplicationError::invalid(
                "only the codex_cli runtime is supported",
            ));
        }
        let now = self.clock.now();
        let session = AgentSession {
            id: self.ids.session_id(),
            title: normalize_title(command.title.as_deref(), "Agent Session"),
            availability: AgentSessionAvailability::Available,
            runtime_binding: AgentRuntimeBinding {
                kind: AgentRuntimeKind::CodexCli,
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
            .list_sessions(query)
            .map_err(AgentSessionApplicationError::repository)?
            .into_iter()
            .map(|session| {
                let has_active_invocation = self
                    .repository
                    .list_invocations(&session.id)
                    .map_err(AgentSessionApplicationError::repository)?
                    .iter()
                    .any(|invocation| invocation.status.is_active());
                Ok(AgentSessionSummaryResult {
                    session,
                    has_active_invocation,
                })
            })
            .collect()
    }

    pub(crate) fn load_session(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<
        (AgentSession, Vec<(AgentInvocation, Vec<AgentRuntimeEvent>)>),
        AgentSessionApplicationError,
    > {
        let session = self
            .repository
            .get_session(session_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent Session not found"))?;
        let histories = self
            .repository
            .list_invocations(session_id)
            .map_err(AgentSessionApplicationError::repository)?
            .into_iter()
            .map(|invocation| {
                let events = self
                    .repository
                    .list_events(&invocation.id)
                    .map_err(AgentSessionApplicationError::repository)?;
                Ok((invocation, events))
            })
            .collect::<Result<Vec<_>, AgentSessionApplicationError>>()?;
        Ok((session, histories))
    }

    pub(crate) fn send_message(
        &self,
        command: SendAgentSessionMessageCommand,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
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
                runtime_kind: Some(AgentRuntimeKind::CodexCli),
                requested_options: command.requested_options.clone().unwrap_or_default(),
            })?,
        };
        if session.availability != AgentSessionAvailability::Available {
            return Err(AgentSessionApplicationError::conflict(
                "archived Agent Sessions cannot accept messages",
            ));
        }

        let requested_options = command
            .requested_options
            .clone()
            .unwrap_or_else(|| session.requested_options.clone());
        let created_at = self.clock.now();
        let invocation = self
            .repository
            .create_pending_invocation(AgentInvocation {
                id: self.ids.invocation_id(),
                session_id: session.id.clone(),
                submitted_text: command.submitted_text.clone(),
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
                return Ok(acknowledgement);
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
            submitted_text: invocation.submitted_text.clone(),
            working_directory: session.working_directory.clone(),
            options: preflight.effective_options,
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
        if let Err(error) = launch {
            // Supervisor-backed spawn/start failures already own their one terminal callback.
            // Recording the returned error is diagnostic only and never synthesizes completion.
            self.record_diagnostic(
                &invocation.id,
                AgentDiagnosticSource::Runtime,
                "runtime_launch_returned_error",
                error.message.clone(),
                serde_json::to_value(&error).ok(),
            );
        }
        Ok(acknowledgement)
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
        for session in self
            .repository
            .list_sessions(ListAgentSessionsQuery::default())
            .map_err(AgentSessionApplicationError::repository)?
        {
            for invocation in self
                .repository
                .list_invocations(&session.id)
                .map_err(AgentSessionApplicationError::repository)?
            {
                if !invocation.status.is_active() {
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
                reconciled += 1;
                self.notify_or_record(AgentSessionNotification::InvocationTerminal {
                    session_id: session.id.clone(),
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
