//! Tauri state, commands, DTO mapping, and persisted update notifications.

mod dto;

use self::dto::{
    AgentInvocationDto, AgentSessionDetailsDto, AgentSessionDto, AgentSessionSummaryDto,
    AgentSessionUpdateDto, CancelAgentInvocationCommandDto, CreateAgentSessionCommandDto,
    ListAgentSessionsQueryDto, LoadAgentSessionQueryDto, SendAgentSessionMessageCommandDto,
    SendAgentSessionMessageResultDto,
};
use crate::agent_sessions::application::{
    AgentSessionApplication, AgentSessionNotification, AgentSessionNotifier,
};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

pub(crate) const AGENT_SESSION_UPDATE_EVENT: &str = "agent-session-update";

pub(crate) struct AgentSessionTauriState {
    application: AgentSessionApplication,
}

impl AgentSessionTauriState {
    pub(crate) fn new(application: AgentSessionApplication) -> Self {
        Self { application }
    }

    pub(crate) fn application(&self) -> &AgentSessionApplication {
        &self.application
    }
}

trait AgentSessionEventPublisher: Send + Sync {
    fn emit(&self, event: &str, payload: AgentSessionUpdateDto) -> Result<(), String>;
}

struct TauriAgentSessionEventPublisher {
    app: AppHandle,
}

impl AgentSessionEventPublisher for TauriAgentSessionEventPublisher {
    fn emit(&self, event: &str, payload: AgentSessionUpdateDto) -> Result<(), String> {
        self.app
            .emit(event, payload)
            .map_err(|error| error.to_string())
    }
}

pub(crate) struct TauriAgentSessionNotifier {
    publisher: Arc<dyn AgentSessionEventPublisher>,
}

impl TauriAgentSessionNotifier {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self {
            publisher: Arc::new(TauriAgentSessionEventPublisher { app }),
        }
    }
}

impl AgentSessionNotifier for TauriAgentSessionNotifier {
    fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
        self.publisher
            .emit(AGENT_SESSION_UPDATE_EVENT, notification.into())
    }
}

#[tauri::command]
pub(crate) fn create_agent_session(
    state: State<'_, AgentSessionTauriState>,
    input: CreateAgentSessionCommandDto,
) -> Result<AgentSessionDto, String> {
    state
        .application
        .create_session(input.into())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn list_agent_sessions(
    state: State<'_, AgentSessionTauriState>,
    query: ListAgentSessionsQueryDto,
) -> Result<Vec<AgentSessionSummaryDto>, String> {
    state
        .application
        .list_sessions(query.into())
        .map(|summaries| summaries.into_iter().map(Into::into).collect())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn load_agent_session(
    state: State<'_, AgentSessionTauriState>,
    query: LoadAgentSessionQueryDto,
) -> Result<AgentSessionDetailsDto, String> {
    state
        .application
        .load_session(&query.session_id)
        .map(|(session, invocations)| AgentSessionDetailsDto::from_history(session, invocations))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn send_agent_session_message(
    state: State<'_, AgentSessionTauriState>,
    input: SendAgentSessionMessageCommandDto,
) -> Result<SendAgentSessionMessageResultDto, String> {
    state
        .application
        .send_message(input.into())
        .map(Into::into)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn cancel_agent_invocation(
    state: State<'_, AgentSessionTauriState>,
    input: CancelAgentInvocationCommandDto,
) -> Result<AgentInvocationDto, String> {
    state
        .application
        .cancel_invocation(input.into())
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
